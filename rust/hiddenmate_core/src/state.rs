use std::collections::{BTreeMap, BTreeSet};

use fmrs_core::{
    piece::{Color, Kind},
    position::{advance::advance::advance_aux, AdvanceOptions, Movement},
};

use crate::{ConcreteWorld, ObservedMove, VariableId};

/// これまでの観測と矛盾しない具体世界の集合。
#[derive(Clone, Debug)]
pub struct HiddenState {
    worlds: Vec<ConcreteWorld>,
    /// 一意に確定して普通の駒へ戻った覆面駒。
    resolved: BTreeMap<VariableId, Kind>,
}

impl HiddenState {
    pub(crate) fn new(worlds: Vec<ConcreteWorld>) -> Self {
        Self::with_resolved(worlds, BTreeMap::new())
    }

    fn with_resolved(
        worlds: Vec<ConcreteWorld>,
        resolved: BTreeMap<VariableId, Kind>,
    ) -> Self {
        debug_assert!(!worlds.is_empty());
        let mut state = Self { worlds, resolved };
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
            for (observed, _) in concrete_moves(world) {
                result.insert(observed);
            }
        }
        result.into_iter().collect()
    }

    /// 観測着手が可能な世界だけを残し、その具体着手を適用する。
    pub fn apply(&self, observed: ObservedMove) -> Option<Self> {
        let mut next_worlds = Vec::new();
        for world in &self.worlds {
            for (candidate, movement) in concrete_moves(world) {
                if candidate == observed {
                    next_worlds.push(world.apply(observed, movement));
                }
            }
        }
        if next_worlds.is_empty() {
            None
        } else {
            Some(Self::with_resolved(next_worlds, self.resolved.clone()))
        }
    }

    /// 残るすべての候補世界で受方に合法応手がないときだけ詰みとする。
    pub fn is_proven_mate(&self) -> bool {
        if self.turn().is_black() {
            return false;
        }
        self.worlds.iter().all(|world| {
            let mut position = world.position().clone();
            let mut movements = Vec::new();
            matches!(
                advance_aux(&mut position, &AdvanceOptions::default(), &mut movements),
                Ok(true)
            ) && movements.is_empty()
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

fn concrete_moves(world: &ConcreteWorld) -> Vec<(ObservedMove, Movement)> {
    let mut position = world.position().clone();
    let mut movements = Vec::new();
    if advance_aux(&mut position, &AdvanceOptions::default(), &mut movements).is_err() {
        return Vec::new();
    }

    movements
        .into_iter()
        .filter(|movement| !is_illegal_pawn_drop_mate(world, movement))
        .flat_map(|movement| {
            world
                .observed_variants(&movement)
                .into_iter()
                .map(move |observed| (observed, movement))
        })
        .collect()
}

/// 黒の王手生成では打歩詰めも一旦候補に含まれるため、着手として公開する前に除く。
/// 覆面駒では、この除外によって「歩なら不合法」という候補世界だけが消える。
fn is_illegal_pawn_drop_mate(world: &ConcreteWorld, movement: &Movement) -> bool {
    // このソルバーで攻方だけに課している王手義務を使った応手生成は、
    // 受方の歩打が打歩詰めかどうかの判定には利用できない。
    if world.position().turn().is_white() || !movement.is_pawn_drop() {
        return false;
    }

    let mut position = world.position().clone();
    position.do_move(movement);
    let mut replies = Vec::new();
    matches!(
        advance_aux(&mut position, &AdvanceOptions::default(), &mut replies),
        Ok(false)
    ) && replies.is_empty()
}
