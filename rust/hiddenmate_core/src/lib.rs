//! HiddenMate の覆面駒（Variable）推論コア。
//!
//! `fmrs_core` の確定局面を「候補世界」として複数保持し、盤上で観測
//! できる着手ごとに候補世界を絞り込む。覆面駒そのものを fmrs の
//! `Kind` に追加しないため、既存の高速な合法手生成をそのまま利用できる。

mod best_mate;
mod format;
mod known_invisible;
mod observed;
mod problem;
mod rule;
mod solver;
mod state;
mod world;

pub use best_mate::{solve_best_mate, BestMateResult};
pub use format::ProblemDocument;
pub use known_invisible::{
    format_known_invisible_solution_japanese, solve_known_invisible_exact, KnownInvisibleDocument,
    KnownInvisibleObservedMove, KnownInvisibleProblem, KnownInvisibleSolution, KnownInvisibleSpec,
    KnownInvisibleState,
};
pub use observed::{DropIdentity, MoveIdentity, ObservedMove};
pub use problem::{VariableProblem, VariableSpec};
pub use rule::{HandVariableMode, MateRule};
pub use solver::{format_solution_japanese, solve_exact, Solution};
pub use state::HiddenState;
pub use world::{ConcreteWorld, VariableId, VariableLocation, VariablePiece};
