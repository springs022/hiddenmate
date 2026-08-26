use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use hiddenmate_core::{format_solution_japanese, solve_exact, ProblemDocument};

#[derive(Debug, Parser)]
#[command(
    name = "hiddenmate",
    version,
    about = "覆面駒（Variable）入り協力詰を検討します"
)]
struct Arguments {
    /// HiddenMate問題JSON
    problem: PathBuf,

    /// 列挙する解の最大数
    #[arg(long, default_value_t = 100)]
    max_solutions: usize,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let json = fs::read_to_string(&arguments.problem)
        .with_context(|| format!("問題ファイルを読めません: {}", arguments.problem.display()))?;
    let document = ProblemDocument::from_json(&json)?;
    let plies = document.plies;
    let (problem, hand_variable_mode) = document.into_problem_with_hand_variable_mode()?;
    let state = problem.enumerate_with_hand_variable_mode(hand_variable_mode)?;

    println!("初形候補世界: {}", state.world_count());
    for (id, candidates) in state.all_candidates() {
        println!("  V{}: {:?}", id.0, candidates);
    }

    let solutions = solve_exact(&state, plies, arguments.max_solutions);
    println!("解数: {}", solutions.len());
    for (index, solution) in solutions.iter().enumerate() {
        let moves = format_solution_japanese(&state, solution).join(" ");
        println!("{}: {}", index + 1, moves);
    }
    Ok(())
}
