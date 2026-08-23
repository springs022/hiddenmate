use fmrs_core::{
    piece::{Color, Kind},
    position::Square,
};
use hiddenmate_core::{
    solve_exact, MoveIdentity, ObservedMove, VariableId, VariableProblem, VariableSpec,
};

fn square(file: usize, rank: usize) -> Square {
    Square::new(file - 1, rank - 1)
}

#[test]
fn enumerates_candidate_worlds_and_completes_white_hand() {
    let state = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            square: square(6, 4),
            candidates: vec![Kind::Rook, Kind::ProRook],
        }],
    }
    .enumerate()
    .unwrap();

    assert_eq!(state.world_count(), 2);
    assert_eq!(
        state.candidates(VariableId(1)),
        [Kind::Rook, Kind::ProRook].into_iter().collect()
    );
    for world in state.worlds() {
        // 覆面駒が飛または龍なので、もう一枚の飛車が受方持駒になる。
        assert_eq!(world.position().hands().count(Color::WHITE, Kind::Rook), 1);
        assert_eq!(world.position().hands().count(Color::WHITE, Kind::Pawn), 18);
    }
}

#[test]
fn checking_obligation_resolves_rook_to_promoted_rook() {
    // 64の覆面駒を84へ動かす。93玉に対して、飛では王手にならず、
    // 龍なら斜め一歩の利きで王手になる。
    let state = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        variables: vec![VariableSpec {
            id: VariableId(7),
            color: Color::BLACK,
            square: square(6, 4),
            candidates: vec![Kind::Rook, Kind::ProRook],
        }],
    }
    .enumerate()
    .unwrap();

    let observed = ObservedMove::Move {
        identity: MoveIdentity::Variable(VariableId(7)),
        source: square(6, 4),
        destination: square(8, 4),
        promote: false,
    };
    assert!(state.observed_moves().contains(&observed));

    let next = state.apply(observed).unwrap();
    assert_eq!(next.world_count(), 1);
    assert_eq!(next.resolved_kind(VariableId(7)), Some(Kind::ProRook));
}

#[test]
fn solves_one_ply_variable_helpmate() {
    // 83銀が82・92を塞ぎ、85香が84の龍を守る。94桂は余分な
    // 64V-94を防ぐため、64V-84だけで93玉が詰む。
    let state = VariableProblem {
        base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b - 1".to_string(),
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            square: square(6, 4),
            candidates: vec![Kind::Rook, Kind::ProRook],
        }],
    }
    .enumerate()
    .unwrap();

    let solutions = solve_exact(&state, 1, 10);
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].len(), 1);
    assert_eq!(solutions[0][0].to_string(), "V1:64-84");

    let final_state = state.apply(solutions[0][0]).unwrap();
    assert!(final_state.is_proven_mate());
    assert_eq!(
        final_state.resolved_kind(VariableId(1)),
        Some(Kind::ProRook)
    );
}

#[test]
fn rejects_duplicate_variable_ids() {
    let error = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        variables: vec![
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                square: square(6, 4),
                candidates: vec![Kind::Rook],
            },
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                square: square(7, 4),
                candidates: vec![Kind::Bishop],
            },
        ],
    }
    .enumerate()
    .unwrap_err();

    assert!(error.to_string().contains("重複"));
}
