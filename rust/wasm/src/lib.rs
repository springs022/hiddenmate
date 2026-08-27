mod backward_search;
mod solver;
mod utils;

use fmrs_core::piece::Kind;
use hiddenmate_core::{format_solution_japanese, solve_exact, ProblemDocument};
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
}

#[derive(Serialize)]
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
    let candidates = state
        .all_candidates()
        .into_iter()
        .map(|(id, kinds)| VariableCandidates {
            id: id.0,
            kinds: kinds.into_iter().map(kind_code).collect(),
        })
        .collect();
    let solutions = solve_exact(&state, plies, max_solutions)
        .iter()
        .map(|solution| format_solution_japanese(&state, solution))
        .collect();
    let response = VariableSolveResponse {
        world_count: state.world_count(),
        candidates,
        solutions,
    };
    Ok(serde_json::to_string(&response)?)
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
}
