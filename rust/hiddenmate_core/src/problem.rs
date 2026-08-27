use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use fmrs_core::{
    piece::{Color, Kind, KINDS, NUM_HAND_KIND},
    position::position::PositionAux,
};

use crate::{
    ConcreteWorld, HandVariableMode, HiddenState, MateRule, VariableId, VariableLocation,
    VariablePiece,
};

const MAX_VARIABLES: usize = 6;
const NUM_SOURCE_KIND: usize = NUM_HAND_KIND + 1;

/// 初形で位置・所属が判明している覆面駒。
#[derive(Debug, Clone)]
pub struct VariableSpec {
    pub id: VariableId,
    pub color: Color,
    pub location: VariableLocation,
    pub candidates: Vec<Kind>,
}

/// 覆面駒協力詰の初形。
///
/// `base_sfen` には覆面駒を置かず、通常駒だけを書く。
/// 覆面駒の正体は、base局面の受方持駒または標準駒数から算出される
/// 駒箱の在庫から割り当てる。
#[derive(Debug, Clone)]
pub struct VariableProblem {
    pub base_sfen: String,
    pub variables: Vec<VariableSpec>,
    pub rule: MateRule,
}

impl VariableProblem {
    pub fn enumerate(self) -> Result<HiddenState> {
        self.enumerate_with_hand_variable_mode(HandVariableMode::default())
    }

    pub fn enumerate_with_hand_variable_mode(
        self,
        hand_variable_mode: HandVariableMode,
    ) -> Result<HiddenState> {
        let base = PositionAux::from_sfen(&self.base_sfen)
            .with_context(|| format!("SFENを解釈できません: {}", self.base_sfen))?;

        validate_specs(&base, &self.variables)?;

        let mut worlds = Vec::new();
        enumerate_assignments(
            &base,
            &self.variables,
            self.rule,
            0,
            Vec::new(),
            &mut worlds,
        );
        if worlds.is_empty() {
            bail!("初形の合法性と矛盾しない覆面駒の割当がありません");
        }
        Ok(HiddenState::new_with_hand_variable_mode(
            worlds,
            self.rule,
            hand_variable_mode,
        ))
    }
}

fn validate_specs(base: &PositionAux, specs: &[VariableSpec]) -> Result<()> {
    if specs.len() > MAX_VARIABLES {
        bail!("覆面駒は6枚まで指定できます");
    }
    let mut ids = BTreeSet::new();
    let mut squares = BTreeSet::new();
    for spec in specs {
        if !ids.insert(spec.id) {
            bail!("覆面駒ID {:?} が重複しています", spec.id);
        }
        if let VariableLocation::Board(square) = spec.location {
            if !squares.insert(square) {
                bail!("覆面駒の配置マス {:?} が重複しています", square);
            }
            if base.get(square).is_some() {
                bail!("覆面駒の配置マス {:?} に通常駒があります", square);
            }
        } else if spec.location != VariableLocation::Hand(spec.color) {
            bail!("覆面駒ID {:?} の所属と駒台が一致しません", spec.id);
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
    rule: MateRule,
    index: usize,
    assigned: Vec<VariablePiece>,
    worlds: &mut Vec<ConcreteWorld>,
) {
    if index == specs.len() {
        build_worlds(base, assigned, rule, worlds);
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
            location: spec.location,
        });
        enumerate_assignments(base, specs, rule, index + 1, next, worlds);
    }
}

fn build_worlds(
    base: &PositionAux,
    variables: Vec<VariablePiece>,
    rule: MateRule,
    worlds: &mut Vec<ConcreteWorld>,
) {
    let mut assigned_counts = [0usize; NUM_SOURCE_KIND];
    for piece in &variables {
        let base_kind = piece.kind.maybe_unpromote();
        if base_kind.index() >= NUM_SOURCE_KIND {
            return;
        }
        assigned_counts[base_kind.index()] += 1;
    }

    let mut white_limits = [0usize; NUM_SOURCE_KIND];
    let mut box_limits = [0usize; NUM_SOURCE_KIND];
    for &kind in &KINDS[..NUM_SOURCE_KIND] {
        let index = kind.index();
        if index < NUM_HAND_KIND {
            white_limits[index] = base.hands().count(Color::WHITE, kind);
        }
        let used = inventory_count(base, kind);
        let max = kind.max_count() as usize;
        if used > max {
            return;
        }
        box_limits[index] = max - used;
        if assigned_counts[index] > white_limits[index] + box_limits[index] {
            return;
        }
    }

    enumerate_source_counts(
        base,
        &variables,
        rule,
        &assigned_counts,
        &white_limits,
        &box_limits,
        0,
        [0; NUM_SOURCE_KIND],
        worlds,
    );
}

#[allow(clippy::too_many_arguments)]
fn enumerate_source_counts(
    base: &PositionAux,
    variables: &[VariablePiece],
    rule: MateRule,
    assigned_counts: &[usize; NUM_SOURCE_KIND],
    white_limits: &[usize; NUM_SOURCE_KIND],
    box_limits: &[usize; NUM_SOURCE_KIND],
    index: usize,
    mut white_consumed: [usize; NUM_SOURCE_KIND],
    worlds: &mut Vec<ConcreteWorld>,
) {
    if index == NUM_SOURCE_KIND {
        if let Some(world) = build_world(base.clone(), variables.to_vec(), rule, &white_consumed) {
            worlds.push(world);
        }
        return;
    }

    let assigned = assigned_counts[index];
    let min_from_white = assigned.saturating_sub(box_limits[index]);
    let max_from_white = assigned.min(white_limits[index]);
    for count in min_from_white..=max_from_white {
        white_consumed[index] = count;
        enumerate_source_counts(
            base,
            variables,
            rule,
            assigned_counts,
            white_limits,
            box_limits,
            index + 1,
            white_consumed,
            worlds,
        );
    }
}

fn build_world(
    mut position: PositionAux,
    variables: Vec<VariablePiece>,
    rule: MateRule,
    white_consumed: &[usize; NUM_SOURCE_KIND],
) -> Option<ConcreteWorld> {
    for &kind in &KINDS[..NUM_HAND_KIND] {
        position
            .hands_mut()
            .remove_n(Color::WHITE, kind, white_consumed[kind.index()]);
    }
    for piece in &variables {
        match piece.location {
            VariableLocation::Board(square) => {
                if position.get(square).is_some() {
                    return None;
                }
                position.set(square, piece.color, piece.kind);
            }
            VariableLocation::Hand(color) => {
                if piece.kind.index() >= NUM_HAND_KIND || color != piece.color {
                    return None;
                }
                position.hands_mut().add(color, piece.kind);
            }
        }
    }

    if position.is_illegal_initial_position() {
        return None;
    }

    let white_kings = position.bitboard(Color::WHITE, Kind::King).count_ones();
    let black_kings = position.bitboard(Color::BLACK, Kind::King).count_ones();
    if white_kings != 1
        || match rule {
            MateRule::Helpmate => black_kings > 1,
            MateRule::HelpSelfmate => black_kings != 1,
        }
    {
        return None;
    }

    let mut checked_position = position.clone();
    let black_checked = checked_position.checked_slow(Color::BLACK);
    let white_checked = checked_position.checked_slow(Color::WHITE);
    if black_checked && white_checked {
        return None;
    }
    // 初形で王手されていてはならないのは「手番でない側」の玉だけ。
    // 攻方手番では、攻方玉への王手を外しながら王手する着手も合法になり得る。
    if position.turn().is_black() && white_checked {
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
