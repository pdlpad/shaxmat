//! Chess search and evaluation for SHAXMAT.
//!
//! Negamax with alpha-beta (principal variation search), iterative deepening,
//! quiescence search, a transposition table, MVV-LVA with killers and history for
//! ordering, null-move pruning and late move reductions. Uses no threads or file
//! system, and reads time only through [`Clock`], so it runs on
//! `wasm32-unknown-unknown`.

mod eval;
mod level;
mod rng;
mod search;
mod tt;

use shaxmat_core::shakmaty::zobrist::Zobrist64;
use shaxmat_core::{Chess, Game, Move};

pub use eval::evaluate;
pub use level::{LEVEL_COUNT, LevelEngine, LevelParams, level_params};
pub use search::{MATE, MATE_BOUND, Searcher};

/// A position to search, with the game so far for repetition detection.
#[derive(Debug, Clone)]
pub struct SearchPosition {
    pub position: Chess,
    /// Repetition keys of the earlier positions, oldest first; the last one is
    /// the position itself.
    pub history: Vec<Zobrist64>,
}

impl From<&Game> for SearchPosition {
    fn from(game: &Game) -> Self {
        SearchPosition {
            position: game.position().clone(),
            history: game.repetition_keys().to_vec(),
        }
    }
}

impl From<&Chess> for SearchPosition {
    fn from(position: &Chess) -> Self {
        SearchPosition::from(&Game::from_position(position.clone()))
    }
}

/// When to stop searching. The first limit reached wins. With none set the search
/// stops at [`Limits::DEFAULT_DEPTH`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Limits {
    pub depth: Option<u32>,
    pub time_ms: Option<u64>,
    pub nodes: Option<u64>,
    /// How many best moves [`Engine::analyze`] returns; 0 means all of them.
    pub top_n: usize,
}

impl Limits {
    pub const DEFAULT_DEPTH: u32 = 6;

    pub fn depth(depth: u32) -> Self {
        Limits {
            depth: Some(depth),
            ..Limits::default()
        }
    }

    pub fn time_ms(time_ms: u64) -> Self {
        Limits {
            time_ms: Some(time_ms),
            ..Limits::default()
        }
    }

    pub fn with_top_n(self, top_n: usize) -> Self {
        Limits { top_n, ..self }
    }
}

/// A move with its score in centipawns from the mover's point of view. Scores
/// beyond [`MATE_BOUND`] in absolute value mean a forced mate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoredMove {
    pub mv: Move,
    pub score: i32,
}

pub trait Engine {
    /// The move to play, or `None` if there are no legal moves.
    fn best_move(&mut self, position: &SearchPosition, limits: &Limits) -> Option<Move>;

    /// The best `limits.top_n` moves, best first.
    fn analyze(&mut self, position: &SearchPosition, limits: &Limits) -> Vec<ScoredMove>;
}

/// Monotonic time source, supplied by the host: `std::time::Instant` panics on
/// `wasm32-unknown-unknown`.
pub trait Clock: Send {
    fn now_ms(&self) -> u64;
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_clock::NativeClock;

#[cfg(not(target_arch = "wasm32"))]
mod native_clock {
    use std::time::Instant;

    use crate::Clock;

    pub struct NativeClock(Instant);

    impl Default for NativeClock {
        fn default() -> Self {
            NativeClock(Instant::now())
        }
    }

    impl Clock for NativeClock {
        fn now_ms(&self) -> u64 {
            self.0.elapsed().as_millis() as u64
        }
    }
}
