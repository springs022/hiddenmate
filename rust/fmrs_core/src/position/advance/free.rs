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

/// 手番側の玉が通常の意味で詰んでいるかを返す。
///
/// `legal_movements` は高速な着手生成を優先して打歩詰めの歩打ちも一旦生成するため、
/// ここではその歩打ちだけを再帰的に除外してから合法応手の有無を判定する。
/// 歩打ちを再帰するたびに手駒の歩が1枚減るので、この再帰は有限である。
pub fn is_legal_mate(position: &mut PositionAux) -> bool {
    let turn = position.turn();
    if !position.checked_slow(turn) {
        return false;
    }

    !has_legal_movement(position)
}

fn has_legal_movement(position: &PositionAux) -> bool {
    let mut movements = Vec::new();
    legal_movements(position, &mut movements);

    movements.into_iter().any(|movement| {
        if !movement.is_pawn_drop() {
            return true;
        }

        let mut next = position.clone();
        next.do_move(&movement);
        !is_legal_mate(&mut next)
    })
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

    #[test]
    fn detects_mate_for_either_color() {
        let mut black_mated = PositionAux::from_sfen("7rK/7g1/9/9/9/9/9/9/4k4 b - 1").unwrap();
        assert!(is_legal_mate(&mut black_mated));

        let mut white_mated = PositionAux::from_sfen("4K4/9/9/9/9/9/9/7G1/7Rk w - 1").unwrap();
        assert!(is_legal_mate(&mut white_mated));
    }
}
