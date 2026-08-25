//! 通常駒の協力自玉詰（協力して攻方玉を詰める）の順算ソルバー。
//!
//! 通常協力詰と同様、攻方（黒）は受方玉への王手を続け、受方（白）は
//! 王手を外す。通常協力詰との違いは、白の応手によって黒玉が通常の意味で
//! 詰んだ局面をゴールとする点にある。

use std::{collections::VecDeque, rc::Rc};

use anyhow::bail;
use log::info;

use crate::{
    memo::{Memo, MemoTrait},
    nohash::{NoHashMap64, NoHashSet64},
    piece::{Color, Kind},
    position::{
        advance::advance::advance_aux,
        advance::{is_legal_mate, AdvanceOptions},
        position::{CachedPosition, PositionAux},
        previous, previous_with_digest, BitBoard, Movement,
    },
};

use super::Solution;

/// 協力自玉詰の終局局面かを返す。
///
/// 終局は白の着手直後（黒番）に限る。白の歩打ちによる詰みは打歩詰めなので
/// 解として認めない。
pub fn is_help_selfmate(position: &mut PositionAux) -> bool {
    position.turn().is_black() && !position.pawn_drop() && is_legal_mate(position)
}

pub fn help_selfmate_solve(
    position: PositionAux,
    solutions_upto: usize,
    silent: bool,
) -> anyhow::Result<HelpSelfmateReconstructor> {
    let mut solver = HelpSelfmateSolver::new(position, solutions_upto, silent)?;
    loop {
        match solver.advance()? {
            HelpSelfmateSolverStatus::Intermediate(_) => {}
            HelpSelfmateSolverStatus::Mate(reconstructor) => return Ok(reconstructor),
            HelpSelfmateSolverStatus::NoSolution => {
                return Ok(HelpSelfmateReconstructor::no_solution())
            }
        }
    }
}

/// `max_plies` 手以下に存在する協力自玉詰の手順を、短い順にすべて列挙する。
///
/// 最短解用の幅優先探索と異なり、同じ局面へ異なる手数で到達する経路も保持する。
/// 指定手数の作品検討では、そのような迂回手順も別解として必要になるためである。
pub fn help_selfmate_solutions_within(
    mut position: PositionAux,
    max_plies: usize,
    solutions_upto: usize,
) -> anyhow::Result<Vec<Solution>> {
    validate_initial_position(&mut position)?;
    if max_plies == 0 || solutions_upto == 0 {
        return Ok(Vec::new());
    }
    if position.turn().is_white() && !position.checked_slow(Color::WHITE) {
        bail!("White-to-move initial position is not checked");
    }

    let mut context = BoundedSearch {
        target_plies: 0,
        solutions_upto,
        path: Vec::with_capacity(max_plies),
        solutions: Vec::new(),
    };
    for target_plies in 1..=max_plies {
        if context.solutions.len() >= solutions_upto {
            break;
        }
        context.target_plies = target_plies;
        context.search(&mut position)?;
    }
    Ok(context.solutions)
}

struct BoundedSearch {
    target_plies: usize,
    solutions_upto: usize,
    path: Solution,
    solutions: Vec<Solution>,
}

impl BoundedSearch {
    fn search(&mut self, position: &mut PositionAux) -> anyhow::Result<()> {
        if self.path.len() >= self.target_plies || self.solutions.len() >= self.solutions_upto {
            return Ok(());
        }

        let mut movements = Vec::new();
        let wrong_king_mated = advance_aux(position, &AdvanceOptions::default(), &mut movements)?;
        if wrong_king_mated {
            return Ok(());
        }

        for movement in movements {
            if self.solutions.len() >= self.solutions_upto {
                break;
            }

            let mut next = position.clone();
            next.do_move(&movement);
            self.path.push(movement);

            if is_help_selfmate(&mut next) {
                if self.path.len() == self.target_plies {
                    self.solutions.push(self.path.clone());
                }
            } else if self.path.len() < self.target_plies {
                self.search(&mut next)?;
            }
            self.path.pop();
        }
        Ok(())
    }
}

pub enum HelpSelfmateSolverStatus {
    Intermediate(u32),
    Mate(HelpSelfmateReconstructor),
    NoSolution,
}

/// 最短手数を幅優先で探索する協力自玉詰ソルバー。
///
/// frontier は白番局面だけを保持し、白の応手と次の黒の王手をまとめて展開する。
/// これは通常協力詰の `LowMemStandardSolver` と同じ構成である。
pub struct HelpSelfmateSolver {
    initial_position_digests: NoHashSet64,
    solutions_upto: usize,
    /// 現在の白番 frontier の初形からの距離。
    step: u16,
    positions: Vec<CachedPosition>,
    next_positions: Vec<CachedPosition>,
    movements: Vec<Movement>,
    checking_movements: Vec<Movement>,
    mate_positions: Vec<PositionAux>,
    mate_position_digests: NoHashSet64,
    memo_white_turn: Memo,
    stone: Option<BitBoard>,
    silent: bool,
}

impl HelpSelfmateSolver {
    pub fn new(
        mut position: PositionAux,
        solutions_upto: usize,
        silent: bool,
    ) -> anyhow::Result<Self> {
        validate_initial_position(&mut position)?;

        let initial_position_digests = std::iter::once(position.digest()).collect();
        let stone = *position.stone();
        let mut memo_white_turn = Memo::with_capacity(4096);
        let mut positions = Vec::new();
        let mut step = 0;

        if position.turn().is_black() {
            let mut movements = Vec::new();
            advance_aux(&mut position, &AdvanceOptions::default(), &mut movements)?;
            for movement in movements {
                let digest = position.moved_digest(&movement);
                if memo_white_turn.contains_or_insert(digest, 1) {
                    continue;
                }
                positions.push(CachedPosition::from_aux(&position).after_movement(&movement));
            }
            step = 1;
        } else {
            // 白番開始（受先）では初形自身が王手を受けている必要がある。
            if !position.checked_slow(Color::WHITE) {
                bail!("White-to-move initial position is not checked");
            }
            memo_white_turn.contains_or_insert(position.digest(), 0);
            positions.push(CachedPosition::from_aux(&position));
        }

        Ok(Self {
            initial_position_digests,
            solutions_upto,
            step,
            positions,
            next_positions: Vec::with_capacity(1024),
            movements: Vec::with_capacity(256),
            checking_movements: Vec::with_capacity(256),
            mate_positions: Vec::new(),
            mate_position_digests: NoHashSet64::default(),
            memo_white_turn,
            stone,
            silent,
        })
    }

    pub fn advance(&mut self) -> anyhow::Result<HelpSelfmateSolverStatus> {
        if self.positions.is_empty() {
            return Ok(HelpSelfmateSolverStatus::NoSolution);
        }

        self.expand_white_frontier()?;

        if !self.mate_positions.is_empty() {
            let mate_in = self.step + 1;
            if !self.silent {
                info!(
                    "Found {} help-selfmates in {} moves searching {} positions",
                    self.mate_positions.len(),
                    mate_in,
                    self.memo_white_turn.len()
                );
            }
            return Ok(HelpSelfmateSolverStatus::Mate(
                HelpSelfmateReconstructor::new(
                    std::mem::take(&mut self.initial_position_digests),
                    std::mem::take(&mut self.mate_positions),
                    std::mem::take(&mut self.memo_white_turn),
                    mate_in,
                    self.solutions_upto,
                ),
            ));
        }

        std::mem::swap(&mut self.positions, &mut self.next_positions);
        self.step += 2;
        Ok(HelpSelfmateSolverStatus::Intermediate(self.step as u32))
    }

    fn expand_white_frontier(&mut self) -> anyhow::Result<()> {
        self.next_positions.clear();

        for cached in self.positions.iter() {
            let mut white_position = cached.to_aux(self.stone);
            self.movements.clear();
            let wrong_king_mated = advance_aux(
                &mut white_position,
                &AdvanceOptions::default(),
                &mut self.movements,
            )?;
            if wrong_king_mated {
                continue;
            }

            for white_movement in self.movements.iter() {
                let mut black_position = white_position.clone();
                black_position.do_move(white_movement);

                if is_help_selfmate(&mut black_position) {
                    if self.mate_position_digests.insert(black_position.digest()) {
                        self.mate_positions.push(black_position);
                    }
                    continue;
                }

                // 黒玉への逆王手が詰みでなければ、黒は王手を外しながら白玉へ
                // 王手を返す必要がある。この条件は既存の黒着手生成が処理する。
                self.checking_movements.clear();
                advance_aux(
                    &mut black_position,
                    &AdvanceOptions::default(),
                    &mut self.checking_movements,
                )?;

                for black_movement in self.checking_movements.iter() {
                    let digest = black_position.moved_digest(black_movement);
                    if self
                        .memo_white_turn
                        .contains_or_insert(digest, self.step + 2)
                    {
                        continue;
                    }
                    self.next_positions.push(
                        CachedPosition::from_aux(&black_position).after_movement(black_movement),
                    );
                }
            }
        }
        Ok(())
    }
}

fn validate_initial_position(position: &mut PositionAux) -> anyhow::Result<()> {
    if position.bitboard(Color::BLACK, Kind::King).count_ones() != 1 {
        bail!("Help-selfmate requires exactly one black king");
    }
    if position.bitboard(Color::WHITE, Kind::King).count_ones() != 1 {
        bail!("Help-selfmate requires exactly one white king");
    }
    if position.is_illegal_initial_position() {
        bail!("Illegal initial position");
    }
    if position.checked_slow(position.turn().opposite()) {
        bail!("The non-moving side is already checked");
    }
    Ok(())
}

pub struct HelpSelfmateReconstructor {
    initial_position_digests: NoHashSet64,
    mates: Vec<PositionAux>,
    memo_white_turn: Memo,
    mate_in: u16,
    solutions_upto: usize,
}

impl HelpSelfmateReconstructor {
    fn new(
        initial_position_digests: NoHashSet64,
        mates: Vec<PositionAux>,
        memo_white_turn: Memo,
        mate_in: u16,
        solutions_upto: usize,
    ) -> Self {
        Self {
            initial_position_digests,
            mates,
            memo_white_turn,
            mate_in,
            solutions_upto,
        }
    }

    fn no_solution() -> Self {
        Self::new(Default::default(), Vec::new(), Memo::default(), 0, 0)
    }

    pub fn mate_in(&self) -> Option<u16> {
        (!self.mates.is_empty()).then_some(self.mate_in)
    }

    pub fn is_empty(&self) -> bool {
        self.mates.is_empty()
    }

    pub fn cached_positions(&self) -> usize {
        self.memo_white_turn.len()
    }

    pub fn solutions(&self) -> Vec<Solution> {
        if self.solutions_upto == 0 {
            return Vec::new();
        }

        let mut solutions = Vec::new();
        for mate in self.mates.iter() {
            if solutions.len() >= self.solutions_upto {
                break;
            }
            self.reconstruct_from(mate, &mut solutions);
        }
        solutions
    }

    fn reconstruct_from(&self, mate: &PositionAux, solutions: &mut Vec<Solution>) {
        let mut queue = VecDeque::new();
        queue.push_back((mate.clone(), self.mate_in, MovementList::nil()));
        let mut visit_count = NoHashMap64::default();

        while let Some((mut black_position, step, following)) = queue.pop_front() {
            if solutions.len() >= self.solutions_upto {
                break;
            }
            debug_assert!(black_position.turn().is_black());
            debug_assert!(step > 0);

            let count = visit_count.entry(black_position.digest()).or_insert(0);
            if *count >= self.solutions_upto as u64 {
                continue;
            }
            *count += 1;

            let mut white_unmoves = Vec::new();
            previous_with_digest(
                &mut black_position,
                step < self.mate_in,
                |unmove, digest| {
                    if self.memo_white_turn.get(&digest) == Some(step - 1) {
                        white_unmoves.push(unmove);
                    }
                },
            );

            for white_unmove in white_unmoves {
                let mut white_position = black_position.clone();
                let white_move = white_position.undo_move(&white_unmove);
                if !white_position.checked_slow(Color::WHITE)
                    || white_position.checked_slow(Color::BLACK)
                {
                    continue;
                }
                let following = MovementList::cons(white_move, following.clone());

                if step == 1 {
                    if self
                        .initial_position_digests
                        .contains(&white_position.digest())
                    {
                        solutions.push(following.vec());
                    }
                    continue;
                }

                let mut black_unmoves = Vec::new();
                previous(&mut white_position, true, &mut black_unmoves);
                for black_unmove in black_unmoves {
                    let mut previous_black_position = white_position.clone();
                    let black_move = previous_black_position.undo_move(&black_unmove);
                    if previous_black_position.checked_slow(Color::WHITE) {
                        continue;
                    }
                    let following = MovementList::cons(black_move, following.clone());

                    if step == 2 {
                        if self
                            .initial_position_digests
                            .contains(&previous_black_position.digest())
                        {
                            solutions.push(following.vec());
                        }
                    } else {
                        queue.push_back((previous_black_position, step - 2, following));
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
enum MovementList {
    Nil,
    Cons {
        current: Movement,
        following: Rc<MovementList>,
    },
}

impl MovementList {
    fn nil() -> Rc<Self> {
        Rc::new(Self::Nil)
    }

    fn cons(current: Movement, following: Rc<Self>) -> Rc<Self> {
        Rc::new(Self::Cons { current, following })
    }

    fn vec(self: &Rc<Self>) -> Vec<Movement> {
        let mut result = Vec::new();
        let mut current = self;
        loop {
            match current.as_ref() {
                Self::Nil => return result,
                Self::Cons {
                    current: movement,
                    following,
                } => {
                    result.push(*movement);
                    current = following;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sfen;

    #[test]
    fn recognizes_black_king_mate_only() {
        let mut mate = PositionAux::from_sfen("7rK/7g1/9/9/9/9/9/9/4k4 b - 1").unwrap();
        assert!(is_help_selfmate(&mut mate));

        mate.set_pawn_drop(true);
        assert!(!is_help_selfmate(&mut mate), "白の打歩詰めは解にしない");
        mate.set_pawn_drop(false);
        mate.set_turn(Color::WHITE);
        assert!(!is_help_selfmate(&mut mate));
    }

    #[test]
    fn solves_white_to_move_help_selfmate() {
        // 白は 3一飛で2一の角を取るか、2二金で取って黒玉を詰める。
        let position = PositionAux::from_sfen("6rBK/7g1/5k3/9/9/9/9/9/9 w - 1").unwrap();
        let solutions = help_selfmate_solve(position, 10, true).unwrap().solutions();
        let encoded: Vec<Vec<String>> = solutions
            .iter()
            .map(|solution| solution.iter().map(sfen::encode_move).collect())
            .collect();

        assert!(
            encoded.iter().any(|solution| solution == &["3a2a"]),
            "{encoded:?}"
        );
        assert!(
            encoded.iter().any(|solution| solution == &["2b2a"]),
            "{encoded:?}"
        );
    }

    #[test]
    fn solves_two_move_help_selfmate_from_black_turn() {
        let position = PositionAux::from_sfen("6r1K/7gB/5k3/9/9/9/9/9/9 b - 1").unwrap();
        let reconstructor = help_selfmate_solve(position, 10, true).unwrap();
        assert_eq!(reconstructor.mate_in(), Some(2));

        let encoded: Vec<Vec<String>> = reconstructor
            .solutions()
            .iter()
            .map(|solution| solution.iter().map(sfen::encode_move).collect())
            .collect();
        assert_eq!(encoded.len(), 4, "同一手順を重複して返さない: {encoded:?}");
        assert!(
            encoded.iter().any(|solution| solution == &["1b2a", "3a2a"]),
            "{encoded:?}"
        );
    }

    #[test]
    fn bounded_search_includes_shorter_and_longer_solutions() {
        let position =
            PositionAux::from_sfen("9/9/9/9/7l1/9/8k/9/7SK b G2r2b3g3s4n3l18p 1").unwrap();
        let solutions = help_selfmate_solutions_within(position, 4, 100).unwrap();

        assert_eq!(
            solutions.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![2, 4, 4, 4]
        );
    }

    #[test]
    fn requires_both_kings() {
        let position = PositionAux::from_sfen("8k/9/9/9/9/9/9/9/9 b - 1").unwrap();
        assert!(help_selfmate_solve(position, 1, true).is_err());
    }
}
