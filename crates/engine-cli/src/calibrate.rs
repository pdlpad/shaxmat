//! Estimates the Elo of each level against Stockfish.
//!
//! `UCI_Elo` only goes down to 1320. Levels aimed up to `MAX_ANCHOR_GAP` below that
//! still play it directly; the rest are chained from the weakest level measured,
//! through self-play against the next level up. Self-play overstates differences,
//! so the chained estimates are lower bounds.

use std::ops::RangeInclusive;
use std::process::ExitCode;

use shaxmat_engine::{LEVEL_COUNT, level_params};

use crate::contest::{MatchResult, Player, elo_difference, run_match};
use crate::uci::STOCKFISH_ENV;

const MIN_UCI_ELO: u32 = 1320;
const MAX_UCI_ELO: u32 = 3190;
/// Beyond this gap a match gives too few points to estimate from.
const MAX_ANCHOR_GAP: u32 = 400;

pub fn run(
    levels: RangeInclusive<usize>,
    games: usize,
    threads: usize,
    movetime_ms: u64,
    seed: u64,
) -> ExitCode {
    if std::env::var_os(STOCKFISH_ENV).is_none() {
        println!("{STOCKFISH_ENV} is not set, skipping calibration.");
        return ExitCode::SUCCESS;
    }

    println!("| Level | Target | Opponent | Score | Estimated Elo |");
    println!("|---|---|---|---|---|");
    let mut estimates = [None; LEVEL_COUNT + 1];
    let direct = levels
        .clone()
        .filter(|&l| level_params(l).target_elo + MAX_ANCHOR_GAP >= MIN_UCI_ELO);
    for level in direct {
        let target = level_params(level).target_elo;
        let anchor = target.clamp(MIN_UCI_ELO, MAX_UCI_ELO);
        let opponent = Player::Stockfish {
            elo: anchor,
            movetime_ms,
        };
        let result = run_match(Player::Level(level), opponent, games, threads, seed);
        let estimate = anchor as f64 + elo_difference(result.score(), games);
        estimates[level] = Some(estimate);
        print_row(level, &format!("Stockfish {anchor}"), &result, estimate);
    }

    for level in levels.rev() {
        if estimates[level].is_some() {
            continue;
        }
        let Some(upper) = estimates.get(level + 1).copied().flatten() else {
            continue;
        };
        let result = run_match(
            Player::Level(level),
            Player::Level(level + 1),
            games,
            threads,
            seed,
        );
        let estimate = upper + elo_difference(result.score(), games);
        estimates[level] = Some(estimate);
        print_row(level, &format!("L{}", level + 1), &result, estimate);
    }
    ExitCode::SUCCESS
}

fn print_row(level: usize, opponent: &str, result: &MatchResult, estimate: f64) {
    println!(
        "| {level} | {} | {opponent} | {:.1}% | {estimate:.0} |",
        level_params(level).target_elo,
        result.score() * 100.0,
    );
}
