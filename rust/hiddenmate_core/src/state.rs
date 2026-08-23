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
}

impl HiddenState {
    pub(crate) fn new(worlds: Vec<ConcreteWorld>) -> Self {
        debug_assert!(!worlds.is_empty());
        Self { worlds }
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
        self.worlds
            .iter()
            .filter_map(|world| world.variable(id).map(|piece| piece.kind))
            .collect()
    }

    pub fn resolved_kind(&self, id: VariableId) -> Option<Kind> {
        let candidates = self.candidates(id);
        if candidates.len() == 1 {
            candidates.into_iter().next()
        } else {
            None
        }
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
            Some(Self::new(next_worlds))
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
    if !movement.is_pawn_drop() {
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
