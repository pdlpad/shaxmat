//! Playing games between levels and external UCI engines.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use shaxmat_core::{Chess, Color, Game, GameStatus, Move};
use shaxmat_engine::{Engine, LevelEngine, Limits, NativeClock, SearchPosition};

use crate::uci::UciEngine;

/// A game longer than this many plies is scored as a draw.
const MAX_PLIES: usize = 400;

/// Balanced openings, so that deterministic levels do not repeat one game.
const OPENINGS: [&str; 20] = [
    "e4 e5 Nf3 Nc6 Bb5 a6",
    "e4 e5 Nf3 Nc6 Bc4 Bc5",
    "e4 e5 f4 exf4 Nf3 g5",
    "e4 c5 Nf3 d6 d4 cxd4",
    "e4 c5 Nc3 Nc6 g3 g6",
    "e4 e6 d4 d5 Nc3 Nf6",
    "e4 e6 d4 d5 e5 c5",
    "e4 c6 d4 d5 e5 Bf5",
    "e4 c6 d4 d5 Nc3 dxe4",
    "e4 d6 d4 Nf6 Nc3 g6",
    "d4 d5 c4 e6 Nc3 Nf6",
    "d4 d5 c4 c6 Nf3 Nf6",
    "d4 d5 c4 dxc4 Nf3 Nf6",
    "d4 Nf6 c4 e6 Nc3 Bb4",
    "d4 Nf6 c4 g6 Nc3 Bg7",
    "d4 Nf6 c4 e6 Nf3 b6",
    "d4 f5 g3 Nf6 Bg2 e6",
    "c4 e5 Nc3 Nf6 Nf3 Nc6",
    "c4 c5 Nf3 Nf6 Nc3 Nc6",
    "Nf3 d5 g3 Nf6 Bg2 e6",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    Level(usize),
    Stockfish { elo: u32, movetime_ms: u64 },
}

enum Runner {
    Level(Box<LevelEngine>),
    Uci(UciEngine),
}

impl Runner {
    fn new(player: Player, seed: u64) -> Self {
        match player {
            Player::Level(level) => Runner::Level(Box::new(LevelEngine::new(
                level,
                seed,
                Box::new(NativeClock::default()),
            ))),
            Player::Stockfish { elo, movetime_ms } => {
                Runner::Uci(UciEngine::stockfish(elo, movetime_ms))
            }
        }
    }

    fn choose(&mut self, game: &Game) -> Option<Move> {
        match self {
            Runner::Level(engine) => {
                engine.best_move(&SearchPosition::from(game), &Limits::default())
            }
            Runner::Uci(engine) => engine.best_move(game),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MatchResult {
    pub wins: usize,
    pub draws: usize,
    pub losses: usize,
    pub max_move_ms: u64,
    pub total_plies: usize,
}

impl MatchResult {
    fn games(&self) -> usize {
        self.wins + self.draws + self.losses
    }

    /// The first player's share of the points.
    pub fn score(&self) -> f64 {
        (self.wins as f64 + self.draws as f64 / 2.0) / self.games().max(1) as f64
    }

    pub fn average_plies(&self) -> f64 {
        self.total_plies as f64 / self.games().max(1) as f64
    }
}

/// Elo difference implied by a score. A sweep is counted as half a game short of
/// perfect, to keep the value finite.
pub fn elo_difference(score: f64, games: usize) -> f64 {
    let margin = 0.5 / games.max(1) as f64;
    let score = score.clamp(margin, 1.0 - margin);
    400.0 * (score / (1.0 - score)).log10()
}

/// Plays `games` games on `threads` threads, the openings in turn, colors
/// alternating so that each opening is played from both sides.
pub fn run_match(a: Player, b: Player, games: usize, threads: usize, seed: u64) -> MatchResult {
    let next_game = AtomicUsize::new(0);
    let total = Mutex::new(MatchResult::default());
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                loop {
                    let index = next_game.fetch_add(1, Ordering::Relaxed);
                    if index >= games {
                        break;
                    }
                    let outcome = play_game(a, b, index, seed);
                    let mut total = total.lock().expect("no game thread panics");
                    match outcome.a_score {
                        Some(true) => total.wins += 1,
                        Some(false) => total.losses += 1,
                        None => total.draws += 1,
                    }
                    total.max_move_ms = total.max_move_ms.max(outcome.max_move_ms);
                    total.total_plies += outcome.plies;
                }
            });
        }
    });
    total.into_inner().expect("no game thread panics")
}

struct GameOutcome {
    /// `Some(true)` if the first player won, `None` for a draw.
    a_score: Option<bool>,
    max_move_ms: u64,
    plies: usize,
}

fn play_game(a: Player, b: Player, index: usize, seed: u64) -> GameOutcome {
    let opening = OPENINGS[(index / 2) % OPENINGS.len()];
    let a_color = if index.is_multiple_of(2) {
        Color::White
    } else {
        Color::Black
    };
    let mut game = Game::from_position(Chess::default());
    for san in opening.split_whitespace() {
        game.play_san(san).expect("openings are legal");
    }

    let game_seed = seed.wrapping_mul(1_000_003).wrapping_add(index as u64);
    let mut runner_a = Runner::new(a, game_seed.wrapping_mul(2));
    let mut runner_b = Runner::new(b, game_seed.wrapping_mul(2).wrapping_add(1));

    let mut max_move_ms = 0;
    while !game.status().is_over() && game.moves().len() < MAX_PLIES {
        let runner = if game.positions().len() % 2 == 1 {
            if a_color == Color::White {
                &mut runner_a
            } else {
                &mut runner_b
            }
        } else if a_color == Color::Black {
            &mut runner_a
        } else {
            &mut runner_b
        };
        let start = Instant::now();
        let mv = runner
            .choose(&game)
            .expect("a running game has a legal move");
        max_move_ms = max_move_ms.max(start.elapsed().as_millis() as u64);
        game.play(mv).expect("engines play legal moves");
    }

    let a_score = match game.status() {
        GameStatus::Checkmate { winner } => Some(winner == a_color),
        _ => None,
    };
    GameOutcome {
        a_score,
        max_move_ms,
        plies: game.moves().len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openings_are_legal() {
        for opening in OPENINGS {
            let mut game = Game::new();
            for san in opening.split_whitespace() {
                game.play_san(san).unwrap();
            }
        }
    }

    #[test]
    fn scores_and_elo() {
        let even = MatchResult {
            wins: 10,
            draws: 20,
            losses: 10,
            ..MatchResult::default()
        };
        assert_eq!(even.score(), 0.5);
        assert_eq!(elo_difference(0.5, 40), 0.0);
        assert!((elo_difference(0.75, 40) - 190.8).abs() < 0.1);
        assert!(elo_difference(1.0, 40).is_finite());
    }

    #[test]
    fn short_match_finishes() {
        let result = run_match(Player::Level(2), Player::Level(1), 4, 4, 1);
        assert_eq!(result.wins + result.draws + result.losses, 4);
    }
}
