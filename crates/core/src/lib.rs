//! Chess rules, notation and game history for SHAXMAT.
//!
//! A thin layer over [`shakmaty`]: this crate adds what a playable game needs on
//! top of move generation (history, draw detection, PGN). It knows nothing about
//! graphics or input and uses neither threads nor the file system.

mod fen;
mod game;
mod pgn;

pub use fen::{parse_fen, to_fen};
pub use game::{Game, GameError, GameStatus};
pub use pgn::PgnTags;
pub use shakmaty::{self, Chess, Color, Move, MoveList, Position, Role, Square, perft};
