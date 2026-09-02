use std::collections::{BTreeMap, BTreeSet};

use fmrs_core::{
    piece::{Color, Kind},
    position::{
        advance::{advance::advance_aux, is_legal_mate, legal_movements},
        AdvanceOptions, Movement,
    },
    sfen,
};

use crate::{ConcreteWorld, HandVariableMode, MateRule, ObservedMove, VariableId};

/// これまでの観測と矛盾しない具体世界の集合。
#[derive(Clone, Debug)]
pub struct HiddenState {
    worlds: Vec<ConcreteWorld>,
    /// 一意に確定して普通の駒へ戻った覆面駒。
    resolved: BTreeMap<VariableId, Kind>,
    /// 受先初手では、王手されていなくても受方が着手できる。
    free_white_move: bool,
    rule: MateRule,
    hand_variable_mode: HandVariableMode,
}

impl HiddenState {
    #[cfg(test)]
    pub(crate) fn new(worlds: Vec<ConcreteWorld>, rule: MateRule) -> Self {
        Self::new_with_hand_variable_mode(worlds, rule, HandVariableMode::default())
    }

    pub(crate) fn new_with_hand_variable_mode(
        worlds: Vec<ConcreteWorld>,
        rule: MateRule,
        hand_variable_mode: HandVariableMode,
    ) -> Self {
        let free_white_move = worlds[0].position().turn().is_white();
        Self::with_resolved(
            worlds,
            BTreeMap::new(),
            free_white_move,
            rule,
            hand_variable_mode,
        )
    }

    fn with_resolved(
        worlds: Vec<ConcreteWorld>,
        resolved: BTreeMap<VariableId, Kind>,
        free_white_move: bool,
        rule: MateRule,
        hand_variable_mode: HandVariableMode,
    ) -> Self {
        debug_assert!(!worlds.is_empty());
        let mut state = Self {
            worlds,
            resolved,
            free_white_move,
            rule,
            hand_variable_mode,
        };
        state.reveal_resolved_variables();
        state
    }

    /// 候補が一種類になった覆面駒を、次の着手から通常駒として扱う。
    fn reveal_resolved_variables(&mut self) {
        let ids: BTreeSet<_> = self
            .worlds
            .iter()
            .flat_map(|world| world.variables().map(|piece| piece.id))
            .collect();
        for id in ids {
            let candidates: BTreeSet<_> = self
                .worlds
                .iter()
                .filter_map(|world| world.variable(id).map(|piece| piece.kind))
                .collect();
            if candidates.len() != 1 {
                continue;
            }
            let kind = *candidates.iter().next().expect("候補が一種類ある");
            self.resolved.insert(id, kind);
            for world in &mut self.worlds {
                world.forget_variable(id);
            }
        }
    }

    pub fn worlds(&self) -> &[ConcreteWorld] {
        &self.worlds
    }

    pub fn world_count(&self) -> usize {
        self.worlds.len()
    }

    pub fn turn(&self) -> Color {
        self.worlds[0].position().turn()
    }

    pub fn rule(&self) -> MateRule {
        self.rule
    }

    pub fn hand_variable_mode(&self) -> HandVariableMode {
        self.hand_variable_mode
    }

    pub fn candidates(&self, id: VariableId) -> BTreeSet<Kind> {
        if let Some(&kind) = self.resolved.get(&id) {
            return [kind].into_iter().collect();
        }
        self.worlds
            .iter()
            .filter_map(|world| world.variable(id).map(|piece| piece.kind))
            .collect()
    }

    pub fn resolved_kind(&self, id: VariableId) -> Option<Kind> {
        self.resolved.get(&id).copied()
    }

    /// 少なくとも一つの候補世界で可能な観測着手を返す。
    pub fn observed_moves(&self) -> Vec<ObservedMove> {
        let mut result = BTreeSet::new();
        for world in &self.worlds {
            for (observed, _, _) in
                concrete_moves(world, self.free_white_move, self.hand_variable_mode)
            {
                result.insert(observed);
            }
        }
        result.into_iter().collect()
    }

    /// 観測着手が可能な世界だけを残し、その具体着手を適用する。
    pub fn apply(&self, observed: ObservedMove) -> Option<Self> {
        let mut next_worlds = Vec::new();
        for world in &self.worlds {
            for (candidate, movement, selected_variable) in
                concrete_moves(world, self.free_white_move, self.hand_variable_mode)
            {
                if candidate == observed {
                    next_worlds.push(world.apply(observed, movement, selected_variable));
                }
            }
        }
        if next_worlds.is_empty() {
            None
        } else {
            Some(Self::with_resolved(
                next_worlds,
                self.resolved.clone(),
                false,
                self.rule,
                self.hand_variable_mode,
            ))
        }
    }

    /// 残るすべての候補世界で、選択したルールの対象玉が詰んだときだけ詰みとする。
    pub fn is_proven_mate(&self) -> bool {
        if self.turn() != self.rule.terminal_turn() {
            return false;
        }
        self.worlds.iter().all(|world| {
            let mut position = world.position().clone();
            match self.rule {
                MateRule::Helpmate | MateRule::BestMate => {
                    let mut movements = Vec::new();
                    matches!(
                        advance_aux(&mut position, &AdvanceOptions::default(), &mut movements),
                        Ok(true)
                    ) && movements.is_empty()
                }
                MateRule::HelpSelfmate => {
                    fmrs_core::solve::help_selfmate::is_help_selfmate(&mut position)
                }
            }
        })
    }

    /// 最善詰探索の置換表で使う、候補世界と観測状態を含む完全なキー。
    pub(crate) fn search_key(&self) -> String {
        let mut worlds = self
            .worlds
            .iter()
            .map(|world| {
                let variables = world
                    .variables()
                    .map(|piece| {
                        let location = match piece.location {
                            crate::VariableLocation::Board(square) => {
                                format!("b{}{}", square.col(), square.row())
                            }
                            crate::VariableLocation::Hand(color) => {
                                format!("h{}", usize::from(color.is_white()))
                            }
                        };
                        format!(
                            "{}:{}:{}:{}",
                            piece.id.0,
                            usize::from(piece.color.is_white()),
                            piece.kind.index(),
                            location
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}[{variables}]", sfen::encode_position(world.position()))
            })
            .collect::<Vec<_>>();
        worlds.sort_unstable();
        let resolved = self
            .resolved
            .iter()
            .map(|(id, kind)| format!("{}:{}", id.0, kind.index()))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}|{}|{}|{}",
            usize::from(self.free_white_move),
            self.hand_variable_mode as u8,
            resolved,
            worlds.join(";")
        )
    }

    /// デバッグ/UI用に全覆面駒の候補をまとめて返す。
    pub fn all_candidates(&self) -> BTreeMap<VariableId, BTreeSet<Kind>> {
        let ids: BTreeSet<_> = self
            .worlds
            .iter()
            .flat_map(|world| world.variables().map(|piece| piece.id))
            .chain(self.resolved.keys().copied())
            .collect();
        ids.into_iter()
            .map(|id| (id, self.candidates(id)))
            .collect()
    }
}

fn concrete_moves(
    world: &ConcreteWorld,
    free_white_move: bool,
    hand_variable_mode: HandVariableMode,
) -> Vec<(ObservedMove, Movement, Option<VariableId>)> {
    let mut position = world.position().clone();
    let mut movements = Vec::new();
    if free_white_move {
        legal_movements(&position, &mut movements);
    } else if advance_aux(&mut position, &AdvanceOptions::default(), &mut movements).is_err() {
        return Vec::new();
    }

    movements
        .into_iter()
        .filter(|movement| !is_illegal_pawn_drop_mate(world, movement))
        .flat_map(|movement| {
            world
                .observed_variants(&movement, hand_variable_mode)
                .into_iter()
                .map(move |(observed, selected_variable)| (observed, movement, selected_variable))
        })
        .collect()
}

/// 合法手生成では打歩詰めも一旦候補に含まれるため、着手として公開する前に除く。
/// 覆面駒では、この除外によって「歩なら不合法」という候補世界だけが消える。
fn is_illegal_pawn_drop_mate(world: &ConcreteWorld, movement: &Movement) -> bool {
    if !movement.is_pawn_drop() {
        return false;
    }

    let mut position = world.position().clone();
    position.do_move(movement);
    is_legal_mate(&mut position)
}

#[cfg(test)]
mod tests {
    use fmrs_core::position::{position::PositionAux, Square};

    use super::*;
    use crate::{MoveIdentity, VariableLocation, VariablePiece};

    fn world(sfen: &str) -> ConcreteWorld {
        ConcreteWorld::new(PositionAux::from_sfen(sfen).unwrap(), Vec::new())
    }

    #[test]
    fn help_selfmate_requires_mate_in_every_remaining_world() {
        let mated = world("7rK/7g1/9/9/9/9/9/9/4k4 b - 1");
        let not_mated = world("7rK/9/9/9/9/9/9/9/4k4 b - 1");

        assert!(
            HiddenState::new(vec![mated.clone(), mated], MateRule::HelpSelfmate).is_proven_mate()
        );
        assert!(!HiddenState::new(
            vec![world("7rK/7g1/9/9/9/9/9/9/4k4 b - 1"), not_mated],
            MateRule::HelpSelfmate
        )
        .is_proven_mate());
    }

    #[test]
    fn black_can_move_when_reverse_check_exists_in_only_some_worlds() {
        let variable = |kind| VariablePiece {
            id: VariableId(1),
            color: Color::WHITE,
            kind,
            location: VariableLocation::Board(Square::S88),
        };
        // 88金の世界だけ99玉が逆王手を受けている。88桂の世界では、
        // 19飛を12へ動かして11玉へ王手できる。
        let checked = ConcreteWorld::new(
            PositionAux::from_sfen("8k/9/9/9/9/9/9/1g7/K7R b - 1").unwrap(),
            vec![variable(Kind::Gold)],
        );
        let unchecked = ConcreteWorld::new(
            PositionAux::from_sfen("8k/9/9/9/9/9/9/1n7/K7R b - 1").unwrap(),
            vec![variable(Kind::Knight)],
        );
        let state = HiddenState::new(vec![checked, unchecked], MateRule::HelpSelfmate);
        let checking_move = ObservedMove::Move {
            identity: MoveIdentity::Known,
            source: Square::S19,
            destination: Square::S12,
            promote: false,
        };

        assert!(state.observed_moves().contains(&checking_move));
        assert_eq!(state.apply(checking_move).unwrap().world_count(), 1);
    }
}
