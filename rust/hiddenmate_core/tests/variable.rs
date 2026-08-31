use fmrs_core::{
    piece::{Color, Kind},
    position::Square,
};
use hiddenmate_core::{
    format_solution_japanese, solve_exact, solve_replay_exact, DropIdentity, HandVariableMode,
    MateRule, MoveIdentity, ObservedMove, VariableId, VariableLocation, VariableProblem,
    VariableSpec,
};

fn square(file: usize, rank: usize) -> Square {
    Square::new(file - 1, rank - 1)
}

#[test]
fn solution_count_ignores_variable_ids_in_both_hand_modes() {
    let problem = VariableProblem {
        base_sfen: "9/9/kS7/9/1L7/9/9/9/9 b 2r2b4g3s4n3l18p 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Hand(Color::BLACK),
                candidates: fmrs_core::piece::KINDS[..7].to_vec(),
            },
            VariableSpec {
                id: VariableId(2),
                color: Color::BLACK,
                location: VariableLocation::Hand(Color::BLACK),
                candidates: fmrs_core::piece::KINDS[..7].to_vec(),
            },
        ],
    };

    for state in [
        problem
            .clone()
            .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
            .unwrap(),
        problem
            .enumerate_with_hand_variable_mode(HandVariableMode::Indistinguishable)
            .unwrap(),
    ] {
        let solutions = solve_exact(&state, 1, 100);
        assert_eq!(solutions.len(), 3);
        assert_eq!(
            solutions
                .iter()
                .map(|solution| format_solution_japanese(&state, solution))
                .collect::<Vec<_>>(),
            vec![vec!["82▲打"], vec!["92▲打"], vec!["94▲打"]]
        );
    }
}

#[test]
fn indistinguishable_hand_variables_branch_then_resolve_which_one_remains() {
    // V1（金・銀）とV2（飛・香）を区別せず55へ打つ。54玉から43玉の後、
    // 55の覆面駒を44へ動かせるのはV1だけなので、駒台に残った駒がV2と確定する。
    let problem = VariableProblem {
        base_sfen: "9/9/9/4k4/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Hand(Color::BLACK),
                candidates: vec![Kind::Gold, Kind::Silver],
            },
            VariableSpec {
                id: VariableId(2),
                color: Color::BLACK,
                location: VariableLocation::Hand(Color::BLACK),
                candidates: vec![Kind::Rook, Kind::Lance],
            },
        ],
    };

    assert_eq!(
        problem.clone().enumerate().unwrap().hand_variable_mode(),
        HandVariableMode::Indistinguishable
    );

    let distinguishable = problem
        .clone()
        .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
        .unwrap();
    let distinguishable_drops = distinguishable
        .observed_moves()
        .into_iter()
        .filter(|observed| {
            matches!(
                observed,
                ObservedMove::Drop {
                    destination: Square::S55,
                    ..
                }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(distinguishable_drops.len(), 2);
    assert!(distinguishable_drops.iter().any(|observed| matches!(
        observed,
        ObservedMove::Drop {
            identity: DropIdentity::Variable(VariableId(1)),
            ..
        }
    )));
    assert!(distinguishable_drops.iter().any(|observed| matches!(
        observed,
        ObservedMove::Drop {
            identity: DropIdentity::Variable(VariableId(2)),
            ..
        }
    )));

    let state = problem
        .enumerate_with_hand_variable_mode(HandVariableMode::Indistinguishable)
        .unwrap();
    let anonymous_drop = ObservedMove::Drop {
        identity: DropIdentity::AnonymousVariable,
        destination: Square::S55,
    };
    assert!(state.observed_moves().contains(&anonymous_drop));

    let after_drop = state.apply(anonymous_drop).unwrap();
    assert_eq!(after_drop.world_count(), 8);
    assert!(after_drop.worlds().iter().any(|world| {
        world.variable(VariableId(1)).unwrap().location == VariableLocation::Board(Square::S55)
    }));
    assert!(after_drop.worlds().iter().any(|world| {
        world.variable(VariableId(2)).unwrap().location == VariableLocation::Board(Square::S55)
    }));

    let king_move = ObservedMove::Move {
        identity: MoveIdentity::Known,
        source: Square::S54,
        destination: Square::S43,
        promote: false,
    };
    let after_king_move = after_drop.apply(king_move).unwrap();
    let anonymous_move = ObservedMove::Move {
        identity: MoveIdentity::AnonymousVariable,
        source: Square::S55,
        destination: Square::S44,
        promote: false,
    };
    let inferred = after_king_move.apply(anonymous_move).unwrap();

    assert_eq!(inferred.world_count(), 4);
    assert_eq!(
        inferred.candidates(VariableId(2)),
        [Kind::Lance, Kind::Rook].into_iter().collect()
    );
    assert!(inferred.worlds().iter().all(|world| {
        world.variable(VariableId(2)).unwrap().location == VariableLocation::Hand(Color::BLACK)
    }));
    assert_eq!(
        format_solution_japanese(&state, &vec![anonymous_drop, king_move, anonymous_move]),
        vec!["55▲打", "43玉(54)", "44▲(55)"]
    );
}

#[test]
fn rejects_more_than_six_variables() {
    let variables = (1..=7)
        .map(|id| VariableSpec {
            id: VariableId(id),
            color: Color::BLACK,
            location: VariableLocation::Hand(Color::BLACK),
            candidates: vec![Kind::Pawn],
        })
        .collect();

    let error = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables,
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap_err();

    assert!(error.to_string().contains("覆面駒は6枚まで指定できます"));
}

#[test]
fn enumerates_candidate_worlds_from_piece_box() {
    let state = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Board(square(6, 4)),
            candidates: vec![Kind::Rook, Kind::ProRook],
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();

    assert_eq!(state.world_count(), 2);
    assert_eq!(
        state.candidates(VariableId(1)),
        [Kind::Rook, Kind::ProRook].into_iter().collect()
    );
    for world in state.worlds() {
        assert_eq!(world.position().hands().count(Color::WHITE, Kind::Rook), 0);
        assert_eq!(world.position().hands().count(Color::WHITE, Kind::Pawn), 0);
    }
}

#[test]
fn default_and_explicit_backends_match_worlds_moves_and_solutions() {
    let problem = VariableProblem {
        base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Board(square(6, 4)),
            candidates: vec![Kind::Rook, Kind::ProRook, Kind::Rook],
        }],
    };

    let default = problem.clone().enumerate().unwrap();
    let explicit = problem.enumerate_explicit().unwrap();

    assert_eq!(default.world_count(), explicit.world_count());
    assert_eq!(default.all_candidates(), explicit.all_candidates());
    assert_eq!(default.observed_moves(), explicit.observed_moves());
    assert_eq!(
        solve_exact(&default, 3, 100),
        solve_exact(&explicit, 3, 100)
    );
}

#[test]
fn profiled_enumeration_and_solve_report_candidate_counts() {
    let problem = VariableProblem {
        base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Board(square(6, 4)),
            candidates: vec![Kind::Rook, Kind::ProRook],
        }],
    };

    let (state, enumeration) = problem.enumerate_profiled().unwrap();
    let (solutions, solve_metrics) = hiddenmate_core::solve_exact_profiled(&state, 1, 10);

    assert_eq!(enumeration.world_count, state.world_count());
    assert_eq!(solutions, solve_exact(&state, 1, 10));
    assert_eq!(solve_metrics.initial_world_count, state.world_count());
    assert!(solve_metrics.visited_state_count > 0);
    assert!(solve_metrics.generated_transition_count > 0);
    assert!(solve_metrics.peak_world_count >= state.world_count());
}

#[test]
fn variable_can_come_from_white_hand_or_piece_box() {
    let state = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b r 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Board(square(6, 4)),
            candidates: vec![Kind::Rook],
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();

    assert_eq!(state.world_count(), 2);
    let remaining_rooks = state
        .worlds()
        .iter()
        .map(|world| world.position().hands().count(Color::WHITE, Kind::Rook))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(remaining_rooks, [0, 1].into_iter().collect());
}

#[test]
fn checking_obligation_resolves_rook_to_promoted_rook() {
    // 64の覆面駒を84へ動かす。93玉に対して、飛では王手にならず、
    // 龍なら斜め一歩の利きで王手になる。
    let state = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(7),
            color: Color::BLACK,
            location: VariableLocation::Board(square(6, 4)),
            candidates: vec![Kind::Rook, Kind::ProRook],
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
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
fn allows_initial_check_against_attacker_king() {
    // 攻方手番では、攻方玉に王手が掛かっている初形も合法。
    let state = VariableProblem {
        base_sfen: "8k/7g1/8K/9/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();

    assert_eq!(state.world_count(), 1);
}

#[test]
fn resolved_variable_is_ordinary_before_its_next_move() {
    let state = VariableProblem {
        base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Board(square(6, 4)),
            candidates: vec![Kind::ProRook],
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();

    assert_eq!(state.resolved_kind(VariableId(1)), Some(Kind::ProRook));
    let solutions = solve_exact(&state, 1, 10);
    assert_eq!(solutions.len(), 1);
    assert!(matches!(
        solutions[0][0],
        ObservedMove::Move {
            identity: MoveIdentity::Known,
            ..
        }
    ));
    assert_eq!(
        format_solution_japanese(&state, &solutions[0]),
        vec!["84龍(64)"]
    );
}

#[test]
fn solves_one_ply_variable_helpmate() {
    // 83銀が82・92を塞ぎ、85香が84の龍を守る。94桂は余分な
    // 64V-94を防ぐため、64V-84だけで93玉が詰む。
    let state = VariableProblem {
        base_sfen: "9/9/kS7/N8/1L7/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Board(square(6, 4)),
            candidates: vec![Kind::Rook, Kind::ProRook],
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();

    let solutions = solve_exact(&state, 1, 10);
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].len(), 1);
    assert_eq!(solutions[0][0].to_string(), "V1:64-84");
    assert_eq!(
        format_solution_japanese(&state, &solutions[0]),
        vec!["84▲(64)"]
    );

    let final_state = state.apply(solutions[0][0]).unwrap();
    assert!(final_state.is_proven_mate());
    assert_eq!(
        final_state.resolved_kind(VariableId(1)),
        Some(Kind::ProRook)
    );

    // 3手指定でも、上限以下の1手解を含める。
    assert!(solve_exact(&state, 3, 10)
        .iter()
        .any(|solution| solution.len() == 1));
}

#[test]
fn rejects_duplicate_variable_ids() {
    let error = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Board(square(6, 4)),
                candidates: vec![Kind::Rook],
            },
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Board(square(7, 4)),
                candidates: vec![Kind::Bishop],
            },
        ],
    }
    .enumerate()
    .unwrap_err();

    assert!(error.to_string().contains("重複"));
}

#[test]
fn enumerates_variable_in_hand_and_can_observe_its_drop() {
    let state = VariableProblem {
        base_sfen: "9/9/k8/9/9/9/9/9/9 b - 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(4),
            color: Color::BLACK,
            location: VariableLocation::Hand(Color::BLACK),
            candidates: fmrs_core::piece::KINDS.to_vec(),
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();

    assert_eq!(state.world_count(), 7);
    assert!(state.worlds().iter().all(|world| {
        world.variable(VariableId(4)).unwrap().location == VariableLocation::Hand(Color::BLACK)
    }));
    assert!(state.observed_moves().iter().any(|observed| matches!(
        observed,
        ObservedMove::Drop {
            identity: hiddenmate_core::DropIdentity::Variable(VariableId(4)),
            ..
        }
    )));
}

#[test]
fn pawn_drop_mate_world_is_removed_from_a_variable_drop() {
    // 94への覆面駒打ちは、歩なら打歩詰めで不合法だが、香・銀・金・飛なら合法な詰み。
    // 観測した94▲打から歩の世界だけを除外し、解として成立させる。
    let state = VariableProblem {
        base_sfen: "9/9/kS7/9/1L7/9/9/9/9 b 2r2b4g3s4n3l18p 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Hand(Color::BLACK),
            candidates: fmrs_core::piece::KINDS.to_vec(),
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();
    let drop_94 = ObservedMove::Drop {
        identity: DropIdentity::Variable(VariableId(1)),
        destination: square(9, 4),
    };

    let next = state.apply(drop_94).unwrap();
    assert_eq!(
        next.candidates(VariableId(1)),
        [Kind::Lance, Kind::Silver, Kind::Gold, Kind::Rook]
            .into_iter()
            .collect()
    );
    assert!(next.is_proven_mate());

    let solutions = solve_exact(&state, 1, 10);
    let formatted = solutions
        .iter()
        .map(|solution| format_solution_japanese(&state, solution).join(" "))
        .collect::<Vec<_>>();
    assert_eq!(formatted, vec!["82▲打", "92▲打", "94▲打"]);
}

#[test]
fn defender_pawn_block_is_not_filtered_as_pawn_drop_mate() {
    // 55飛の王手に35歩合。55飛は59飛にピンされているため35歩を取れず、
    // 攻方に次の王手がなくても、受方の35歩合自体は合法である。
    let state = VariableProblem {
        base_sfen: "4K4/9/9/9/4R3k/9/9/9/4r4 w p 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![],
    }
    .enumerate()
    .unwrap();
    let pawn_block = ObservedMove::Drop {
        identity: DropIdentity::Known(Kind::Pawn),
        destination: square(3, 5),
    };

    assert!(state.observed_moves().contains(&pawn_block));
}

#[test]
fn includes_valid_three_ply_line_even_when_shorter_mates_exist() {
    let state = VariableProblem {
        base_sfen: "9/9/kS7/N8/9/1L7/9/9/9 b p 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![VariableSpec {
            id: VariableId(1),
            color: Color::BLACK,
            location: VariableLocation::Board(square(4, 2)),
            candidates: fmrs_core::piece::KINDS.to_vec(),
        }],
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();

    let first = ObservedMove::Move {
        identity: MoveIdentity::Variable(VariableId(1)),
        source: square(4, 2),
        destination: square(7, 5),
        promote: true,
    };
    assert!(state.observed_moves().contains(&first));
    let after_first = state.apply(first).unwrap();
    assert_eq!(
        after_first.resolved_kind(VariableId(1)),
        Some(Kind::ProBishop)
    );
    let second = ObservedMove::Drop {
        identity: DropIdentity::Known(Kind::Pawn),
        destination: square(8, 4),
    };
    assert!(after_first.observed_moves().contains(&second));
    let after_second = after_first.apply(second).unwrap();
    let third = ObservedMove::Move {
        identity: MoveIdentity::Known,
        source: square(7, 5),
        destination: square(8, 4),
        promote: false,
    };
    assert!(after_second.observed_moves().contains(&third));
    assert!(after_second.apply(third).unwrap().is_proven_mate());

    let shorter = solve_exact(&state, 1, 100)
        .iter()
        .map(|solution| format_solution_japanese(&state, solution).join(" "))
        .collect::<Vec<_>>();

    assert_eq!(
        shorter,
        vec!["82▲(42)", "82▲成(42)", "92▲(42)", "92▲成(42)"]
    );
    let up_to_three = solve_exact(&state, 3, 100)
        .iter()
        .map(|solution| format_solution_japanese(&state, solution).join(" "))
        .collect::<Vec<_>>();
    assert!(up_to_three
        .iter()
        .any(|solution| solution == "75▲成(42) 84歩打 同馬(75)"));
}

#[test]
fn solves_known_help_selfmate_up_to_four_plies() {
    let state = VariableProblem {
        base_sfen: "9/9/9/9/7l1/9/8k/9/7SK b G 1".to_string(),
        variables: vec![],
        rule: MateRule::HelpSelfmate,
    }
    .enumerate()
    .unwrap();

    let solutions = solve_exact(&state, 4, 100);

    assert_eq!(
        solutions.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![2, 4, 4, 4]
    );
}

#[test]
fn help_selfmate_requires_exactly_one_attacker_king_in_every_world() {
    let error = VariableProblem {
        base_sfen: "9/9/9/9/7l1/9/8k/9/8S b G 1".to_string(),
        variables: vec![],
        rule: MateRule::HelpSelfmate,
    }
    .enumerate()
    .unwrap_err();

    assert!(error.to_string().contains("割当がありません"));
}

#[test]
fn white_start_help_selfmate_allows_a_free_defender_move() {
    let state = VariableProblem {
        base_sfen: "9/9/9/9/9/9/8k/9/9 w - 1".to_string(),
        variables: vec![
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Board(square(2, 9)),
                candidates: fmrs_core::piece::KINDS.to_vec(),
            },
            VariableSpec {
                id: VariableId(2),
                color: Color::BLACK,
                location: VariableLocation::Board(square(1, 9)),
                candidates: fmrs_core::piece::KINDS.to_vec(),
            },
        ],
        rule: MateRule::HelpSelfmate,
    }
    .enumerate()
    .unwrap();

    assert_eq!(state.world_count(), 26);
    assert_eq!(
        state.candidates(VariableId(1)),
        fmrs_core::piece::KINDS.into_iter().collect()
    );
    assert_eq!(
        state.candidates(VariableId(2)),
        fmrs_core::piece::KINDS.into_iter().collect()
    );
    assert!(state.worlds().iter().any(|world| {
        world.variable(VariableId(1)).unwrap().kind == Kind::Pawn
            && world.variable(VariableId(2)).unwrap().kind == Kind::King
    }));
}

#[test]
fn japanese_notation_uses_same_and_nonpromotion() {
    let state = VariableProblem {
        base_sfen: "9/9/9/9/9/9/8k/9/9 w r 1".to_string(),
        variables: vec![
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Board(Square::S49),
                candidates: fmrs_core::piece::KINDS.to_vec(),
            },
            VariableSpec {
                id: VariableId(2),
                color: Color::BLACK,
                location: VariableLocation::Board(Square::S19),
                candidates: fmrs_core::piece::KINDS.to_vec(),
            },
        ],
        rule: MateRule::HelpSelfmate,
    }
    .enumerate_with_hand_variable_mode(HandVariableMode::Distinguishable)
    .unwrap();
    let solution = vec![
        ObservedMove::Drop {
            identity: DropIdentity::Known(Kind::Rook),
            destination: Square::S31,
        },
        ObservedMove::Move {
            identity: MoveIdentity::Variable(VariableId(1)),
            source: Square::S49,
            destination: Square::S39,
            promote: false,
        },
        ObservedMove::Move {
            identity: MoveIdentity::Known,
            source: Square::S31,
            destination: Square::S39,
            promote: false,
        },
    ];

    assert_eq!(
        format_solution_japanese(&state, &solution),
        vec!["31飛打", "39▲(49)", "同飛生(31)"]
    );
}

#[test]
fn replay_backend_matches_explicit_backend() {
    let problem = VariableProblem {
        base_sfen: "9/9/kS7/9/1L7/9/9/9/9 b 2r2b4g3s4n3l18p 1".to_string(),
        rule: MateRule::Helpmate,
        variables: vec![
            VariableSpec {
                id: VariableId(1),
                color: Color::BLACK,
                location: VariableLocation::Hand(Color::BLACK),
                candidates: fmrs_core::piece::KINDS[..7].to_vec(),
            },
            VariableSpec {
                id: VariableId(2),
                color: Color::BLACK,
                location: VariableLocation::Hand(Color::BLACK),
                candidates: fmrs_core::piece::KINDS[..7].to_vec(),
            },
        ],
    };
    for mode in [
        HandVariableMode::Indistinguishable,
        HandVariableMode::Distinguishable,
    ] {
        let explicit = problem
            .clone()
            .enumerate_explicit_with_hand_variable_mode(mode)
            .unwrap();
        let (replay, enumeration) = problem
            .clone()
            .enumerate_replay_profiled_with_hand_variable_mode(mode)
            .unwrap();

        assert_eq!(replay.world_count(), explicit.world_count());
        assert_eq!(enumeration.world_count, explicit.world_count());
        assert_eq!(replay.turn(), explicit.turn());
        assert_eq!(replay.hand_variable_mode(), explicit.hand_variable_mode());
        assert_eq!(replay.observed_moves().unwrap(), explicit.observed_moves());
        for id in [VariableId(1), VariableId(2)] {
            assert_eq!(replay.candidates(id).unwrap(), explicit.candidates(id));
        }
        assert_eq!(
            solve_replay_exact(&replay, 1, 100).unwrap(),
            solve_exact(&explicit, 1, 100)
        );

        for observed in explicit.observed_moves() {
            let expected = explicit.apply(observed).unwrap();
            let actual = replay.apply(observed).unwrap().unwrap();
            assert_eq!(actual.world_count(), expected.world_count(), "{observed:?}");
            assert_eq!(actual.turn(), expected.turn(), "{observed:?}");
            assert_eq!(actual.is_proven_mate().unwrap(), expected.is_proven_mate());
            for id in [VariableId(1), VariableId(2)] {
                assert_eq!(actual.candidates(id).unwrap(), expected.candidates(id));
            }
        }
    }
}

#[test]
fn replay_backend_matches_free_white_help_selfmate() {
    let problem = VariableProblem {
        base_sfen: "9/9/9/9/7l1/9/8k/9/7SK w G 1".to_string(),
        variables: vec![],
        rule: MateRule::HelpSelfmate,
    };
    let explicit = problem.clone().enumerate_explicit().unwrap();
    let replay = problem.enumerate_replay().unwrap();

    assert_eq!(replay.observed_moves().unwrap(), explicit.observed_moves());
    assert_eq!(
        solve_replay_exact(&replay, 4, 100).unwrap(),
        solve_exact(&explicit, 4, 100)
    );
}
