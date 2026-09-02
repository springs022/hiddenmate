use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

use anyhow::{bail, Result};

use fmrs_core::{
    piece::{Color, Kind},
    position::{
        advance::{advance::advance_aux, is_legal_mate, legal_movements},
        AdvanceOptions, Movement,
    },
};

use crate::{
    kind_set::KindSet, problem::for_each_initial_world, world_id_set::WorldIdSet, ConcreteWorld,
    HandVariableMode, MateRule, ObservedMove, VariableId, VariableProblem,
};

const REPLAY_CHECKPOINT_INTERVAL: usize = 2;

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
            let candidates = KindSet::from_iter(
                self.worlds
                    .iter()
                    .filter_map(|world| world.variable(id).map(|piece| piece.kind)),
            );
            if candidates.len() != 1 {
                continue;
            }
            let kind = candidates.iter().next().expect("候補が一種類ある");
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
        KindSet::from_iter(
            self.worlds
                .iter()
                .filter_map(|world| world.variable(id).map(|piece| piece.kind)),
        )
        .to_btree_set()
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
                MateRule::Helpmate => {
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

/// 具体世界を永続保持せず、問題と観測履歴から必要時に候補世界を再生する状態。
#[derive(Clone, Debug)]
pub struct ReplayHiddenState {
    context: Arc<ReplayHiddenContext>,
    history: Vec<ObservedMove>,
    /// 初形、および各観測着手後に新たに確定した覆面駒。
    resolved_steps: Vec<BTreeMap<VariableId, Kind>>,
    resolved: BTreeMap<VariableId, Kind>,
    world_count: usize,
    initial_turn: Color,
    free_white_move: bool,
    rule: MateRule,
    hand_variable_mode: HandVariableMode,
    active_initials: WorldIdSet,
    checkpoint: HiddenCheckpoint,
    transition_cache: Arc<OnceLock<BTreeMap<ObservedMove, HiddenTransition>>>,
    mate_cache: Arc<OnceLock<bool>>,
}

#[derive(Debug)]
struct ReplayHiddenContext {
    initial_worlds: Arc<Vec<SeededConcreteWorld>>,
}

#[derive(Clone, Debug)]
struct SeededConcreteWorld {
    initial_id: usize,
    world: ConcreteWorld,
}

#[derive(Clone, Debug)]
struct HiddenCheckpoint {
    depth: usize,
    worlds: Arc<Vec<SeededConcreteWorld>>,
}

#[derive(Clone, Debug)]
struct HiddenTransition {
    worlds: Arc<Vec<SeededConcreteWorld>>,
    newly_resolved: BTreeMap<VariableId, Kind>,
    active_initials: WorldIdSet,
}

impl ReplayHiddenState {
    pub(crate) fn new(
        problem: VariableProblem,
        hand_variable_mode: HandVariableMode,
    ) -> Result<Self> {
        let mut initial_worlds = Vec::new();
        let mut candidates = BTreeMap::<VariableId, KindSet>::new();
        let mut initial_turn = None;
        for_each_initial_world(&problem, &mut |world| {
            initial_turn.get_or_insert(world.position().turn());
            extend_candidate_sets(&mut candidates, &world);
            initial_worlds.push(world);
            Ok(())
        })?;
        if initial_worlds.is_empty() {
            bail!("初形の合法性と矛盾しない覆面駒の割当がありません");
        }
        let initial_resolved = uniquely_resolved(&candidates);
        for world in &mut initial_worlds {
            forget_resolved(world, &initial_resolved);
        }
        let initial_worlds = Arc::new(
            initial_worlds
                .into_iter()
                .enumerate()
                .map(|(initial_id, world)| SeededConcreteWorld { initial_id, world })
                .collect::<Vec<_>>(),
        );
        let world_count = initial_worlds.len();
        let initial_turn = initial_turn.expect("候補世界が存在する");
        Ok(Self {
            context: Arc::new(ReplayHiddenContext {
                initial_worlds: initial_worlds.clone(),
            }),
            history: Vec::new(),
            resolved_steps: vec![initial_resolved.clone()],
            resolved: initial_resolved,
            world_count,
            initial_turn,
            free_white_move: initial_turn.is_white(),
            rule: problem.rule,
            hand_variable_mode,
            active_initials: WorldIdSet::full(world_count),
            checkpoint: HiddenCheckpoint {
                depth: 0,
                worlds: initial_worlds,
            },
            transition_cache: Arc::new(OnceLock::new()),
            mate_cache: Arc::new(OnceLock::new()),
        })
    }

    pub fn world_count(&self) -> usize {
        self.world_count
    }

    pub fn turn(&self) -> Color {
        if self.history.len() % 2 == 0 {
            self.initial_turn
        } else {
            self.initial_turn.opposite()
        }
    }

    pub fn rule(&self) -> MateRule {
        self.rule
    }

    pub fn hand_variable_mode(&self) -> HandVariableMode {
        self.hand_variable_mode
    }

    pub fn resolved_kind(&self, id: VariableId) -> Option<Kind> {
        self.resolved.get(&id).copied()
    }

    pub fn candidates(&self, id: VariableId) -> Result<BTreeSet<Kind>> {
        if let Some(&kind) = self.resolved.get(&id) {
            return Ok([kind].into_iter().collect());
        }
        let mut result = KindSet::default();
        self.for_each_world(&mut |_, world| {
            if let Some(piece) = world.variable(id) {
                result.insert(piece.kind);
            }
            Ok(())
        })?;
        Ok(result.to_btree_set())
    }

    pub fn observed_moves(&self) -> Result<Vec<ObservedMove>> {
        Ok(self.transitions()?.keys().copied().collect())
    }

    pub fn apply(&self, observed: ObservedMove) -> Result<Option<Self>> {
        let Some(transition) = self.transitions()?.get(&observed).cloned() else {
            return Ok(None);
        };

        let mut next = self.clone();
        next.history.push(observed);
        next.resolved_steps.push(transition.newly_resolved.clone());
        next.resolved.extend(transition.newly_resolved);
        next.world_count = transition.worlds.len();
        next.active_initials = transition.active_initials;
        if next.history.len() % REPLAY_CHECKPOINT_INTERVAL == 0 {
            next.checkpoint = HiddenCheckpoint {
                depth: next.history.len(),
                worlds: transition.worlds,
            };
        }
        next.transition_cache = Arc::new(OnceLock::new());
        next.mate_cache = Arc::new(OnceLock::new());
        Ok(Some(next))
    }

    pub fn is_proven_mate(&self) -> Result<bool> {
        if let Some(&cached) = self.mate_cache.get() {
            return Ok(cached);
        }
        if self.turn() != self.rule.terminal_turn() {
            let _ = self.mate_cache.set(false);
            return Ok(false);
        }
        let mut all_mated = true;
        self.for_each_world(&mut |_, world| {
            let mut position = world.position().clone();
            let mated = match self.rule {
                MateRule::Helpmate => {
                    let mut movements = Vec::new();
                    matches!(
                        advance_aux(&mut position, &AdvanceOptions::default(), &mut movements),
                        Ok(true)
                    ) && movements.is_empty()
                }
                MateRule::HelpSelfmate => {
                    fmrs_core::solve::help_selfmate::is_help_selfmate(&mut position)
                }
            };
            all_mated &= mated;
            Ok(())
        })?;
        let _ = self.mate_cache.set(all_mated);
        Ok(all_mated)
    }

    fn transitions(&self) -> Result<&BTreeMap<ObservedMove, HiddenTransition>> {
        if self.transition_cache.get().is_none() {
            let transitions = self.build_transitions()?;
            let _ = self.transition_cache.set(transitions);
        }
        Ok(self
            .transition_cache
            .get()
            .expect("遷移キャッシュを設定済み"))
    }

    fn build_transitions(&self) -> Result<BTreeMap<ObservedMove, HiddenTransition>> {
        let mut grouped = BTreeMap::<ObservedMove, Vec<SeededConcreteWorld>>::new();
        let free_white_move = self.history.is_empty() && self.free_white_move;
        self.for_each_world(&mut |initial_id, world| {
            for (observed, movement, selected_variable) in
                concrete_moves(&world, free_white_move, self.hand_variable_mode)
            {
                grouped
                    .entry(observed)
                    .or_default()
                    .push(SeededConcreteWorld {
                        initial_id,
                        world: world.apply(observed, movement, selected_variable),
                    });
            }
            Ok(())
        })?;

        Ok(grouped
            .into_iter()
            .map(|(observed, mut worlds)| {
                let mut candidates = BTreeMap::<VariableId, KindSet>::new();
                for seeded in &worlds {
                    extend_candidate_sets(&mut candidates, &seeded.world);
                }
                let newly_resolved = uniquely_resolved(&candidates);
                let mut active_initials =
                    WorldIdSet::with_capacity(self.context.initial_worlds.len());
                for seeded in &mut worlds {
                    forget_resolved(&mut seeded.world, &newly_resolved);
                    active_initials.insert(seeded.initial_id);
                }
                (
                    observed,
                    HiddenTransition {
                        worlds: Arc::new(worlds),
                        newly_resolved,
                        active_initials,
                    },
                )
            })
            .collect())
    }

    fn for_each_world(
        &self,
        visitor: &mut impl FnMut(usize, ConcreteWorld) -> Result<()>,
    ) -> Result<()> {
        for seeded in self.checkpoint.worlds.iter() {
            if !self.active_initials.contains(seeded.initial_id) {
                continue;
            }
            let mut frontier = vec![seeded.world.clone()];
            for (step, observed) in self
                .history
                .iter()
                .copied()
                .enumerate()
                .skip(self.checkpoint.depth)
            {
                let mut next_frontier = Vec::new();
                for world in frontier {
                    let free_white_move = step == 0 && self.free_white_move;
                    for (candidate, movement, selected_variable) in
                        concrete_moves(&world, free_white_move, self.hand_variable_mode)
                    {
                        if candidate == observed {
                            let mut next = world.apply(observed, movement, selected_variable);
                            forget_resolved(&mut next, &self.resolved_steps[step + 1]);
                            next_frontier.push(next);
                        }
                    }
                }
                frontier = next_frontier;
                if frontier.is_empty() {
                    break;
                }
            }
            for world in frontier {
                visitor(seeded.initial_id, world)?;
            }
        }
        Ok(())
    }
}

fn extend_candidate_sets(candidates: &mut BTreeMap<VariableId, KindSet>, world: &ConcreteWorld) {
    for piece in world.variables() {
        candidates.entry(piece.id).or_default().insert(piece.kind);
    }
}

fn uniquely_resolved(candidates: &BTreeMap<VariableId, KindSet>) -> BTreeMap<VariableId, Kind> {
    candidates
        .iter()
        .filter(|(_, kinds)| kinds.len() == 1)
        .map(|(&id, kinds)| (id, kinds.iter().next().expect("候補が一種類ある")))
        .collect()
}

fn forget_resolved(world: &mut ConcreteWorld, resolved: &BTreeMap<VariableId, Kind>) {
    for id in resolved.keys().copied() {
        world.forget_variable(id);
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
