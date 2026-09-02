mod backward_search;
mod solver;
mod utils;

use std::collections::BTreeMap;

use fmrs_core::piece::Kind;
use hiddenmate_core::{
    format_known_invisible_solution_japanese, format_solution_japanese, solve_best_mate,
    solve_exact, solve_known_invisible_exact, HiddenState, KnownInvisibleDocument, MateRule,
    ObservedMove, ProblemDocument, Solution,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[wasm_bindgen]
pub struct OneWayMateResult {
    pub is_one_way: bool,
    pub steps: u32,
}

#[wasm_bindgen]
pub fn is_white_in_check(sfen: &str) -> bool {
    let Ok(mut position) = fmrs_core::position::position::PositionAux::from_sfen(sfen) else {
        return false;
    };
    position.checked_slow(fmrs_core::piece::Color::WHITE)
}

#[wasm_bindgen]
pub fn check_one_way_mate(sfen: &str) -> Option<OneWayMateResult> {
    let mut position = fmrs_core::position::position::PositionAux::from_sfen(sfen).ok()?;
    if position.checked_slow(fmrs_core::piece::Color::WHITE) {
        position.set_turn(fmrs_core::piece::Color::WHITE);
    }
    match fmrs_core::solve::one_way::one_way_mate_steps(&mut position, &mut vec![]) {
        Ok(s) => Some(OneWayMateResult {
            is_one_way: true,
            steps: s as u32,
        }),
        Err(s) => Some(OneWayMateResult {
            is_one_way: false,
            steps: s as u32,
        }),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VariableSolveResponse {
    world_count: usize,
    candidates: Vec<VariableCandidates>,
    solutions: Vec<Vec<String>>,
    solution_candidates: Vec<Vec<Vec<VariableCandidates>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    best_mate_in: Option<usize>,
    variations_truncated: bool,
}

#[derive(Clone, Serialize)]
struct VariableCandidates {
    id: u16,
    kinds: Vec<&'static str>,
}

/// 覆面駒問題JSONを解き、Web UI向けのJSONを返す。
#[wasm_bindgen]
pub fn solve_variable_problem(json: &str, max_solutions: u32) -> Result<String, JsValue> {
    solve_variable_problem_json(json, max_solutions as usize)
        .map_err(|error| JsValue::from_str(&format!("{error:#}")))
}

fn solve_variable_problem_json(json: &str, max_solutions: usize) -> anyhow::Result<String> {
    let document = ProblemDocument::from_json(json)?;
    let plies = document.plies;
    let (problem, hand_variable_mode) = document.into_problem_with_hand_variable_mode()?;
    let state = problem.enumerate_with_hand_variable_mode(hand_variable_mode)?;
    let candidates = variable_candidates(&state);
    let (raw_solutions, best_mate_in, variations_truncated) = if state.rule() == MateRule::BestMate
    {
        match solve_best_mate(&state, plies, max_solutions)? {
            Some(result) => (
                result.variations,
                Some(result.mate_in),
                result.variations_truncated,
            ),
            None => (Vec::new(), None, false),
        }
    } else {
        (solve_exact(&state, plies, max_solutions), None, false)
    };
    let solution_candidates = collect_solution_candidates(&state, &raw_solutions)?;
    let solutions = raw_solutions
        .iter()
        .map(|solution| format_solution_japanese(&state, solution))
        .collect();
    let response = VariableSolveResponse {
        world_count: state.world_count(),
        candidates,
        solutions,
        solution_candidates,
        best_mate_in,
        variations_truncated,
    };
    Ok(serde_json::to_string(&response)?)
}

fn variable_candidates(state: &HiddenState) -> Vec<VariableCandidates> {
    state
        .all_candidates()
        .into_iter()
        .map(|(id, kinds)| VariableCandidates {
            id: id.0,
            kinds: kinds.into_iter().map(kind_code).collect(),
        })
        .collect()
}

fn collect_solution_candidates(
    initial: &HiddenState,
    solutions: &[Solution],
) -> anyhow::Result<Vec<Vec<Vec<VariableCandidates>>>> {
    let mut result = solutions
        .iter()
        .map(|solution| Vec::with_capacity(solution.len()))
        .collect::<Vec<_>>();
    let solution_indices = (0..solutions.len()).collect::<Vec<_>>();
    collect_solution_candidates_inner(initial, solutions, &solution_indices, 0, &mut result)?;
    Ok(result)
}

fn collect_solution_candidates_inner(
    state: &HiddenState,
    solutions: &[Solution],
    solution_indices: &[usize],
    depth: usize,
    result: &mut [Vec<Vec<VariableCandidates>>],
) -> anyhow::Result<()> {
    let mut groups = BTreeMap::<ObservedMove, Vec<usize>>::new();
    for &solution_index in solution_indices {
        if let Some(&observed) = solutions[solution_index].get(depth) {
            groups.entry(observed).or_default().push(solution_index);
        }
    }

    for (observed, group) in groups {
        let next = state.apply(observed).ok_or_else(|| {
            anyhow::anyhow!(
                "解{}の{}手目を候補世界へ適用できません",
                group[0] + 1,
                depth + 1
            )
        })?;
        let candidates = variable_candidates(&next);
        for &solution_index in &group {
            result[solution_index].push(candidates.clone());
        }
        collect_solution_candidates_inner(&next, solutions, &group, depth + 1, result)?;
    }
    Ok(())
}

fn kind_code(kind: Kind) -> &'static str {
    match kind {
        Kind::Pawn => "P",
        Kind::Lance => "L",
        Kind::Knight => "N",
        Kind::Silver => "S",
        Kind::Gold => "G",
        Kind::Bishop => "B",
        Kind::Rook => "R",
        Kind::King => "K",
        Kind::ProPawn => "+P",
        Kind::ProLance => "+L",
        Kind::ProKnight => "+N",
        Kind::ProSilver => "+S",
        Kind::ProBishop => "+B",
        Kind::ProRook => "+R",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KnownInvisibleSolveResponse {
    world_count: usize,
    solutions: Vec<Vec<String>>,
}

/// 駒種を指定した透明駒問題JSONを解き、Web UI向けのJSONを返す。
#[wasm_bindgen]
pub fn solve_known_invisible_problem(json: &str, max_solutions: u32) -> Result<String, JsValue> {
    solve_known_invisible_problem_json(json, max_solutions as usize)
        .map_err(|error| JsValue::from_str(&format!("{error:#}")))
}

fn solve_known_invisible_problem_json(json: &str, max_solutions: usize) -> anyhow::Result<String> {
    let document = KnownInvisibleDocument::from_json(json)?;
    let (problem, plies) = document.into_problem()?;
    let state = problem.enumerate()?;
    let solutions = solve_known_invisible_exact(&state, plies, max_solutions)?
        .iter()
        .map(|solution| format_known_invisible_solution_japanese(&state, solution))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(serde_json::to_string(&KnownInvisibleSolveResponse {
        world_count: state.world_count(),
        solutions,
    })?)
}

#[cfg(test)]
mod hiddenmate_tests {
    use super::*;

    #[test]
    fn solves_variable_problem_for_web() {
        let json = r#"{
            "baseSfen": "9/9/kS7/N8/1L7/9/9/9/8K b 2r2b4g3s3n3l18p 1",
            "plies": 1,
            "variables": [{
                "id": 1,
                "color": "black",
                "square": "64",
                "candidates": ["R", "+R"]
            }]
        }"#;
        let response = solve_variable_problem_json(json, 100).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["worldCount"], 2);
        assert_eq!(
            value["candidates"][0]["kinds"],
            serde_json::json!(["R", "+R"])
        );
        assert_eq!(value["solutions"][0][0], "84▲(64)");
        assert_eq!(
            value["solutionCandidates"][0].as_array().unwrap().len(),
            value["solutions"][0].as_array().unwrap().len()
        );
        assert_eq!(value["solutionCandidates"][0][0][0]["id"], 1);
    }

    #[test]
    fn even_plies_does_not_return_one_ply_solutions() {
        let json = r#"{
            "baseSfen": "9/9/kS7/N8/1L7/9/9/9/9 b 2r2b4g3s3n3l18p 1",
            "plies": 2,
            "variables": [{
                "id": 1,
                "color": "black",
                "square": "64"
            }]
        }"#;

        let response = solve_variable_problem_json(json, 100).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        let solutions = value["solutions"].as_array().unwrap();

        assert!(solutions.len() > 1);
        assert!(solutions
            .iter()
            .all(|solution| solution.as_array().unwrap().len() == 2));
    }

    #[test]
    fn solves_help_selfmate_rule_for_web() {
        let json = r#"{
            "baseSfen": "9/9/9/9/7l1/9/8k/9/7SK b G 1",
            "plies": 4,
            "rule": "helpSelfmate",
            "variables": []
        }"#;

        let response = solve_variable_problem_json(json, 100).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        let lengths = value["solutions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|solution| solution.as_array().unwrap().len())
            .collect::<Vec<_>>();

        assert_eq!(lengths, vec![2, 4, 4, 4]);
    }

    #[test]
    fn solves_best_mate_rule_for_web() {
        let json = r#"{
            "baseSfen": "9/9/kS7/N8/1L7/9/9/9/9 b R 1",
            "plies": 1,
            "rule": "bestMate",
            "variables": []
        }"#;

        let response = solve_variable_problem_json(json, 100).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["bestMateIn"], 1);
        assert_eq!(value["variationsTruncated"], false);
        assert!(!value["solutions"].as_array().unwrap().is_empty());
        assert!(value["solutions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|line| line.as_array().unwrap().len() == 1));
    }

    #[test]
    fn white_start_help_selfmate_keeps_unchecked_variable_worlds() {
        let json = r#"{
            "baseSfen": "9/9/9/9/9/9/8k/9/9 b - 1",
            "plies": 3,
            "rule": "helpSelfmate",
            "variables": [
                { "id": 1, "color": "black", "square": "29" },
                { "id": 2, "color": "black", "square": "19" }
            ]
        }"#;

        let response = solve_variable_problem_json(json, 0).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["worldCount"], 26);
        assert_eq!(
            value["candidates"][0]["kinds"].as_array().unwrap().len(),
            14
        );
        assert_eq!(
            value["candidates"][1]["kinds"].as_array().unwrap().len(),
            14
        );
        assert!(value["candidates"][0]["kinds"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("P")));
        assert!(value["candidates"][1]["kinds"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("K")));
    }

    #[test]
    fn validates_known_invisible_problem_for_web() {
        let json = r#"{
            "baseSfen":"9/9/k8/9/9/9/9/9/9 b 2r2b4g4s4n4l18p 1",
            "plies":1,
            "invisibles":[{"color":"black","kind":"K","count":1}]
        }"#;
        let response = solve_known_invisible_problem_json(json, 0).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert!(value["worldCount"].as_u64().unwrap() > 1);
        assert_eq!(value["solutions"], serde_json::json!([]));
    }
}
