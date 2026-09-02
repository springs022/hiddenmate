use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use fmrs_core::{
    piece::{Color, Kind},
    position::{
        advance::{advance::advance_aux, is_legal_mate, legal_movements},
        position::PositionAux,
        AdvanceOptions, Movement, Square,
    },
    sfen,
};
use serde::{Deserialize, Serialize};

use crate::{DropIdentity, MateRule, MoveIdentity, ObservedMove};

const MAX_KNOWN_INVISIBLES: usize = 2;
const MAX_WORLDS: usize = 20_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownInvisibleDocument {
    pub base_sfen: String,
    pub plies: usize,
    #[serde(default)]
    pub rule: MateRule,
    pub invisibles: Vec<KnownInvisibleDocumentSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownInvisibleDocumentSpec {
    pub color: DocumentColor,
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentColor {
    Black,
    White,
}

#[derive(Debug, Clone)]
pub struct KnownInvisibleProblem {
    pub base_sfen: String,
    pub invisibles: Vec<KnownInvisibleSpec>,
    pub rule: MateRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KnownInvisibleSpec {
    pub color: Color,
    pub kind: Kind,
}

impl KnownInvisibleDocument {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("問題JSONを解釈できません")
    }

    pub fn into_problem(self) -> Result<(KnownInvisibleProblem, usize)> {
        if self.rule == MateRule::BestMate {
            bail!("最善詰は現在、覆面駒でのみ利用できます");
        }
        let mut base = PositionAux::from_sfen(&self.base_sfen)
            .with_context(|| format!("SFENを解釈できません: {}", self.base_sfen))?;
        base.set_turn(self.rule.initial_turn(self.plies));
        let mut invisibles = Vec::new();
        for spec in self.invisibles {
            if spec.count == 0 {
                continue;
            }
            let color = match spec.color {
                DocumentColor::Black => Color::BLACK,
                DocumentColor::White => Color::WHITE,
            };
            let kind = parse_kind(&spec.kind)?;
            invisibles.extend((0..spec.count).map(|_| KnownInvisibleSpec { color, kind }));
        }
        if invisibles.len() > MAX_KNOWN_INVISIBLES {
            bail!("透明駒（駒種指定）は合計2枚まで指定できます");
        }
        Ok((
            KnownInvisibleProblem {
                base_sfen: sfen::encode_position(&base),
                invisibles,
                rule: self.rule,
            },
            self.plies,
        ))
    }
}

fn parse_kind(value: &str) -> Result<Kind> {
    match value.to_ascii_uppercase().as_str() {
        "P" => Ok(Kind::Pawn),
        "L" => Ok(Kind::Lance),
        "N" => Ok(Kind::Knight),
        "S" => Ok(Kind::Silver),
        "G" => Ok(Kind::Gold),
        "B" => Ok(Kind::Bishop),
        "R" => Ok(Kind::Rook),
        "K" => Ok(Kind::King),
        "+P" => Ok(Kind::ProPawn),
        "+L" => Ok(Kind::ProLance),
        "+N" => Ok(Kind::ProKnight),
        "+S" => Ok(Kind::ProSilver),
        "+B" => Ok(Kind::ProBishop),
        "+R" => Ok(Kind::ProRook),
        _ => bail!("未知の駒種 `{value}` です"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum InvisibleLocation {
    Board(Square),
    Hand(Color),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InvisiblePiece {
    color: Color,
    kind: Kind,
    location: InvisibleLocation,
}

#[derive(Clone, Debug)]
struct InvisibleWorld {
    position: PositionAux,
    invisibles: Vec<InvisiblePiece>,
}

impl InvisibleWorld {
    fn new(position: PositionAux, mut invisibles: Vec<InvisiblePiece>) -> Self {
        invisibles.sort_unstable();
        Self {
            position,
            invisibles,
        }
    }

    fn key(&self) -> String {
        format!(
            "{}|{:?}",
            sfen::encode_position(&self.position),
            self.invisibles
        )
    }

    fn invisible_index_on_board(&self, square: Square) -> Option<usize> {
        self.invisibles
            .iter()
            .position(|piece| piece.location == InvisibleLocation::Board(square))
    }

    fn transition_variants(
        &self,
        movement: Movement,
    ) -> Vec<(KnownInvisibleObservedMove, InvisibleWorld)> {
        match movement {
            Movement::Move {
                source,
                dest,
                promote,
                ..
            } => {
                let moving_hidden = self.invisible_index_on_board(source);
                let captured_hidden = self.invisible_index_on_board(dest);
                let captured_visible =
                    self.position.get(dest).is_some() && captured_hidden.is_none();
                let observed = if moving_hidden.is_some() {
                    if captured_visible {
                        KnownInvisibleObservedMove::InvisibleCapture(dest)
                    } else {
                        KnownInvisibleObservedMove::Invisible
                    }
                } else {
                    KnownInvisibleObservedMove::Known(ObservedMove::Move {
                        identity: MoveIdentity::Known,
                        source,
                        destination: dest,
                        promote,
                    })
                };
                vec![(
                    observed,
                    self.apply(movement, moving_hidden, captured_hidden),
                )]
            }
            Movement::Drop(destination, kind) => {
                let turn = self.position.turn();
                let hidden_indices: Vec<_> = self
                    .invisibles
                    .iter()
                    .enumerate()
                    .filter(|(_, piece)| {
                        piece.color == turn
                            && piece.kind == kind
                            && piece.location == InvisibleLocation::Hand(turn)
                    })
                    .map(|(index, _)| index)
                    .collect();
                let mut result = Vec::new();
                if self.position.hands().count(turn, kind) > hidden_indices.len() {
                    result.push((
                        KnownInvisibleObservedMove::Known(ObservedMove::Drop {
                            identity: DropIdentity::Known(kind),
                            destination,
                        }),
                        self.apply(movement, None, None),
                    ));
                }
                for index in hidden_indices {
                    result.push((
                        KnownInvisibleObservedMove::Invisible,
                        self.apply(movement, Some(index), None),
                    ));
                }
                result
            }
        }
    }

    fn apply(
        &self,
        movement: Movement,
        moving_hidden: Option<usize>,
        captured_hidden: Option<usize>,
    ) -> Self {
        let mover = self.position.turn();
        let mut next = self.clone();
        next.position.do_move(&movement);
        if let Some(index) = captured_hidden {
            let captured = &mut next.invisibles[index];
            captured.color = mover;
            captured.kind = captured.kind.maybe_unpromote();
            captured.location = InvisibleLocation::Hand(mover);
        }
        if let Some(index) = moving_hidden {
            let moved = &mut next.invisibles[index];
            match movement {
                Movement::Move { dest, promote, .. } => {
                    if promote {
                        moved.kind = moved.kind.promote().expect("合法な成である");
                    }
                    moved.location = InvisibleLocation::Board(dest);
                }
                Movement::Drop(destination, _) => {
                    moved.location = InvisibleLocation::Board(destination);
                }
            }
        }
        next.invisibles.sort_unstable();
        next
    }
}

#[derive(Clone, Debug)]
pub struct KnownInvisibleState {
    worlds: Vec<InvisibleWorld>,
    free_white_move: bool,
    rule: MateRule,
}

impl KnownInvisibleProblem {
    pub fn enumerate(self) -> Result<KnownInvisibleState> {
        let base = PositionAux::from_sfen(&self.base_sfen)
            .with_context(|| format!("SFENを解釈できません: {}", self.base_sfen))?;
        let mut specs = self.invisibles;
        specs.sort_unstable();
        let mut worlds = BTreeMap::new();
        for allocated_base in allocate_invisible_inventory(&base, &specs)? {
            enumerate_locations(
                &allocated_base,
                &specs,
                0,
                Vec::new(),
                self.rule,
                &mut worlds,
            )?;
        }
        if worlds.is_empty() {
            bail!("初形の合法性と矛盾しない透明駒の配置がありません");
        }
        let free_white_move = base.turn().is_white();
        Ok(KnownInvisibleState::from_worlds(
            worlds.into_values().collect(),
            free_white_move,
            self.rule,
        ))
    }
}

fn allocate_invisible_inventory(
    base: &PositionAux,
    specs: &[KnownInvisibleSpec],
) -> Result<Vec<PositionAux>> {
    let mut additions = BTreeMap::<Kind, usize>::new();
    for spec in specs {
        *additions.entry(spec.kind.maybe_unpromote()).or_default() += 1;
    }
    let allocations: Vec<_> = additions
        .into_iter()
        .map(|(kind, assigned)| {
            let white_limit = if kind.index() < fmrs_core::piece::NUM_HAND_KIND {
                base.hands().count(Color::WHITE, kind)
            } else {
                0
            };
            let box_limit = kind.max_count() as usize - inventory_count(base, kind);
            if assigned > white_limit + box_limit {
                bail!("透明駒に割り当てる{kind:?}が受方持駒または駒箱に足りません");
            }
            Ok((
                kind,
                assigned.saturating_sub(box_limit),
                assigned.min(white_limit),
            ))
        })
        .collect::<Result<_>>()?;
    let mut result = Vec::new();
    allocate_invisible_inventory_inner(base, &allocations, 0, &mut Vec::new(), &mut result);
    Ok(result)
}

fn allocate_invisible_inventory_inner(
    base: &PositionAux,
    allocations: &[(Kind, usize, usize)],
    index: usize,
    consumed: &mut Vec<(Kind, usize)>,
    result: &mut Vec<PositionAux>,
) {
    if index == allocations.len() {
        let mut allocated = base.clone();
        for &(kind, count) in consumed.iter() {
            allocated.hands_mut().remove_n(Color::WHITE, kind, count);
        }
        result.push(allocated);
        return;
    }
    let (kind, min_from_white, max_from_white) = allocations[index];
    for count in min_from_white..=max_from_white {
        consumed.push((kind, count));
        allocate_invisible_inventory_inner(base, allocations, index + 1, consumed, result);
        consumed.pop();
    }
}

fn inventory_count(base: &PositionAux, kind: Kind) -> usize {
    let mut used = base.hands().count(Color::BLACK, kind)
        + base.hands().count(Color::WHITE, kind)
        + base.kind_bb(kind).count_ones() as usize;
    if let Some(promoted) = kind.promote() {
        used += base.kind_bb(promoted).count_ones() as usize;
    }
    used
}

fn enumerate_locations(
    base: &PositionAux,
    specs: &[KnownInvisibleSpec],
    index: usize,
    pieces: Vec<InvisiblePiece>,
    rule: MateRule,
    worlds: &mut BTreeMap<String, InvisibleWorld>,
) -> Result<()> {
    if index == specs.len() {
        let mut position = base.clone();
        for piece in &pieces {
            match piece.location {
                InvisibleLocation::Board(square) => position.set(square, piece.color, piece.kind),
                InvisibleLocation::Hand(color) => position.hands_mut().add(color, piece.kind),
            }
        }
        if legal_initial_world(&position, rule) {
            let world = InvisibleWorld::new(position, pieces);
            worlds.entry(world.key()).or_insert(world);
            if worlds.len() > MAX_WORLDS {
                bail!("候補世界が上限の{MAX_WORLDS}件を超えました");
            }
        }
        return Ok(());
    }

    let spec = specs[index];
    for raw in 0..=81 {
        let location = if raw == 81 {
            if spec.kind.index() >= fmrs_core::piece::NUM_HAND_KIND {
                continue;
            }
            InvisibleLocation::Hand(spec.color)
        } else {
            let square = Square::from_index(raw);
            if base.get(square).is_some()
                || pieces
                    .iter()
                    .any(|piece| piece.location == InvisibleLocation::Board(square))
            {
                continue;
            }
            InvisibleLocation::Board(square)
        };
        if index > 0 && specs[index - 1] == spec {
            if let Some(previous) = pieces.last() {
                if location < previous.location {
                    continue;
                }
            }
        }
        let mut next = pieces.clone();
        next.push(InvisiblePiece {
            color: spec.color,
            kind: spec.kind,
            location,
        });
        enumerate_locations(base, specs, index + 1, next, rule, worlds)?;
    }
    Ok(())
}

fn legal_initial_world(position: &PositionAux, rule: MateRule) -> bool {
    if position.is_illegal_initial_position() {
        return false;
    }
    let white_kings = position.bitboard(Color::WHITE, Kind::King).count_ones();
    let black_kings = position.bitboard(Color::BLACK, Kind::King).count_ones();
    if white_kings != 1
        || match rule {
            MateRule::Helpmate | MateRule::BestMate => black_kings > 1,
            MateRule::HelpSelfmate => black_kings != 1,
        }
    {
        return false;
    }
    let mut checked = position.clone();
    let black_checked = checked.checked_slow(Color::BLACK);
    let white_checked = checked.checked_slow(Color::WHITE);
    !((black_checked && white_checked)
        || (position.turn().is_black() && white_checked)
        || (position.turn().is_white() && black_checked))
}

impl KnownInvisibleState {
    fn from_worlds(worlds: Vec<InvisibleWorld>, free_white_move: bool, rule: MateRule) -> Self {
        let mut state = Self {
            worlds,
            free_white_move,
            rule,
        };
        state.reveal_resolved();
        let mut unique = BTreeMap::new();
        for world in std::mem::take(&mut state.worlds) {
            unique.entry(world.key()).or_insert(world);
        }
        state.worlds = unique.into_values().collect();
        state
    }

    fn reveal_resolved(&mut self) {
        loop {
            let Some(first) = self.worlds.first() else {
                return;
            };
            let mut common = first.invisibles.clone();
            for world in self.worlds.iter().skip(1) {
                let mut available = world.invisibles.clone();
                common.retain(|piece| {
                    if let Some(index) = available.iter().position(|candidate| candidate == piece) {
                        available.remove(index);
                        true
                    } else {
                        false
                    }
                });
            }
            let Some(resolved) = common.first().copied() else {
                break;
            };
            for world in &mut self.worlds {
                let index = world
                    .invisibles
                    .iter()
                    .position(|piece| *piece == resolved)
                    .expect("全世界に共通する透明駒がある");
                world.invisibles.remove(index);
            }
        }
    }

    pub fn world_count(&self) -> usize {
        self.worlds.len()
    }

    pub fn turn(&self) -> Color {
        self.worlds[0].position.turn()
    }

    fn successors(&self) -> Result<BTreeMap<KnownInvisibleObservedMove, Vec<InvisibleWorld>>> {
        let mut grouped =
            BTreeMap::<KnownInvisibleObservedMove, BTreeMap<String, InvisibleWorld>>::new();
        for world in &self.worlds {
            for movement in concrete_movements(world, self.free_white_move) {
                for (observed, next) in world.transition_variants(movement) {
                    grouped
                        .entry(observed)
                        .or_default()
                        .entry(next.key())
                        .or_insert(next);
                }
            }
        }
        grouped
            .into_iter()
            .map(|(observed, worlds)| {
                if worlds.len() > MAX_WORLDS {
                    bail!("着手後の候補世界が上限の{MAX_WORLDS}件を超えました");
                }
                Ok((observed, worlds.into_values().collect()))
            })
            .collect()
    }

    fn is_proven_mate(&self) -> bool {
        if self.turn() != self.rule.terminal_turn() {
            return false;
        }
        self.worlds.iter().all(|world| {
            let mut position = world.position.clone();
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
}

fn concrete_movements(world: &InvisibleWorld, free_white_move: bool) -> Vec<Movement> {
    let mut position = world.position.clone();
    let mut movements = Vec::new();
    if free_white_move {
        legal_movements(&position, &mut movements);
    } else if advance_aux(&mut position, &AdvanceOptions::default(), &mut movements).is_err() {
        return Vec::new();
    }
    movements
        .into_iter()
        .filter(|movement| {
            if !movement.is_pawn_drop() {
                return true;
            }
            let mut next = world.position.clone();
            next.do_move(movement);
            !is_legal_mate(&mut next)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum KnownInvisibleObservedMove {
    Known(ObservedMove),
    Invisible,
    InvisibleCapture(#[serde(with = "square_serde")] Square),
}

pub type KnownInvisibleSolution = Vec<KnownInvisibleObservedMove>;

pub fn solve_known_invisible_exact(
    initial: &KnownInvisibleState,
    plies: usize,
    max_solutions: usize,
) -> Result<Vec<KnownInvisibleSolution>> {
    let mut solutions = Vec::new();
    for depth in 0..=plies {
        let turn = if depth % 2 == 0 {
            initial.turn()
        } else {
            initial.turn().opposite()
        };
        if turn != initial.rule.terminal_turn() {
            continue;
        }
        solve_inner(
            initial,
            depth,
            max_solutions,
            &mut Vec::new(),
            &mut solutions,
        )?;
        if solutions.len() >= max_solutions {
            break;
        }
    }
    Ok(solutions)
}

fn solve_inner(
    state: &KnownInvisibleState,
    remaining: usize,
    max_solutions: usize,
    path: &mut KnownInvisibleSolution,
    solutions: &mut Vec<KnownInvisibleSolution>,
) -> Result<()> {
    if solutions.len() >= max_solutions {
        return Ok(());
    }
    if remaining == 0 {
        if state.is_proven_mate() && !solutions.contains(path) {
            solutions.push(path.clone());
        }
        return Ok(());
    }
    if state.is_proven_mate() {
        return Ok(());
    }
    for (observed, worlds) in state.successors()? {
        let next = KnownInvisibleState::from_worlds(worlds, false, state.rule);
        path.push(observed);
        solve_inner(&next, remaining - 1, max_solutions, path, solutions)?;
        path.pop();
        if solutions.len() >= max_solutions {
            break;
        }
    }
    Ok(())
}

pub fn format_known_invisible_solution_japanese(
    initial: &KnownInvisibleState,
    solution: &KnownInvisibleSolution,
) -> Result<Vec<String>> {
    let mut state = initial.clone();
    let mut result = Vec::new();
    let mut previous_destination = None;
    for observed in solution {
        result.push(match observed {
            KnownInvisibleObservedMove::Invisible => {
                previous_destination = None;
                "X".to_string()
            }
            KnownInvisibleObservedMove::InvisibleCapture(square) => {
                let label = destination_label(*square, previous_destination);
                previous_destination = Some(*square);
                format!("{label}X")
            }
            KnownInvisibleObservedMove::Known(move_) => {
                let formatted = format_known(&state, *move_, previous_destination);
                previous_destination = Some(match move_ {
                    ObservedMove::Move { destination, .. }
                    | ObservedMove::Drop { destination, .. } => *destination,
                });
                formatted
            }
        });
        let successors = state.successors()?;
        let worlds = successors
            .get(observed)
            .cloned()
            .context("解手順を候補世界へ適用できません")?;
        state = KnownInvisibleState::from_worlds(worlds, false, state.rule);
    }
    Ok(result)
}

fn format_known(
    state: &KnownInvisibleState,
    observed: ObservedMove,
    previous_destination: Option<Square>,
) -> String {
    match observed {
        ObservedMove::Move {
            source,
            destination,
            promote,
            ..
        } => {
            let (color, kind) = state.worlds[0].position.get(source).expect("可視駒がある");
            format!(
                "{}{}{}({})",
                destination_label(destination, previous_destination),
                japanese_kind(kind),
                movement_suffix(promote, kind.can_promote(), color, source, destination),
                square_label(source)
            )
        }
        ObservedMove::Drop {
            identity: DropIdentity::Known(kind),
            destination,
        } => {
            format!(
                "{}{}打",
                destination_label(destination, previous_destination),
                japanese_kind(kind)
            )
        }
        _ => unreachable!("駒種指定透明駒では既知着手だけを渡す"),
    }
}

fn destination_label(destination: Square, previous_destination: Option<Square>) -> String {
    if previous_destination == Some(destination) {
        "同".to_string()
    } else {
        square_label(destination)
    }
}

fn movement_suffix(
    promote: bool,
    can_promote: bool,
    color: Color,
    source: Square,
    destination: Square,
) -> &'static str {
    if promote {
        "成"
    } else if can_promote
        && (in_promotion_zone(source, color) || in_promotion_zone(destination, color))
    {
        "生"
    } else {
        ""
    }
}

fn in_promotion_zone(square: Square, color: Color) -> bool {
    if color.is_black() {
        square.row() < 3
    } else {
        square.row() >= 6
    }
}

fn square_label(square: Square) -> String {
    format!("{}{}", square.col() + 1, square.row() + 1)
}

fn japanese_kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Pawn => "歩",
        Kind::Lance => "香",
        Kind::Knight => "桂",
        Kind::Silver => "銀",
        Kind::Gold => "金",
        Kind::Bishop => "角",
        Kind::Rook => "飛",
        Kind::King => "玉",
        Kind::ProPawn => "と",
        Kind::ProLance => "杏",
        Kind::ProKnight => "圭",
        Kind::ProSilver => "全",
        Kind::ProBishop => "馬",
        Kind::ProRook => "龍",
    }
}

mod square_serde {
    use fmrs_core::position::Square;
    use serde::Serializer;
    pub fn serialize<S: Serializer>(square: &Square, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}{}", square.col() + 1, square.row() + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_more_than_two() {
        let document = KnownInvisibleDocument::from_json(
            r#"{
            "baseSfen":"9/9/k8/9/9/9/9/9/9 b - 1", "plies":1,
            "invisibles":[{"color":"black","kind":"R","count":3}]
        }"#,
        )
        .unwrap();
        assert!(document
            .into_problem()
            .unwrap_err()
            .to_string()
            .contains("合計2枚"));
    }

    #[test]
    fn one_known_kind_enumerates_locations() {
        let (problem, _) = KnownInvisibleDocument::from_json(
            r#"{
            "baseSfen":"9/9/k8/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1", "plies":1,
            "invisibles":[{"color":"black","kind":"K","count":1}]
        }"#,
        )
        .unwrap()
        .into_problem()
        .unwrap();
        let state = problem.enumerate().unwrap();
        assert!(state.world_count() > 1);
    }

    #[test]
    fn invisible_is_allocated_from_full_white_hand() {
        let (problem, _) = KnownInvisibleDocument::from_json(
            r#"{
            "baseSfen":"4k4/9/9/9/9/9/9/9/4K4 b 2r2b4g4s4n4l18p 1", "plies":1,
            "invisibles":[{"color":"black","kind":"L","count":1}]
        }"#,
        )
        .unwrap()
        .into_problem()
        .unwrap();
        let state = problem.enumerate().unwrap();
        assert!(state.world_count() > 1);
        assert!(state.worlds.iter().all(|world| world
            .position
            .hands()
            .count(Color::WHITE, Kind::Lance)
            == 3));
    }

    #[test]
    fn invisible_non_capture_is_x_and_capture_has_square() {
        let world = InvisibleWorld::new(
            PositionAux::from_sfen("8k/9/9/9/4R4/4p4/9/9/8K b - 1").unwrap(),
            vec![InvisiblePiece {
                color: Color::BLACK,
                kind: Kind::Rook,
                location: InvisibleLocation::Board(Square::S55),
            }],
        );
        let quiet = Movement::Move {
            source: Square::S55,
            source_kind_hint: None,
            dest: Square::S54,
            promote: false,
            capture_kind_hint: None,
        };
        assert_eq!(
            world.transition_variants(quiet)[0].0,
            KnownInvisibleObservedMove::Invisible
        );
        let capture = Movement::Move {
            source: Square::S55,
            source_kind_hint: None,
            dest: Square::S56,
            promote: false,
            capture_kind_hint: None,
        };
        assert_eq!(
            world.transition_variants(capture)[0].0,
            KnownInvisibleObservedMove::InvisibleCapture(Square::S56)
        );
    }

    #[test]
    fn captured_invisible_demotes_changes_owner_and_stays_invisible() {
        let world = InvisibleWorld::new(
            PositionAux::from_sfen("8k/9/9/9/4+rS3/9/9/9/8K b - 1").unwrap(),
            vec![InvisiblePiece {
                color: Color::WHITE,
                kind: Kind::ProRook,
                location: InvisibleLocation::Board(Square::S55),
            }],
        );
        let movement = Movement::Move {
            source: Square::S45,
            source_kind_hint: None,
            dest: Square::S55,
            promote: false,
            capture_kind_hint: None,
        };
        let (_, next) = &world.transition_variants(movement)[0];
        assert_eq!(
            next.invisibles,
            vec![InvisiblePiece {
                color: Color::BLACK,
                kind: Kind::Rook,
                location: InvisibleLocation::Hand(Color::BLACK),
            }]
        );
    }

    #[test]
    fn reveals_only_location_common_to_every_world() {
        let base = PositionAux::from_sfen("8k/9/9/9/4R4/9/9/9/8K b - 1").unwrap();
        let fixed = InvisiblePiece {
            color: Color::BLACK,
            kind: Kind::Rook,
            location: InvisibleLocation::Board(Square::S55),
        };
        let varying = |square| InvisiblePiece {
            color: Color::WHITE,
            kind: Kind::Silver,
            location: InvisibleLocation::Board(square),
        };
        let mut first = base.clone();
        first.set(Square::S44, Color::WHITE, Kind::Silver);
        let mut second = base;
        second.set(Square::S43, Color::WHITE, Kind::Silver);
        let state = KnownInvisibleState::from_worlds(
            vec![
                InvisibleWorld::new(first, vec![fixed, varying(Square::S44)]),
                InvisibleWorld::new(second, vec![fixed, varying(Square::S43)]),
            ],
            false,
            MateRule::Helpmate,
        );
        assert!(state.worlds.iter().all(|world| world.invisibles.len() == 1));
        assert!(state
            .worlds
            .iter()
            .all(|world| !world.invisibles.contains(&fixed)));
    }
}
