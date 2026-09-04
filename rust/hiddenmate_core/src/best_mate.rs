use std::collections::HashMap;

use anyhow::{bail, Result};
use fmrs_core::piece::Color;

use crate::{HiddenState, MateRule, Solution};

/// 最善詰の検討結果。
///
/// `mate_in` は攻方最短・受方最長での手数。`variations` には最善攻と、
/// 詰む場合に最長となる受方応手を、従来UIで扱える手順へ展開して返す。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BestMateResult {
    pub mate_in: usize,
    pub variations: Vec<Solution>,
    pub variations_truncated: bool,
}

type DistanceMemo = HashMap<(String, usize), Option<usize>>;

/// 指定手数以内の最善詰を検討する。
///
/// 攻方は強制詰の手数を最小化し、受方は一つでも不詰になる応手があれば
/// それを選び、すべて詰む場合は手数を最大化する。受方応手の検証自体は
/// `max_variations` では打ち切らず、返却する展開手順だけを制限する。
pub fn solve_best_mate(
    initial: &HiddenState,
    plies: usize,
    max_variations: usize,
) -> Result<Option<BestMateResult>> {
    if initial.rule() != MateRule::BestMate {
        bail!("最善詰以外の状態が最善詰ソルバーへ渡されました");
    }

    let mut memo = DistanceMemo::new();
    let Some(mate_in) = best_mate_distance(initial, plies, &mut memo) else {
        return Ok(None);
    };

    let mut variations = Vec::new();
    let mut path = Vec::with_capacity(mate_in);
    let variations_truncated = collect_variations(
        initial,
        plies,
        mate_in,
        max_variations,
        &mut path,
        &mut variations,
        &mut memo,
    );

    Ok(Some(BestMateResult {
        mate_in,
        variations,
        variations_truncated,
    }))
}

fn best_mate_distance(
    state: &HiddenState,
    remaining: usize,
    memo: &mut DistanceMemo,
) -> Option<usize> {
    if state.is_proven_mate() {
        return Some(0);
    }
    if remaining == 0 {
        return None;
    }

    let key = (state.search_key(), remaining);
    if let Some(&cached) = memo.get(&key) {
        return cached;
    }

    let observed_moves = state.observed_moves();
    let child_distances = observed_moves
        .into_iter()
        .map(|observed| {
            state
                .apply(observed)
                .and_then(|next| best_mate_distance(&next, remaining - 1, memo))
        })
        .collect();
    let result = combine_child_distances(state.turn(), child_distances);

    memo.insert(key, result);
    result
}

fn combine_child_distances(turn: Color, child_distances: Vec<Option<usize>>) -> Option<usize> {
    if child_distances.is_empty() {
        return None;
    }
    if turn == Color::BLACK {
        child_distances
            .into_iter()
            .flatten()
            .min()
            .map(|value| value + 1)
    } else {
        child_distances
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|values| values.into_iter().max())
            .map(|value| value + 1)
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_variations(
    state: &HiddenState,
    remaining: usize,
    distance: usize,
    max_variations: usize,
    path: &mut Solution,
    variations: &mut Vec<Solution>,
    memo: &mut DistanceMemo,
) -> bool {
    if distance == 0 {
        if variations.len() < max_variations {
            variations.push(path.clone());
            return false;
        }
        return true;
    }

    for observed in state.observed_moves() {
        let Some(next) = state.apply(observed) else {
            continue;
        };
        let Some(child_distance) = best_mate_distance(&next, remaining - 1, memo) else {
            continue;
        };
        // 攻方は最短、受方は最長となる着手だけを表示する。
        if child_distance + 1 != distance {
            continue;
        }

        path.push(observed);
        let truncated = collect_variations(
            &next,
            remaining - 1,
            child_distance,
            max_variations,
            path,
            variations,
            memo,
        );
        path.pop();
        if truncated {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HandVariableMode, VariableProblem};

    #[test]
    fn finds_one_ply_best_mate() {
        let state = VariableProblem {
            base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b R 1".to_string(),
            variables: vec![],
            rule: MateRule::BestMate,
        }
        .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
        .unwrap();

        let result = solve_best_mate(&state, 1, 10).unwrap().unwrap();
        assert_eq!(result.mate_in, 1);
        assert!(!result.variations.is_empty());
        assert!(result.variations.iter().all(|line| line.len() == 1));
        assert!(!result.variations_truncated);
    }

    #[test]
    fn zero_variation_limit_does_not_skip_the_proof() {
        let state = VariableProblem {
            base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b R 1".to_string(),
            variables: vec![],
            rule: MateRule::BestMate,
        }
        .enumerate()
        .unwrap();

        let result = solve_best_mate(&state, 1, 0).unwrap().unwrap();
        assert_eq!(result.mate_in, 1);
        assert!(result.variations.is_empty());
        assert!(result.variations_truncated);
    }

    #[test]
    fn attacker_chooses_shortest_forced_child() {
        assert_eq!(
            combine_child_distances(Color::BLACK, vec![Some(4), None, Some(2)]),
            Some(3)
        );
    }

    #[test]
    fn defender_refutes_with_one_escape_and_otherwise_chooses_longest() {
        assert_eq!(
            combine_child_distances(Color::WHITE, vec![Some(2), None, Some(4)]),
            None
        );
        assert_eq!(
            combine_child_distances(Color::WHITE, vec![Some(2), Some(4)]),
            Some(5)
        );
    }
}
