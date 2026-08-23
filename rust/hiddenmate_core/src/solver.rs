use crate::{HiddenState, ObservedMove};

pub type Solution = Vec<ObservedMove>;

/// 指定手数ちょうどで詰む協力手順を列挙する初期実装。
///
/// 両者は協力するため minimax ではなく、観測着手の存在探索となる。
/// `max_solutions` に達した時点で打ち切る。
pub fn solve_exact(initial: &HiddenState, plies: usize, max_solutions: usize) -> Vec<Solution> {
    if max_solutions == 0 {
        return Vec::new();
    }
    let mut solutions = Vec::new();
    let mut path = Vec::with_capacity(plies);
    solve_inner(initial, plies, max_solutions, &mut path, &mut solutions);
    solutions
}

fn solve_inner(
    state: &HiddenState,
    remaining: usize,
    max_solutions: usize,
    path: &mut Solution,
    solutions: &mut Vec<Solution>,
) {
    if solutions.len() >= max_solutions {
        return;
    }
    if remaining == 0 {
        if state.is_proven_mate() {
            solutions.push(path.clone());
        }
        return;
    }
    if state.is_proven_mate() {
        return;
    }

    for observed in state.observed_moves() {
        let Some(next) = state.apply(observed) else {
            continue;
        };
        path.push(observed);
        solve_inner(&next, remaining - 1, max_solutions, path, solutions);
        path.pop();
        if solutions.len() >= max_solutions {
            break;
        }
    }
}
