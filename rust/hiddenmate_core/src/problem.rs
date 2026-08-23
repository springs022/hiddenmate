use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use fmrs_core::{
    piece::{Color, Kind, KINDS, NUM_HAND_KIND},
    position::{position::PositionAux, Square},
};

use crate::{ConcreteWorld, HiddenState, VariableId, VariableLocation, VariablePiece};

/// 初形で位置・所属が判明している覆面駒。
#[derive(Debug, Clone)]
pub struct VariableSpec {
    pub id: VariableId,
    pub color: Color,
    pub square: Square,
    pub candidates: Vec<Kind>,
}

/// 覆面駒協力詰の初形。
///
/// `base_sfen` には覆面駒を置かず、攻方の明示された持駒だけを書く。
/// 受方持駒は、各候補世界で標準40枚から盤上駒・攻方持駒・覆面駒を
/// 引いた残りとして補完する。
#[derive(Debug, Clone)]
pub struct VariableProblem {
    pub base_sfen: String,
    pub variables: Vec<VariableSpec>,
}

impl VariableProblem {
    pub fn enumerate(self) -> Result<HiddenState> {
        let base = PositionAux::from_sfen(&self.base_sfen)
            .with_context(|| format!("SFENを解釈できません: {}", self.base_sfen))?;

        if !base.hands().is_empty(Color::WHITE) {
            bail!("base_sfenの受方持駒は空にしてください。標準駒数から自動補完します");
        }
        validate_specs(&base, &self.variables)?;

        let mut worlds = Vec::new();
        enumerate_assignments(&base, &self.variables, 0, Vec::new(), &mut worlds);
        if worlds.is_empty() {
            bail!("初形の合法性と矛盾しない覆面駒の割当がありません");
        }
        Ok(HiddenState::new(worlds))
    }
}

fn validate_specs(base: &PositionAux, specs: &[VariableSpec]) -> Result<()> {
    let mut ids = BTreeSet::new();
    let mut squares = BTreeSet::new();
    for spec in specs {
        if !ids.insert(spec.id) {
            bail!("覆面駒ID {:?} が重複しています", spec.id);
        }
        if !squares.insert(spec.square) {
            bail!("覆面駒の配置マス {:?} が重複しています", spec.square);
        }
        if base.get(spec.square).is_some() {
            bail!("覆面駒の配置マス {:?} に通常駒があります", spec.square);
        }
        if spec.candidates.is_empty() {
            bail!("覆面駒ID {:?} の候補が空です", spec.id);
        }
    }
    Ok(())
}

fn enumerate_assignments(
    base: &PositionAux,
    specs: &[VariableSpec],
    index: usize,
    assigned: Vec<VariablePiece>,
    worlds: &mut Vec<ConcreteWorld>,
) {
    if index == specs.len() {
        if let Some(world) = build_world(base.clone(), assigned) {
            worlds.push(world);
        }
        return;
    }

    let spec = &specs[index];
    let mut unique_candidates = BTreeSet::new();
    for &kind in &spec.candidates {
        if !unique_candidates.insert(kind) {
            continue;
        }
        let mut next = assigned.clone();
        next.push(VariablePiece {
            id: spec.id,
            color: spec.color,
            kind,
            location: VariableLocation::Board(spec.square),
        });
        enumerate_assignments(base, specs, index + 1, next, worlds);
    }
}

fn build_world(mut position: PositionAux, variables: Vec<VariablePiece>) -> Option<ConcreteWorld> {
    for piece in &variables {
        let VariableLocation::Board(square) = piece.location else {
            return None;
        };
        if position.get(square).is_some() {
            return None;
        }
        position.set(square, piece.color, piece.kind);
    }

    // 王以外の不足駒をすべて受方持駒として補完する。成駒も生駒と
    // 合算して標準駒数を数える。
    for &kind in &KINDS[..NUM_HAND_KIND] {
        let used = inventory_count(&position, kind);
        let max = kind.max_count() as usize;
        if used > max {
            return None;
        }
        position.hands_mut().add_n(Color::WHITE, kind, max - used);
    }

    if position.is_illegal_initial_position() {
        return None;
    }

    let white_kings = position.bitboard(Color::WHITE, Kind::King).count_ones();
    let black_kings = position.bitboard(Color::BLACK, Kind::King).count_ones();
    if white_kings != 1 || black_kings > 1 {
        return None;
    }

    let mut checked_position = position.clone();
    let black_checked = checked_position.checked_slow(Color::BLACK);
    let white_checked = checked_position.checked_slow(Color::WHITE);
    if black_checked && white_checked {
        return None;
    }
    if position.turn().is_black() && (black_checked || white_checked) {
        return None;
    }
    if position.turn().is_white() && black_checked {
        return None;
    }

    Some(ConcreteWorld::new(position, variables))
}

fn inventory_count(position: &PositionAux, base_kind: Kind) -> usize {
    let mut count = position.hands().count(Color::BLACK, base_kind)
        + position.hands().count(Color::WHITE, base_kind);
    count += position.kind_bb(base_kind).count_ones() as usize;
    if let Some(promoted) = base_kind.promote() {
        count += position.kind_bb(promoted).count_ones() as usize;
    }
    count
}
