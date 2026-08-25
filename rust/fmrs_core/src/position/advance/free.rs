use crate::{
    piece::{Kind, KINDS, NUM_HAND_KIND},
    position::{
        bitboard::{reachable, BitBoard},
        position::PositionAux,
        rule::{is_legal_drop, is_legal_move, promotable},
        Movement,
    },
};

/// 王手の有無にかかわらず、手番側の合法着手をすべて生成する。
///
/// 協力詰の受先初手など、通常の王手応手生成を使えない局面で使用する。
pub fn legal_movements(position: &PositionAux, result: &mut Vec<Movement>) {
    let turn = position.turn();
    let opponent_king = position.bitboard(turn.opposite(), Kind::King);

    for kind in KINDS {
        for source in position.bitboard(turn, kind) {
            let destinations =
                reachable(position, turn, source, kind, false).and_not(opponent_king);
            for dest in destinations {
                let capture_kind = position.get(dest).map(|(_, kind)| kind);
                if is_legal_move(turn, source, dest, kind, false) {
                    push_if_king_safe(
                        position,
                        Movement::move_with_hint(source, kind, dest, false, capture_kind),
                        result,
                    );
                }
                if kind.can_promote() && (promotable(source, turn) || promotable(dest, turn)) {
                    push_if_king_safe(
                        position,
                        Movement::move_with_hint(source, kind, dest, true, capture_kind),
                        result,
                    );
                }
            }
        }
    }

    let empty = BitBoard::FULL.and_not(position.occupied_bb());
    let mut pawn_mask = 0usize;
    for square in position.bitboard(turn, Kind::Pawn) {
        pawn_mask |= 1 << square.col();
    }
    for &kind in &KINDS[..NUM_HAND_KIND] {
        if position.hands().count(turn, kind) == 0 {
            continue;
        }
        for dest in empty {
            if is_legal_drop(turn, dest, kind, pawn_mask) {
                push_if_king_safe(position, Movement::Drop(dest, kind), result);
            }
        }
    }
}

fn push_if_king_safe(position: &PositionAux, movement: Movement, result: &mut Vec<Movement>) {
    let mover = position.turn();
    let mut next = position.clone();
    next.do_move(&movement);
    if !next.checked_slow(mover) {
        result.push(movement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_white_waiting_moves_without_initial_check() {
        let position = PositionAux::from_sfen("9/9/kS7/N8/1L7/9/9/9/9 w p 1").unwrap();
        let mut movements = Vec::new();

        legal_movements(&position, &mut movements);

        assert!(movements
            .iter()
            .any(|movement| matches!(movement, Movement::Drop(_, Kind::Pawn))));
    }
}
