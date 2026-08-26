use fmrs_core::{piece::Kind, position::Square};
use serde::{Deserialize, Serialize};

use crate::VariableId;

/// 盤上の着手で、どの駒を動かしたことが見えているか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MoveIdentity {
    Known,
    Variable(VariableId),
    /// 駒台で個体を区別せずに打たれ、元のIDが観測できない覆面駒。
    AnonymousVariable,
}

/// 駒打ちで見えている駒の情報。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DropIdentity {
    Known(Kind),
    Variable(VariableId),
    /// 同じ駒台のどの覆面駒を選んだか観測できない駒打ち。
    AnonymousVariable,
}

/// 棋譜から観測できる着手。
///
/// 覆面駒の具体的な駒種は含めない。「成」は観測できるが、覆面駒の
/// 「生」は特定の駒種を選んだという追加情報を与えない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ObservedMove {
    Move {
        identity: MoveIdentity,
        #[serde(with = "square_serde")]
        source: Square,
        #[serde(with = "square_serde")]
        destination: Square,
        promote: bool,
    },
    Drop {
        identity: DropIdentity,
        #[serde(with = "square_serde")]
        destination: Square,
    },
}

impl std::fmt::Display for ObservedMove {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ObservedMove::Move {
                identity,
                source,
                destination,
                promote,
            } => write!(
                formatter,
                "{}{}-{}{}",
                move_identity_label(identity),
                square_label(source),
                square_label(destination),
                if promote { "+" } else { "" }
            ),
            ObservedMove::Drop {
                identity,
                destination,
            } => write!(
                formatter,
                "{}*{}",
                drop_identity_label(identity),
                square_label(destination)
            ),
        }
    }
}

fn move_identity_label(identity: MoveIdentity) -> String {
    match identity {
        MoveIdentity::Known => String::new(),
        MoveIdentity::Variable(VariableId(id)) => format!("V{id}:"),
        MoveIdentity::AnonymousVariable => "V:".to_string(),
    }
}

fn drop_identity_label(identity: DropIdentity) -> String {
    match identity {
        DropIdentity::Known(kind) => kind_label(kind).to_string(),
        DropIdentity::Variable(VariableId(id)) => format!("V{id}"),
        DropIdentity::AnonymousVariable => "V".to_string(),
    }
}

fn kind_label(kind: Kind) -> &'static str {
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

fn square_label(square: Square) -> String {
    format!("{}{}", square.col() + 1, square.row() + 1)
}

mod square_serde {
    use fmrs_core::position::Square;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(square: &Square, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}{}", square.col() + 1, square.row() + 1))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Square, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let bytes = value.as_bytes();
        if bytes.len() != 2
            || !(b'1'..=b'9').contains(&bytes[0])
            || !(b'1'..=b'9').contains(&bytes[1])
        {
            return Err(D::Error::custom(
                "マスは11から99までの2桁で指定してください",
            ));
        }
        Ok(Square::new(
            (bytes[0] - b'1') as usize,
            (bytes[1] - b'1') as usize,
        ))
    }
}
