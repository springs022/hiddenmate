use std::collections::BTreeMap;

use fmrs_core::{
    piece::{Color, Kind},
    position::{position::PositionAux, Movement, Square},
};
use serde::{Deserialize, Serialize};

use crate::{DropIdentity, MoveIdentity, ObservedMove};

/// 覆面駒を手順中も追跡するための安定した識別子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VariableId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableLocation {
    Board(Square),
    Hand(Color),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariablePiece {
    pub id: VariableId,
    pub color: Color,
    pub kind: Kind,
    pub location: VariableLocation,
}

/// 覆面駒の正体をすべて確定させた一つの通常局面。
#[derive(Clone, Debug)]
pub struct ConcreteWorld {
    position: PositionAux,
    variables: BTreeMap<VariableId, VariablePiece>,
}

impl ConcreteWorld {
    pub(crate) fn new(position: PositionAux, variables: Vec<VariablePiece>) -> Self {
        Self {
            position,
            variables: variables
                .into_iter()
                .map(|piece| (piece.id, piece))
                .collect(),
        }
    }

    pub fn position(&self) -> &PositionAux {
        &self.position
    }

    pub fn variable(&self, id: VariableId) -> Option<&VariablePiece> {
        self.variables.get(&id)
    }

    pub fn variables(&self) -> impl Iterator<Item = &VariablePiece> {
        self.variables.values()
    }

    pub(crate) fn observed_variants(&self, movement: &Movement) -> Vec<ObservedMove> {
        match *movement {
            Movement::Move {
                source,
                dest,
                promote,
                ..
            } => {
                let identity = self
                    .variable_on_board(source)
                    .map_or(MoveIdentity::Known, |piece| {
                        MoveIdentity::Variable(piece.id)
                    });
                vec![ObservedMove::Move {
                    identity,
                    source,
                    destination: dest,
                    promote,
                }]
            }
            Movement::Drop(destination, kind) => {
                let turn = self.position.turn();
                let variables: Vec<_> = self
                    .variables
                    .values()
                    .filter(|piece| {
                        piece.color == turn
                            && piece.kind == kind
                            && piece.location == VariableLocation::Hand(turn)
                    })
                    .map(|piece| ObservedMove::Drop {
                        identity: DropIdentity::Variable(piece.id),
                        destination,
                    })
                    .collect();

                let variable_count = variables.len();
                let total_count = self.position.hands().count(turn, kind);
                let mut result = variables;
                if total_count > variable_count {
                    result.push(ObservedMove::Drop {
                        identity: DropIdentity::Known(kind),
                        destination,
                    });
                }
                result
            }
        }
    }

    pub(crate) fn apply(&self, observed: ObservedMove, movement: Movement) -> Self {
        let mover = self.position.turn();
        let captured_variable = match observed {
            ObservedMove::Move { destination, .. } => {
                self.variable_on_board(destination).map(|piece| piece.id)
            }
            ObservedMove::Drop { .. } => None,
        };

        let mut next = self.clone();
        next.position.do_move(&movement);

        if let Some(id) = captured_variable {
            let captured = next
                .variables
                .get_mut(&id)
                .expect("盤上で特定した覆面駒が存在する");
            captured.color = mover;
            captured.kind = captured.kind.maybe_unpromote();
            captured.location = VariableLocation::Hand(mover);
        }

        match observed {
            ObservedMove::Move {
                identity: MoveIdentity::Variable(id),
                destination,
                promote,
                ..
            } => {
                let moved = next
                    .variables
                    .get_mut(&id)
                    .expect("着手した覆面駒が存在する");
                if promote {
                    moved.kind = moved.kind.promote().expect("合法な成である");
                }
                moved.location = VariableLocation::Board(destination);
            }
            ObservedMove::Drop {
                identity: DropIdentity::Variable(id),
                destination,
            } => {
                let dropped = next.variables.get_mut(&id).expect("打った覆面駒が存在する");
                dropped.location = VariableLocation::Board(destination);
            }
            _ => {}
        }

        next
    }

    fn variable_on_board(&self, square: Square) -> Option<&VariablePiece> {
        self.variables
            .values()
            .find(|piece| piece.location == VariableLocation::Board(square))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_variable_demotes_changes_owner_and_can_be_dropped() {
        let mut position = PositionAux::default();
        position.set(Square::S54, Color::BLACK, Kind::Rook);
        position.set(Square::S55, Color::WHITE, Kind::ProSilver);
        position.set_turn(Color::BLACK);
        let world = ConcreteWorld::new(
            position,
            vec![VariablePiece {
                id: VariableId(3),
                color: Color::WHITE,
                kind: Kind::ProSilver,
                location: VariableLocation::Board(Square::S55),
            }],
        );

        let capture = ObservedMove::Move {
            identity: MoveIdentity::Known,
            source: Square::S54,
            destination: Square::S55,
            promote: false,
        };
        let captured = world.apply(
            capture,
            Movement::Move {
                source: Square::S54,
                source_kind_hint: None,
                dest: Square::S55,
                promote: false,
                capture_kind_hint: None,
            },
        );
        let piece = captured.variable(VariableId(3)).unwrap();
        assert_eq!(piece.color, Color::BLACK);
        assert_eq!(piece.kind, Kind::Silver);
        assert_eq!(piece.location, VariableLocation::Hand(Color::BLACK));

        let mut black_to_move = captured;
        black_to_move.position.set_turn(Color::BLACK);
        let dropped = black_to_move.apply(
            ObservedMove::Drop {
                identity: DropIdentity::Variable(VariableId(3)),
                destination: Square::S44,
            },
            Movement::Drop(Square::S44, Kind::Silver),
        );
        assert_eq!(
            dropped.variable(VariableId(3)).unwrap().location,
            VariableLocation::Board(Square::S44)
        );
    }
}
