use anyhow::{bail, Context, Result};
use fmrs_core::{
    piece::{Color, Kind, KINDS},
    position::Square,
};
use serde::Deserialize;

use crate::{VariableId, VariableProblem, VariableSpec};

/// CLI・Web UIで共有する覆面駒問題のJSON形式。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemDocument {
    pub base_sfen: String,
    pub plies: usize,
    pub variables: Vec<VariableDocument>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDocument {
    pub id: u16,
    pub color: DocumentColor,
    pub square: Option<String>,
    #[serde(default)]
    pub in_hand: bool,
    pub candidates: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentColor {
    Black,
    White,
}

impl ProblemDocument {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("問題JSONを解釈できません")
    }

    pub fn into_problem(self) -> Result<VariableProblem> {
        let variables = self
            .variables
            .into_iter()
            .map(VariableDocument::into_spec)
            .collect::<Result<Vec<_>>>()?;
        Ok(VariableProblem {
            base_sfen: self.base_sfen,
            variables,
        })
    }
}

impl VariableDocument {
    fn into_spec(self) -> Result<VariableSpec> {
        let candidates = match self.candidates {
            Some(candidates) => candidates
                .iter()
                .map(|candidate| parse_kind(candidate))
                .collect::<Result<Vec<_>>>()?,
            None => KINDS.to_vec(),
        };
        let color = match self.color {
            DocumentColor::Black => Color::BLACK,
            DocumentColor::White => Color::WHITE,
        };
        let location = match (self.square, self.in_hand) {
            (Some(square), false) => crate::VariableLocation::Board(parse_square(&square)?),
            (None, true) => crate::VariableLocation::Hand(color),
            (Some(_), true) => bail!("覆面駒V{}にsquareとinHandの両方が指定されています", self.id),
            (None, false) => bail!("覆面駒V{}の配置場所が指定されていません", self.id),
        };
        Ok(VariableSpec {
            id: VariableId(self.id),
            color,
            location,
            candidates,
        })
    }
}

fn parse_square(value: &str) -> Result<Square> {
    let bytes = value.as_bytes();
    if bytes.len() != 2 || !(b'1'..=b'9').contains(&bytes[0]) || !(b'1'..=b'9').contains(&bytes[1])
    {
        bail!("マス `{value}` は11から99までの2桁で指定してください");
    }
    Ok(Square::new(
        (bytes[0] - b'1') as usize,
        (bytes[1] - b'1') as usize,
    ))
}

fn parse_kind(value: &str) -> Result<Kind> {
    match value.to_ascii_uppercase().as_str() {
        "P" => Ok(Kind::Pawn),
        "L" => Ok(Kind::Lance),
        "N" => Ok(Kind::Knight),
        "S" => Ok(Kind::Silver),
        "G" => Ok(Kind::Gold),
        "B" => Ok(Kind::Bishop),
        "R" => Ok(Kind::Rook),
        "K" => Ok(Kind::King),
        "+P" => Ok(Kind::ProPawn),
        "+L" => Ok(Kind::ProLance),
        "+N" => Ok(Kind::ProKnight),
        "+S" => Ok(Kind::ProSilver),
        "+B" => Ok(Kind::ProBishop),
        "+R" => Ok(Kind::ProRook),
        _ => bail!("未知の駒種 `{value}` です"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_problem_document() {
        let document = ProblemDocument::from_json(
            r#"{
                "baseSfen": "9/9/k8/9/9/9/9/9/9 b - 1",
                "plies": 1,
                "variables": [{
                "id": 7,
                "color": "black",
                "square": "64",
                    "candidates": ["R", "+R"]
                }]
            }"#,
        )
        .unwrap();
        assert_eq!(document.plies, 1);

        let problem = document.into_problem().unwrap();
        assert_eq!(problem.variables[0].id, VariableId(7));
        assert_eq!(
            problem.variables[0].location,
            crate::VariableLocation::Board(Square::new(5, 3))
        );
        assert_eq!(
            problem.variables[0].candidates,
            vec![Kind::Rook, Kind::ProRook]
        );
    }

    #[test]
    fn defaults_to_all_kinds_and_parses_hand_location() {
        let document = ProblemDocument::from_json(
            r#"{
                "baseSfen": "9/9/k8/9/9/9/9/9/9 b - 1",
                "plies": 1,
                "variables": [{"id": 2, "color": "white", "inHand": true}]
            }"#,
        )
        .unwrap();
        let problem = document.into_problem().unwrap();
        assert_eq!(problem.variables[0].candidates, KINDS);
        assert_eq!(
            problem.variables[0].location,
            crate::VariableLocation::Hand(Color::WHITE)
        );
    }
}
