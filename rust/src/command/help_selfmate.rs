use fmrs_core::{
    position::position::PositionAux,
    sfen,
    solve::help_selfmate::{help_selfmate_solutions_within, help_selfmate_solve},
    solve::Solution,
};

use super::parse_to_sfen;

pub fn help_selfmate(
    sfen_or_file_or_url: Option<String>,
    plies: Option<usize>,
    solutions_upto: usize,
) -> anyhow::Result<()> {
    let sfen_string = match sfen_or_file_or_url {
        Some(value) => parse_to_sfen(&value)?,
        None => {
            eprintln!("SFENを入力してください");
            eprint!("> ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            parse_to_sfen(input.trim())?
        }
    };
    let position = PositionAux::from_sfen(&sfen_string)
        .map_err(|_| anyhow::anyhow!("SFENの読み込みに失敗しました"))?;

    let solutions = if let Some(plies) = plies {
        help_selfmate_solutions_within(position, plies, solutions_upto)?
    } else {
        help_selfmate_solve(position, solutions_upto, false)?.solutions()
    };
    if solutions.is_empty() {
        eprintln!("解なし");
        return Ok(());
    }

    if let Some(plies) = plies {
        eprintln!(
            "{}手以下で{}解{}",
            plies,
            solutions.len(),
            capped_suffix(solutions.len(), solutions_upto)
        );
    } else {
        eprintln!(
            "{}手で解決（{}解{}）",
            solutions[0].len(),
            solutions.len(),
            capped_suffix(solutions.len(), solutions_upto)
        );
    }
    print_solutions(&sfen_string, solutions);
    Ok(())
}

fn capped_suffix(solution_count: usize, solutions_upto: usize) -> &'static str {
    if solution_count == solutions_upto {
        "以上"
    } else {
        ""
    }
}

fn print_solutions(sfen_string: &str, solutions: Vec<Solution>) {
    for solution in solutions {
        print!("position {} moves", sfen_string.trim());
        for movement in solution {
            print!(" {}", sfen::encode_move(&movement));
        }
        println!();
    }
}
