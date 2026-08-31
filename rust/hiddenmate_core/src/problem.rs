use std::{collections::BTreeSet, time::Instant};

use anyhow::{bail, Context, Result};
use fmrs_core::{
    piece::{Color, Kind, KINDS, NUM_HAND_KIND},
    position::position::PositionAux,
};

use crate::{
    kind_set::KindSet, ConcreteWorld, EnumerationMetrics, HandVariableMode, HiddenState, MateRule,
    ReplayHiddenState, VariableId, VariableLocation, VariablePiece,
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
        self.enumerate_explicit_with_hand_variable_mode(hand_variable_mode)
    }

    /// すべての具体世界を列挙する参照実装。
    ///
    /// 将来の共有・遅延バックエンドは、この結果との一致をテストする。
    pub fn enumerate_explicit(self) -> Result<HiddenState> {
        self.enumerate_explicit_with_hand_variable_mode(HandVariableMode::default())
    }

    pub fn enumerate_profiled(self) -> Result<(HiddenState, EnumerationMetrics)> {
        self.enumerate_profiled_with_hand_variable_mode(HandVariableMode::default())
    }

    pub fn enumerate_profiled_with_hand_variable_mode(
        self,
        hand_variable_mode: HandVariableMode,
    ) -> Result<(HiddenState, EnumerationMetrics)> {
        let started = Instant::now();
        let state = self.enumerate_explicit_with_hand_variable_mode(hand_variable_mode)?;
        let metrics = EnumerationMetrics {
            world_count: state.world_count(),
            elapsed: started.elapsed(),
        };
        Ok((state, metrics))
    }

    pub fn enumerate_explicit_with_hand_variable_mode(
        self,
        hand_variable_mode: HandVariableMode,
    ) -> Result<HiddenState> {
        let mut worlds = Vec::new();
        for_each_initial_world(&self, &mut |world| {
            worlds.push(world);
            Ok(())
        })?;
        if worlds.is_empty() {
            bail!("初形の合法性と矛盾しない覆面駒の割当がありません");
        }
        Ok(HiddenState::new_with_hand_variable_mode(
            worlds,
            self.rule,
            hand_variable_mode,
        ))
    }

    /// 初形問題と観測履歴だけを保持し、候補世界を操作時に再生する。
    pub fn enumerate_replay(self) -> Result<ReplayHiddenState> {
        self.enumerate_replay_with_hand_variable_mode(HandVariableMode::default())
    }

    pub fn enumerate_replay_profiled(self) -> Result<(ReplayHiddenState, EnumerationMetrics)> {
        self.enumerate_replay_profiled_with_hand_variable_mode(HandVariableMode::default())
    }

    pub fn enumerate_replay_with_hand_variable_mode(
        self,
        hand_variable_mode: HandVariableMode,
    ) -> Result<ReplayHiddenState> {
        ReplayHiddenState::new(self, hand_variable_mode)
    }

    pub fn enumerate_replay_profiled_with_hand_variable_mode(
        self,
        hand_variable_mode: HandVariableMode,
    ) -> Result<(ReplayHiddenState, EnumerationMetrics)> {
        let started = Instant::now();
        let state = ReplayHiddenState::new(self, hand_variable_mode)?;
        let metrics = EnumerationMetrics {
            world_count: state.world_count(),
            elapsed: started.elapsed(),
        };
        Ok((state, metrics))
    }
}

pub(crate) fn for_each_initial_world(
    problem: &VariableProblem,
    visitor: &mut impl FnMut(ConcreteWorld) -> Result<()>,
) -> Result<()> {
    let base = PositionAux::from_sfen(&problem.base_sfen)
        .with_context(|| format!("SFENを解釈できません: {}", problem.base_sfen))?;
    validate_specs(&base, &problem.variables)?;
    enumerate_assignments(
        &base,
        &problem.variables,
        problem.rule,
        0,
        Vec::new(),
        visitor,
    )
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
        if KindSet::from_iter(spec.candidates.iter().copied()).is_empty() {
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
    visitor: &mut impl FnMut(ConcreteWorld) -> Result<()>,
) -> Result<()> {
    if index == specs.len() {
        return build_worlds(base, assigned, rule, visitor);
    }

    let spec = &specs[index];
    let candidates = KindSet::from_iter(spec.candidates.iter().copied());
    for kind in candidates.iter() {
        let mut next = assigned.clone();
        next.push(VariablePiece {
            id: spec.id,
            color: spec.color,
            kind,
            location: spec.location,
        });
        enumerate_assignments(base, specs, rule, index + 1, next, visitor)?;
    }
    Ok(())
}

fn build_worlds(
    base: &PositionAux,
    variables: Vec<VariablePiece>,
    rule: MateRule,
    visitor: &mut impl FnMut(ConcreteWorld) -> Result<()>,
) -> Result<()> {
    let mut assigned_counts = [0usize; NUM_SOURCE_KIND];
    for piece in &variables {
        let base_kind = piece.kind.maybe_unpromote();
        if base_kind.index() >= NUM_SOURCE_KIND {
            return Ok(());
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
            return Ok(());
        }
        box_limits[index] = max - used;
        if assigned_counts[index] > white_limits[index] + box_limits[index] {
            return Ok(());
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
        visitor,
    )
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
    visitor: &mut impl FnMut(ConcreteWorld) -> Result<()>,
) -> Result<()> {
    if index == NUM_SOURCE_KIND {
        if let Some(world) = build_world(base.clone(), variables.to_vec(), rule, &white_consumed) {
            visitor(world)?;
        }
        return Ok(());
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
            visitor,
        )?;
    }
    Ok(())
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
