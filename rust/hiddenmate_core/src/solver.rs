use fmrs_core::piece::{Color, Kind};

use crate::{DropIdentity, HiddenState, MoveIdentity, ObservedMove};

pub type Solution = Vec<ObservedMove>;

/// 手順を、通常駒の駒種と覆面駒の所属が分かる日本語表記にする。
pub fn format_solution_japanese(initial: &HiddenState, solution: &Solution) -> Vec<String> {
    let mut state = initial.clone();
    let mut result = Vec::with_capacity(solution.len());
    for &observed in solution {
        result.push(format_observed_japanese(&state, observed));
        state = state
            .apply(observed)
            .expect("解手順は現在の候補世界で適用できる");
    }
    result
}

fn format_observed_japanese(state: &HiddenState, observed: ObservedMove) -> String {
    match observed {
        ObservedMove::Move {
            identity: MoveIdentity::Known,
            source,
            destination,
            promote,
        } => {
            let kind = state.worlds()[0]
                .position()
                .get(source)
                .expect("既知駒の移動元に駒がある")
                .1;
            format!(
                "{}{}{}({})",
                square_label(destination),
                japanese_kind(kind),
                if promote { "成" } else { "" },
                square_label(source)
            )
        }
        ObservedMove::Move {
            identity: MoveIdentity::Variable(id),
            source,
            destination,
            promote,
        } => {
            let color = state.worlds()[0]
                .variable(id)
                .expect("覆面駒が存在する")
                .color;
            format!(
                "{}{}{}({})",
                square_label(destination),
                color_symbol(color),
                if promote { "成" } else { "" },
                square_label(source)
            )
        }
        ObservedMove::Drop {
            identity: DropIdentity::Known(kind),
            destination,
        } => format!("{}{}打", square_label(destination), japanese_kind(kind)),
        ObservedMove::Drop {
            identity: DropIdentity::Variable(id),
            destination,
        } => {
            let color = state.worlds()[0]
                .variable(id)
                .expect("覆面駒が存在する")
                .color;
            format!("{}{}打", square_label(destination), color_symbol(color))
        }
    }
}

fn square_label(square: fmrs_core::position::Square) -> String {
    format!("{}{}", square.col() + 1, square.row() + 1)
}

fn color_symbol(color: Color) -> &'static str {
    if color.is_black() {
        "▲"
    } else {
        "△"
    }
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

/// 指定手数以下で詰む協力手順を、短い順に列挙する。
///
/// 両者は協力するため minimax ではなく、観測着手の存在探索となる。
/// `max_solutions` に達した時点で打ち切る。
pub fn solve_exact(initial: &HiddenState, plies: usize, max_solutions: usize) -> Vec<Solution> {
    if max_solutions == 0 {
        return Vec::new();
    }

    let mut solutions = Vec::new();
    for depth in 0..=plies {
        let turn_at_depth = if depth % 2 == 0 {
            initial.turn()
        } else {
            initial.turn().opposite()
        };
        if turn_at_depth != initial.rule().terminal_turn() {
            continue;
        }

        let mut path = Vec::with_capacity(depth);
        solve_inner(initial, depth, max_solutions, &mut path, &mut solutions);
        if solutions.len() >= max_solutions {
            break;
        }
    }
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

#[cfg(test)]
mod tests {
    use fmrs_core::{
        piece::{Color, Kind},
        position::Square,
    };

    use super::*;
    use crate::{MateRule, VariableId, VariableLocation, VariableProblem, VariableSpec};

    fn state_with_variable(color: Color) -> HiddenState {
        VariableProblem {
            base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b - 1".to_string(),
            variables: vec![VariableSpec {
                id: VariableId(1),
                color,
                location: VariableLocation::Board(Square::S64),
                candidates: vec![Kind::Rook, Kind::ProRook],
            }],
            rule: MateRule::Helpmate,
        }
        .enumerate()
        .unwrap()
    }

    #[test]
    fn formats_known_move_and_drop_in_japanese() {
        let state = VariableProblem {
            base_sfen: "9/1S7/9/9/9/9/9/9/k8 b - 1".to_string(),
            variables: vec![VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Board(Square::S64),
                candidates: vec![Kind::Rook],
            }],
            rule: MateRule::Helpmate,
        }
        .enumerate()
        .unwrap();
        assert_eq!(
            format_observed_japanese(
                &state,
                ObservedMove::Move {
                    identity: MoveIdentity::Known,
                    source: Square::S82,
                    destination: Square::S83,
                    promote: false,
                }
            ),
            "83銀(82)"
        );
        assert_eq!(
            format_observed_japanese(
                &state,
                ObservedMove::Drop {
                    identity: DropIdentity::Known(Kind::Pawn),
                    destination: Square::S92,
                }
            ),
            "92歩打"
        );
    }

    #[test]
    fn formats_variable_with_owner_symbol() {
        let black = state_with_variable(Color::BLACK);
        let white = state_with_variable(Color::WHITE);
        let observed = ObservedMove::Move {
            identity: MoveIdentity::Variable(VariableId(1)),
            source: Square::S64,
            destination: Square::S84,
            promote: false,
        };
        assert_eq!(format_observed_japanese(&black, observed), "84▲(64)");
        assert_eq!(format_observed_japanese(&white, observed), "84△(64)");
    }
}
