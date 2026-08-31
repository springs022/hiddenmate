//! HiddenMate の覆面駒（Variable）推論コア。
//!
//! `fmrs_core` の確定局面を「候補世界」として複数保持し、盤上で観測
//! できる着手ごとに候補世界を絞り込む。覆面駒そのものを fmrs の
//! `Kind` に追加しないため、既存の高速な合法手生成をそのまま利用できる。

mod format;
mod kind_set;
mod known_invisible;
mod metrics;
mod observed;
mod problem;
mod rule;
mod solver;
mod state;
mod world;

pub use format::ProblemDocument;
pub use known_invisible::{
    format_known_invisible_solution_japanese, solve_known_invisible_exact,
    solve_known_invisible_exact_profiled, solve_replay_known_invisible_exact,
    solve_replay_known_invisible_exact_profiled, KnownInvisibleDocument,
    KnownInvisibleObservedMove, KnownInvisibleProblem, KnownInvisibleSolution, KnownInvisibleSpec,
    KnownInvisibleState, ReplayKnownInvisibleState,
};
pub use metrics::{EnumerationMetrics, SolveMetrics};
pub use observed::{DropIdentity, MoveIdentity, ObservedMove};
pub use problem::{VariableProblem, VariableSpec};
pub use rule::{HandVariableMode, MateRule};
pub use solver::{
    format_solution_japanese, solve_exact, solve_exact_profiled, solve_replay_exact,
    solve_replay_exact_profiled, Solution,
};
pub use state::{HiddenState, ReplayHiddenState};
pub use world::{ConcreteWorld, VariableId, VariableLocation, VariablePiece};
