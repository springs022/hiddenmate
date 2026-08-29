use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use hiddenmate_core::{
    format_known_invisible_solution_japanese, format_solution_japanese, solve_exact,
    solve_known_invisible_exact, KnownInvisibleDocument, ProblemDocument,
};

#[derive(Debug, Parser)]
#[command(
    name = "hiddenmate",
    version,
    about = "覆面駒・透明駒入り協力詰を検討します"
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
    let value: serde_json::Value =
        serde_json::from_str(&json).context("問題JSONを解釈できません")?;
    if value.get("invisibles").is_some() {
        return solve_known_invisible(&json, arguments.max_solutions);
    }
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

fn solve_known_invisible(json: &str, max_solutions: usize) -> Result<()> {
    let document = KnownInvisibleDocument::from_json(json)?;
    let (problem, plies) = document.into_problem()?;
    let state = problem.enumerate()?;
    println!("初形候補世界: {}", state.world_count());
    let solutions = solve_known_invisible_exact(&state, plies, max_solutions)?;
    println!("解数: {}", solutions.len());
    for (index, solution) in solutions.iter().enumerate() {
        let moves = format_known_invisible_solution_japanese(&state, solution)?.join(" ");
        println!("{}: {}", index + 1, moves);
    }
    Ok(())
}
