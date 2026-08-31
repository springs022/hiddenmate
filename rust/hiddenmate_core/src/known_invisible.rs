use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
    time::Instant,
};

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

use crate::{DropIdentity, EnumerationMetrics, MateRule, MoveIdentity, ObservedMove, SolveMetrics};

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

/// 盤上81マスと両者の駒台を1つのビット集合で表す。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct InvisibleLocationSet(u128);

impl InvisibleLocationSet {
    fn insert(&mut self, location: InvisibleLocation) {
        self.0 |= 1u128 << location_index(location);
    }

    fn iter(self) -> impl Iterator<Item = InvisibleLocation> {
        (0..83).filter_map(move |index| {
            if self.0 & (1u128 << index) == 0 {
                return None;
            }
            Some(match index {
                0..=80 => InvisibleLocation::Board(Square::from_index(index)),
                81 => InvisibleLocation::Hand(Color::BLACK),
                82 => InvisibleLocation::Hand(Color::WHITE),
                _ => unreachable!("位置ビットは83個"),
            })
        })
    }
}

fn location_index(location: InvisibleLocation) -> usize {
    match location {
        InvisibleLocation::Board(square) => square.index(),
        InvisibleLocation::Hand(color) if color.is_black() => 81,
        InvisibleLocation::Hand(_) => 82,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InvisiblePiece {
    color: Color,
    kind: Kind,
    location: InvisibleLocation,
}

#[derive(Clone, Debug)]
struct InvisibleWorld {
    position: Arc<PositionAux>,
    invisibles: Vec<InvisiblePiece>,
}

impl InvisibleWorld {
    fn new(position: PositionAux, mut invisibles: Vec<InvisiblePiece>) -> Self {
        invisibles.sort_unstable();
        Self {
            position: Arc::new(position),
            invisibles,
        }
    }

    fn key(&self) -> String {
        format!(
            "{}|{:?}",
            sfen::encode_position(self.position.as_ref()),
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
        Arc::make_mut(&mut next.position).do_move(&movement);
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
        self.enumerate_explicit()
    }

    /// すべての具体世界を列挙する参照実装。
    ///
    /// 将来の共有・遅延バックエンドは、この結果との一致をテストする。
    pub fn enumerate_explicit(self) -> Result<KnownInvisibleState> {
        let mut worlds = BTreeMap::new();
        let free_white_move = for_each_initial_world(&self, &mut |world| {
            worlds.entry(world.key()).or_insert(world);
            if worlds.len() > MAX_WORLDS {
                bail!("候補世界が上限の{MAX_WORLDS}件を超えました");
            }
            Ok(())
        })?;
        if worlds.is_empty() {
            bail!("初形の合法性と矛盾しない透明駒の配置がありません");
        }
        Ok(KnownInvisibleState::from_worlds(
            worlds.into_values().collect(),
            free_white_move,
            self.rule,
        ))
    }

    pub fn enumerate_profiled(self) -> Result<(KnownInvisibleState, EnumerationMetrics)> {
        let started = Instant::now();
        let state = self.enumerate_explicit()?;
        let metrics = EnumerationMetrics {
            world_count: state.world_count(),
            elapsed: started.elapsed(),
        };
        Ok((state, metrics))
    }

    /// 初形問題と観測履歴だけを保持し、候補世界を操作時に再生する。
    pub fn enumerate_replay(self) -> Result<ReplayKnownInvisibleState> {
        ReplayKnownInvisibleState::new(self)
    }

    pub fn enumerate_replay_profiled(
        self,
    ) -> Result<(ReplayKnownInvisibleState, EnumerationMetrics)> {
        let started = Instant::now();
        let state = ReplayKnownInvisibleState::new(self)?;
        let metrics = EnumerationMetrics {
            world_count: state.world_count(),
            elapsed: started.elapsed(),
        };
        Ok((state, metrics))
    }
}

fn for_each_initial_world(
    problem: &KnownInvisibleProblem,
    visitor: &mut impl FnMut(InvisibleWorld) -> Result<()>,
) -> Result<bool> {
    let base = PositionAux::from_sfen(&problem.base_sfen)
        .with_context(|| format!("SFENを解釈できません: {}", problem.base_sfen))?;
    let mut specs = problem.invisibles.clone();
    specs.sort_unstable();
    let location_domains = specs
        .iter()
        .map(|spec| initial_location_domain(&base, *spec))
        .collect::<Vec<_>>();
    for allocated_base in allocate_invisible_inventory(&base, &specs)? {
        generate_locations(
            &allocated_base,
            &specs,
            &location_domains,
            0,
            Vec::new(),
            problem.rule,
            visitor,
        )?;
    }
    Ok(base.turn().is_white())
}

fn initial_location_domain(base: &PositionAux, spec: KnownInvisibleSpec) -> InvisibleLocationSet {
    let mut result = InvisibleLocationSet::default();
    for raw in 0..81 {
        let square = Square::from_index(raw);
        if base.get(square).is_none() {
            result.insert(InvisibleLocation::Board(square));
        }
    }
    if spec.kind.index() < fmrs_core::piece::NUM_HAND_KIND {
        result.insert(InvisibleLocation::Hand(spec.color));
    }
    result
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

fn generate_locations(
    base: &PositionAux,
    specs: &[KnownInvisibleSpec],
    location_domains: &[InvisibleLocationSet],
    index: usize,
    pieces: Vec<InvisiblePiece>,
    rule: MateRule,
    visitor: &mut impl FnMut(InvisibleWorld) -> Result<()>,
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
            visitor(InvisibleWorld::new(position, pieces))?;
        }
        return Ok(());
    }

    let spec = specs[index];
    for location in location_domains[index].iter() {
        if let InvisibleLocation::Board(square) = location {
            if pieces
                .iter()
                .any(|piece| piece.location == InvisibleLocation::Board(square))
            {
                continue;
            }
        }
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
        generate_locations(
            base,
            specs,
            location_domains,
            index + 1,
            next,
            rule,
            visitor,
        )?;
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
            MateRule::Helpmate => black_kings > 1,
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
            let mut position = world.position.as_ref().clone();
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
}

/// 候補世界を保持せず、初形生成器と観測履歴から必要時に再生する状態。
#[derive(Clone, Debug)]
pub struct ReplayKnownInvisibleState {
    problem: Arc<KnownInvisibleProblem>,
    history: Vec<KnownInvisibleObservedMove>,
    /// 初形、および各観測着手後に可視化された透明駒。
    resolved_steps: Vec<Vec<InvisiblePiece>>,
    world_count: usize,
    initial_turn: Color,
    free_white_move: bool,
    rule: MateRule,
    observed_moves_cache: Arc<OnceLock<Vec<KnownInvisibleObservedMove>>>,
    mate_cache: Arc<OnceLock<bool>>,
}

impl ReplayKnownInvisibleState {
    fn new(problem: KnownInvisibleProblem) -> Result<Self> {
        let mut common = None;
        let mut generated = 0usize;
        let free_white_move = for_each_initial_world(&problem, &mut |world| {
            generated += 1;
            retain_common_invisibles(&mut common, &world.invisibles);
            Ok(())
        })?;
        if generated == 0 {
            bail!("初形の合法性と矛盾しない透明駒の配置がありません");
        }
        let resolved = common.unwrap_or_default();
        let mut keys = BTreeSet::new();
        for_each_initial_world(&problem, &mut |mut world| {
            remove_resolved_invisibles(&mut world, &resolved);
            keys.insert(world.key());
            if keys.len() > MAX_WORLDS {
                bail!("候補世界が上限の{MAX_WORLDS}件を超えました");
            }
            Ok(())
        })?;
        let initial_turn = PositionAux::from_sfen(&problem.base_sfen)
            .with_context(|| format!("SFENを解釈できません: {}", problem.base_sfen))?
            .turn();
        Ok(Self {
            problem: Arc::new(problem.clone()),
            history: Vec::new(),
            resolved_steps: vec![resolved],
            world_count: keys.len(),
            initial_turn,
            free_white_move,
            rule: problem.rule,
            observed_moves_cache: Arc::new(OnceLock::new()),
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

    pub fn observed_moves(&self) -> Result<Vec<KnownInvisibleObservedMove>> {
        if let Some(cached) = self.observed_moves_cache.get() {
            return Ok(cached.clone());
        }
        let mut result = BTreeSet::new();
        let free_white_move = self.history.is_empty() && self.free_white_move;
        self.for_each_world(&mut |world| {
            for movement in concrete_movements(&world, free_white_move) {
                for (observed, _) in world.transition_variants(movement) {
                    result.insert(observed);
                }
            }
            Ok(())
        })?;
        let result = result.into_iter().collect::<Vec<_>>();
        let _ = self.observed_moves_cache.set(result.clone());
        Ok(result)
    }

    pub fn apply(&self, observed: KnownInvisibleObservedMove) -> Result<Option<Self>> {
        let mut next_worlds = BTreeMap::new();
        let free_white_move = self.history.is_empty() && self.free_white_move;
        self.for_each_world(&mut |world| {
            for movement in concrete_movements(&world, free_white_move) {
                for (candidate, next) in world.transition_variants(movement) {
                    if candidate == observed {
                        next_worlds.entry(next.key()).or_insert(next);
                    }
                }
            }
            if next_worlds.len() > MAX_WORLDS {
                bail!("着手後の候補世界が上限の{MAX_WORLDS}件を超えました");
            }
            Ok(())
        })?;
        if next_worlds.is_empty() {
            return Ok(None);
        }

        let mut common = None;
        for world in next_worlds.values() {
            retain_common_invisibles(&mut common, &world.invisibles);
        }
        let resolved = common.unwrap_or_default();
        let mut normalized = BTreeMap::new();
        for (_, mut world) in next_worlds {
            remove_resolved_invisibles(&mut world, &resolved);
            normalized.entry(world.key()).or_insert(world);
        }

        let mut next = self.clone();
        next.history.push(observed);
        next.resolved_steps.push(resolved);
        next.world_count = normalized.len();
        next.observed_moves_cache = Arc::new(OnceLock::new());
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
        self.for_each_world(&mut |world| {
            let mut position = world.position.as_ref().clone();
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

    fn for_each_world(&self, visitor: &mut impl FnMut(InvisibleWorld) -> Result<()>) -> Result<()> {
        for_each_initial_world(&self.problem, &mut |mut initial| {
            remove_resolved_invisibles(&mut initial, &self.resolved_steps[0]);
            let mut frontier = vec![initial];
            for (step, observed) in self.history.iter().copied().enumerate() {
                let mut next_frontier = Vec::new();
                for world in frontier {
                    let free_white_move = step == 0 && self.free_white_move;
                    for movement in concrete_movements(&world, free_white_move) {
                        for (candidate, mut next) in world.transition_variants(movement) {
                            if candidate == observed {
                                remove_resolved_invisibles(
                                    &mut next,
                                    &self.resolved_steps[step + 1],
                                );
                                next_frontier.push(next);
                            }
                        }
                    }
                }
                frontier = next_frontier;
                if frontier.is_empty() {
                    break;
                }
            }
            for world in frontier {
                visitor(world)?;
            }
            Ok(())
        })?;
        Ok(())
    }
}

fn retain_common_invisibles(
    common: &mut Option<Vec<InvisiblePiece>>,
    candidates: &[InvisiblePiece],
) {
    let Some(current) = common.as_mut() else {
        *common = Some(candidates.to_vec());
        return;
    };
    let mut available = candidates.to_vec();
    current.retain(|piece| {
        if let Some(index) = available.iter().position(|candidate| candidate == piece) {
            available.remove(index);
            true
        } else {
            false
        }
    });
}

fn remove_resolved_invisibles(world: &mut InvisibleWorld, resolved: &[InvisiblePiece]) {
    for piece in resolved {
        if let Some(index) = world
            .invisibles
            .iter()
            .position(|candidate| candidate == piece)
        {
            world.invisibles.remove(index);
        }
    }
}

fn concrete_movements(world: &InvisibleWorld, free_white_move: bool) -> Vec<Movement> {
    let mut position = world.position.as_ref().clone();
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
            let mut next = world.position.as_ref().clone();
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
    solve_known_invisible_exact_profiled(initial, plies, max_solutions)
        .map(|(solutions, _)| solutions)
}

pub fn solve_known_invisible_exact_profiled(
    initial: &KnownInvisibleState,
    plies: usize,
    max_solutions: usize,
) -> Result<(Vec<KnownInvisibleSolution>, SolveMetrics)> {
    let started = Instant::now();
    let mut metrics = SolveMetrics::new(initial.world_count());
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
            &mut metrics,
        )?;
        if solutions.len() >= max_solutions {
            break;
        }
    }
    metrics.total_elapsed = started.elapsed();
    Ok((solutions, metrics))
}

/// 候補世界を保持しない再生型状態を使い、指定手数以下の解を短い順に列挙する。
pub fn solve_replay_known_invisible_exact(
    initial: &ReplayKnownInvisibleState,
    plies: usize,
    max_solutions: usize,
) -> Result<Vec<KnownInvisibleSolution>> {
    solve_replay_known_invisible_exact_profiled(initial, plies, max_solutions)
        .map(|(solutions, _)| solutions)
}

pub fn solve_replay_known_invisible_exact_profiled(
    initial: &ReplayKnownInvisibleState,
    plies: usize,
    max_solutions: usize,
) -> Result<(Vec<KnownInvisibleSolution>, SolveMetrics)> {
    let started = Instant::now();
    let mut metrics = SolveMetrics::new(initial.world_count());
    if max_solutions == 0 {
        metrics.total_elapsed = started.elapsed();
        return Ok((Vec::new(), metrics));
    }

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
        solve_replay_inner(
            initial,
            depth,
            max_solutions,
            &mut Vec::with_capacity(depth),
            &mut solutions,
            &mut metrics,
        )?;
        if solutions.len() >= max_solutions {
            break;
        }
    }
    metrics.total_elapsed = started.elapsed();
    Ok((solutions, metrics))
}

fn solve_replay_inner(
    state: &ReplayKnownInvisibleState,
    remaining: usize,
    max_solutions: usize,
    path: &mut KnownInvisibleSolution,
    solutions: &mut Vec<KnownInvisibleSolution>,
    metrics: &mut SolveMetrics,
) -> Result<()> {
    metrics.visit_state(state.world_count());
    if solutions.len() >= max_solutions {
        return Ok(());
    }
    if remaining == 0 {
        if state.is_proven_mate()? && !solutions.contains(path) {
            solutions.push(path.clone());
        }
        return Ok(());
    }
    if state.is_proven_mate()? {
        return Ok(());
    }

    let move_generation_started = Instant::now();
    let observed_moves = state.observed_moves()?;
    metrics.move_generation_elapsed += move_generation_started.elapsed();
    for observed in observed_moves {
        let transition_started = Instant::now();
        let Some(next) = state.apply(observed)? else {
            continue;
        };
        metrics.transition_elapsed += transition_started.elapsed();
        metrics.record_successor(next.world_count());
        path.push(observed);
        solve_replay_inner(
            &next,
            remaining - 1,
            max_solutions,
            path,
            solutions,
            metrics,
        )?;
        path.pop();
        if solutions.len() >= max_solutions {
            break;
        }
    }
    Ok(())
}

fn solve_inner(
    state: &KnownInvisibleState,
    remaining: usize,
    max_solutions: usize,
    path: &mut KnownInvisibleSolution,
    solutions: &mut Vec<KnownInvisibleSolution>,
    metrics: &mut SolveMetrics,
) -> Result<()> {
    metrics.visit_state(state.world_count());
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
    let transition_started = Instant::now();
    let successors = state.successors()?;
    let transition_elapsed = transition_started.elapsed();
    metrics.move_generation_elapsed += transition_elapsed;
    metrics.transition_elapsed += transition_elapsed;
    for (observed, worlds) in successors {
        metrics.record_successor(worlds.len());
        let next = KnownInvisibleState::from_worlds(worlds, false, state.rule);
        path.push(observed);
        solve_inner(
            &next,
            remaining - 1,
            max_solutions,
            path,
            solutions,
            metrics,
        )?;
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

    #[test]
    fn default_and_explicit_enumeration_match_and_metrics_are_reported() {
        let problem = KnownInvisibleProblem {
            base_sfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1".to_string(),
            invisibles: vec![KnownInvisibleSpec {
                color: Color::BLACK,
                kind: Kind::Lance,
            }],
            rule: MateRule::Helpmate,
        };

        let default = problem.clone().enumerate().unwrap();
        let explicit = problem.clone().enumerate_explicit().unwrap();
        let (profiled, enumeration) = problem.enumerate_profiled().unwrap();

        assert_eq!(default.world_count(), 71);
        assert_eq!(default.world_count(), explicit.world_count());
        assert_eq!(
            default
                .worlds
                .iter()
                .map(InvisibleWorld::key)
                .collect::<Vec<_>>(),
            explicit
                .worlds
                .iter()
                .map(InvisibleWorld::key)
                .collect::<Vec<_>>()
        );
        assert_eq!(enumeration.world_count, profiled.world_count());

        let expected = solve_known_invisible_exact(&default, 5, 1).unwrap();
        let (actual, metrics) = solve_known_invisible_exact_profiled(&profiled, 5, 1).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(metrics.initial_world_count, 71);
        assert!(metrics.visited_state_count > 0);
        assert!(metrics.generated_transition_count > 0);
    }

    #[test]
    fn replay_backend_matches_explicit_backend() {
        let problem = KnownInvisibleProblem {
            base_sfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1".to_string(),
            invisibles: vec![KnownInvisibleSpec {
                color: Color::BLACK,
                kind: Kind::Lance,
            }],
            rule: MateRule::Helpmate,
        };
        let explicit = problem.clone().enumerate_explicit().unwrap();
        let (replay, enumeration) = problem.enumerate_replay_profiled().unwrap();

        assert_eq!(replay.world_count(), explicit.world_count());
        assert_eq!(enumeration.world_count, explicit.world_count());
        assert_eq!(replay.turn(), explicit.turn());
        assert_eq!(
            replay
                .observed_moves()
                .unwrap()
                .into_iter()
                .collect::<BTreeSet<_>>(),
            explicit.successors().unwrap().into_keys().collect()
        );

        let expected = solve_known_invisible_exact(&explicit, 5, 2).unwrap();
        let (actual, metrics) = solve_replay_known_invisible_exact_profiled(&replay, 5, 2).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(metrics.initial_world_count, explicit.world_count());
        assert!(metrics.visited_state_count > 0);
    }

    #[test]
    fn replay_apply_matches_explicit_successor_counts() {
        let problem = KnownInvisibleProblem {
            base_sfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1".to_string(),
            invisibles: vec![KnownInvisibleSpec {
                color: Color::BLACK,
                kind: Kind::Lance,
            }],
            rule: MateRule::Helpmate,
        };
        let explicit = problem.clone().enumerate_explicit().unwrap();
        let replay = problem.enumerate_replay().unwrap();

        for (observed, worlds) in explicit.successors().unwrap() {
            let next = replay.apply(observed).unwrap().unwrap();
            let normalized = KnownInvisibleState::from_worlds(worlds, false, MateRule::Helpmate);
            assert_eq!(next.world_count(), normalized.world_count(), "{observed:?}");
            assert_eq!(next.turn(), normalized.turn(), "{observed:?}");
            assert_eq!(next.is_proven_mate().unwrap(), normalized.is_proven_mate());
        }
    }

    #[test]
    fn replay_backend_matches_free_white_help_selfmate() {
        let problem = KnownInvisibleProblem {
            base_sfen: "9/9/9/9/7l1/9/8k/9/7SK w G 1".to_string(),
            invisibles: vec![],
            rule: MateRule::HelpSelfmate,
        };
        let explicit = problem.clone().enumerate_explicit().unwrap();
        let replay = problem.enumerate_replay().unwrap();

        assert_eq!(
            replay.observed_moves().unwrap().len(),
            explicit.successors().unwrap().len()
        );
        assert_eq!(
            solve_replay_known_invisible_exact(&replay, 3, 100).unwrap(),
            solve_known_invisible_exact(&explicit, 3, 100).unwrap()
        );
    }
}
