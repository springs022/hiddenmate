use std::{env, time::Instant};

use anyhow::{bail, Result};
use fmrs_core::{
    piece::{Color, Kind},
    position::Square,
};
use hiddenmate_core::{
    solve_exact_profiled, solve_known_invisible_exact_profiled, solve_replay_exact_profiled,
    solve_replay_known_invisible_exact_profiled, HandVariableMode, KnownInvisibleProblem,
    KnownInvisibleSpec, MateRule, VariableId, VariableLocation, VariableProblem, VariableSpec,
};
use serde_json::json;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let scenario = args.next().unwrap_or_else(|| "variable2-1ply".to_string());
    let backend = args.next().unwrap_or_else(|| "explicit".to_string());
    if args.next().is_some() {
        bail!("usage: backend_benchmark [scenario] [explicit|replay]");
    }

    match scenario.as_str() {
        "variable2-1ply" => run_variable(&scenario, &backend, variable_two(), 1, 100),
        "variable6-init" => run_variable(&scenario, &backend, variable_six(), 0, 0),
        "invisible1-5ply" => run_invisible(&scenario, &backend, invisible_one(), 5, 2),
        "invisible2-1ply" => run_invisible(&scenario, &backend, invisible_two(), 1, 1),
        _ => bail!("unknown scenario: {scenario}"),
    }
}

fn run_variable(
    scenario: &str,
    backend: &str,
    problem: VariableProblem,
    plies: usize,
    max_solutions: usize,
) -> Result<()> {
    match backend {
        "explicit" => {
            let started = Instant::now();
            let state = problem
                .enumerate_explicit_with_hand_variable_mode(HandVariableMode::Indistinguishable)?;
            let init = started.elapsed();
            let (solutions, metrics) = solve_exact_profiled(&state, plies, max_solutions);
            print_result(
                scenario,
                backend,
                state.world_count(),
                init.as_micros(),
                metrics.total_elapsed.as_micros(),
                solutions.len(),
                metrics.visited_state_count,
                metrics.generated_transition_count,
            );
        }
        "replay" => {
            let started = Instant::now();
            let state = problem
                .enumerate_replay_with_hand_variable_mode(HandVariableMode::Indistinguishable)?;
            let init = started.elapsed();
            let (solutions, metrics) = solve_replay_exact_profiled(&state, plies, max_solutions)?;
            print_result(
                scenario,
                backend,
                state.world_count(),
                init.as_micros(),
                metrics.total_elapsed.as_micros(),
                solutions.len(),
                metrics.visited_state_count,
                metrics.generated_transition_count,
            );
        }
        _ => bail!("unknown backend: {backend}"),
    }
    Ok(())
}

fn run_invisible(
    scenario: &str,
    backend: &str,
    problem: KnownInvisibleProblem,
    plies: usize,
    max_solutions: usize,
) -> Result<()> {
    match backend {
        "explicit" => {
            let started = Instant::now();
            let state = problem.enumerate_explicit()?;
            let init = started.elapsed();
            let (solutions, metrics) =
                solve_known_invisible_exact_profiled(&state, plies, max_solutions)?;
            print_result(
                scenario,
                backend,
                state.world_count(),
                init.as_micros(),
                metrics.total_elapsed.as_micros(),
                solutions.len(),
                metrics.visited_state_count,
                metrics.generated_transition_count,
            );
        }
        "replay" => {
            let started = Instant::now();
            let state = problem.enumerate_replay()?;
            let init = started.elapsed();
            let (solutions, metrics) =
                solve_replay_known_invisible_exact_profiled(&state, plies, max_solutions)?;
            print_result(
                scenario,
                backend,
                state.world_count(),
                init.as_micros(),
                metrics.total_elapsed.as_micros(),
                solutions.len(),
                metrics.visited_state_count,
                metrics.generated_transition_count,
            );
        }
        _ => bail!("unknown backend: {backend}"),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_result(
    scenario: &str,
    backend: &str,
    world_count: usize,
    init_micros: u128,
    solve_micros: u128,
    solutions: usize,
    visited_states: usize,
    transitions: usize,
) {
    println!(
        "{}",
        json!({
            "scenario": scenario,
            "backend": backend,
            "worldCount": world_count,
            "initMicros": init_micros,
            "solveMicros": solve_micros,
            "solutions": solutions,
            "visitedStates": visited_states,
            "transitions": transitions,
            "peakWorkingSetBytes": peak_working_set_bytes(),
        })
    );
}

#[cfg(windows)]
fn peak_working_set_bytes() -> usize {
    use std::{ffi::c_void, mem};

    #[allow(non_snake_case)]
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        PageFaultCount: u32,
        PeakWorkingSetSize: usize,
        WorkingSetSize: usize,
        QuotaPeakPagedPoolUsage: usize,
        QuotaPagedPoolUsage: usize,
        QuotaPeakNonPagedPoolUsage: usize,
        QuotaNonPagedPoolUsage: usize,
        PagefileUsage: usize,
        PeakPagefileUsage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    let mut counters: ProcessMemoryCounters = unsafe { mem::zeroed() };
    counters.cb = mem::size_of::<ProcessMemoryCounters>() as u32;
    let succeeded = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    if succeeded == 0 {
        0
    } else {
        counters.PeakWorkingSetSize
    }
}

#[cfg(not(windows))]
fn peak_working_set_bytes() -> usize {
    0
}

fn variable_two() -> VariableProblem {
    VariableProblem {
        base_sfen: "9/9/kS7/9/1L7/9/9/9/9 b 2r2b4g3s4n3l18p 1".to_string(),
        rule: MateRule::Helpmate,
        variables: (1..=2)
            .map(|id| VariableSpec {
                id: VariableId(id),
                color: Color::BLACK,
                location: VariableLocation::Hand(Color::BLACK),
                candidates: fmrs_core::piece::KINDS[..7].to_vec(),
            })
            .collect(),
    }
}

fn variable_six() -> VariableProblem {
    let squares = [
        Square::S99,
        Square::S89,
        Square::S79,
        Square::S69,
        Square::S59,
        Square::S49,
    ];
    VariableProblem {
        base_sfen: "8k/9/9/9/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: squares
            .into_iter()
            .enumerate()
            .map(|(index, square)| VariableSpec {
                id: VariableId(index as u16 + 1),
                color: Color::BLACK,
                location: VariableLocation::Board(square),
                candidates: vec![Kind::Pawn, Kind::Lance, Kind::Knight, Kind::Silver],
            })
            .collect(),
    }
}

fn invisible_one() -> KnownInvisibleProblem {
    KnownInvisibleProblem {
        base_sfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1".to_string(),
        invisibles: vec![KnownInvisibleSpec {
            color: Color::BLACK,
            kind: Kind::Lance,
        }],
        rule: MateRule::Helpmate,
    }
}

fn invisible_two() -> KnownInvisibleProblem {
    KnownInvisibleProblem {
        base_sfen: "7k1/9/7K1/9/9/9/9/9/9 b - 1".to_string(),
        invisibles: vec![
            KnownInvisibleSpec {
                color: Color::BLACK,
                kind: Kind::Lance,
            },
            KnownInvisibleSpec {
                color: Color::WHITE,
                kind: Kind::Rook,
            },
        ],
        rule: MateRule::Helpmate,
    }
}
