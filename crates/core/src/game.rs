use std::fmt;

use shakmaty::san::SanPlus;
use shakmaty::zobrist::Zobrist64;
use shakmaty::{Chess, Color, EnPassantMode, Move, MoveList, Position};

use crate::parse_fen;

/// Halfmove clock value at which the fifty-move rule ends the game.
const FIFTY_MOVE_HALFMOVES: u32 = 100;
const THREEFOLD: usize = 3;

/// Why a game is over, or that it is not.
///
/// Draws by repetition and the fifty-move rule end the game automatically,
/// without a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    InProgress,
    Checkmate { winner: Color },
    Stalemate,
    InsufficientMaterial,
    FiftyMoves,
    ThreefoldRepetition,
}

impl GameStatus {
    pub fn is_over(self) -> bool {
        self != GameStatus::InProgress
    }

    /// The PGN result token.
    pub fn result(self) -> &'static str {
        match self {
            GameStatus::InProgress => "*",
            GameStatus::Checkmate {
                winner: Color::White,
            } => "1-0",
            GameStatus::Checkmate {
                winner: Color::Black,
            } => "0-1",
            _ => "1/2-1/2",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    InvalidFen(String),
    InvalidSan(String),
    IllegalMove,
    GameOver,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::InvalidFen(reason) => write!(f, "invalid FEN: {reason}"),
            GameError::InvalidSan(san) => write!(f, "invalid SAN: {san}"),
            GameError::IllegalMove => f.write_str("illegal move"),
            GameError::GameOver => f.write_str("game is over"),
        }
    }
}

impl std::error::Error for GameError {}

/// A game from a starting position: the current position, the moves played and
/// the resulting status.
#[derive(Debug, Clone)]
pub struct Game {
    /// Every position reached; the first is the start, the last is the current one.
    positions: Vec<Chess>,
    /// Repetition keys, parallel to `positions`.
    keys: Vec<Zobrist64>,
    moves: Vec<Move>,
    sans: Vec<SanPlus>,
    status: GameStatus,
}

impl Default for Game {
    fn default() -> Self {
        Self::from_position(Chess::default())
    }
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_position(position: Chess) -> Self {
        let mut game = Game {
            keys: vec![repetition_key(&position)],
            positions: vec![position],
            moves: Vec::new(),
            sans: Vec::new(),
            status: GameStatus::InProgress,
        };
        game.status = game.compute_status();
        game
    }

    pub fn from_fen(fen: &str) -> Result<Self, GameError> {
        parse_fen(fen).map(Self::from_position)
    }

    pub fn position(&self) -> &Chess {
        self.positions.last().expect("history is never empty")
    }

    pub fn start_position(&self) -> &Chess {
        &self.positions[0]
    }

    /// Positions in order: `positions()[i]` is the one before `moves()[i]`.
    pub fn positions(&self) -> &[Chess] {
        &self.positions
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Repetition keys of [`Game::positions`], for search draw detection.
    pub fn repetition_keys(&self) -> &[Zobrist64] {
        &self.keys
    }

    /// Moves played, in SAN with check and mate suffixes.
    pub fn sans(&self) -> Vec<String> {
        self.sans.iter().map(ToString::to_string).collect()
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    /// Legal moves in the current position; empty once the game is over.
    pub fn legal_moves(&self) -> MoveList {
        if self.status.is_over() {
            MoveList::new()
        } else {
            self.position().legal_moves()
        }
    }

    pub fn play(&mut self, mv: Move) -> Result<(), GameError> {
        if self.status.is_over() {
            return Err(GameError::GameOver);
        }
        let mut next = self.position().clone();
        if !next.is_legal(mv) {
            return Err(GameError::IllegalMove);
        }
        let san = SanPlus::from_move_and_play_unchecked(&mut next, mv);
        self.keys.push(repetition_key(&next));
        self.positions.push(next);
        self.moves.push(mv);
        self.sans.push(san);
        self.status = self.compute_status();
        Ok(())
    }

    /// Plays a move given in SAN, for example `Nf3` or `exd8=Q+`.
    pub fn play_san(&mut self, san: &str) -> Result<(), GameError> {
        if self.status.is_over() {
            return Err(GameError::GameOver);
        }
        let invalid = || GameError::InvalidSan(san.to_owned());
        let parsed = SanPlus::from_ascii(san.as_bytes()).map_err(|_| invalid())?;
        let mv = parsed.san.to_move(self.position()).map_err(|_| invalid())?;
        self.play(mv)
    }

    /// Takes back the last move. Returns it, or `None` at the start of the game.
    pub fn undo(&mut self) -> Option<Move> {
        let mv = self.moves.pop()?;
        self.sans.pop();
        self.positions.pop();
        self.keys.pop();
        self.status = self.compute_status();
        Some(mv)
    }

    /// Checkmate comes first: mate on the hundredth halfmove beats the fifty-move
    /// rule. Repetition and fifty moves need the full history, hence not
    /// `Position::outcome`.
    fn compute_status(&self) -> GameStatus {
        let position = self.position();
        if position.is_checkmate() {
            GameStatus::Checkmate {
                winner: !position.turn(),
            }
        } else if position.is_stalemate() {
            GameStatus::Stalemate
        } else if position.is_insufficient_material() {
            GameStatus::InsufficientMaterial
        } else if self.repetitions() >= THREEFOLD {
            GameStatus::ThreefoldRepetition
        } else if position.halfmoves() >= FIFTY_MOVE_HALFMOVES {
            GameStatus::FiftyMoves
        } else {
            GameStatus::InProgress
        }
    }

    /// How many times the current position has occurred, itself included.
    fn repetitions(&self) -> usize {
        let current = self.keys.last().expect("history is never empty");
        self.keys.iter().filter(|key| *key == current).count()
    }
}

/// Identifies a position for repetition: pieces, side to move, castling rights
/// and en passant, the latter only if the capture is legal.
fn repetition_key(position: &Chess) -> Zobrist64 {
    position.zobrist_hash(EnPassantMode::Legal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play_all(game: &mut Game, sans: &str) {
        for san in sans.split_whitespace() {
            game.play_san(san).unwrap();
        }
    }

    #[test]
    fn detects_checkmate_and_stops_the_game() {
        let mut game = Game::new();
        play_all(&mut game, "f3 e5 g4 Qh4#");
        assert_eq!(
            game.status(),
            GameStatus::Checkmate {
                winner: Color::Black
            }
        );
        assert!(game.legal_moves().is_empty());
        assert_eq!(game.play_san("a3"), Err(GameError::GameOver));
    }

    #[test]
    fn detects_stalemate() {
        let mut game = Game::from_fen("7k/8/6K1/5Q2/8/8/8/8 w - - 0 1").unwrap();
        play_all(&mut game, "Qf7");
        assert_eq!(game.status(), GameStatus::Stalemate);
    }

    #[test]
    fn detects_insufficient_material() {
        let mut game = Game::from_fen("8/8/8/4k3/8/3K4/3r4/8 w - - 0 1").unwrap();
        play_all(&mut game, "Kxd2");
        assert_eq!(game.status(), GameStatus::InsufficientMaterial);
    }

    #[test]
    fn detects_fifty_moves() {
        let mut game = Game::from_fen("7k/8/8/8/8/8/R7/K7 w - - 99 80").unwrap();
        play_all(&mut game, "Ra3");
        assert_eq!(game.status(), GameStatus::FiftyMoves);
    }

    #[test]
    fn checkmate_beats_fifty_moves() {
        let mut game = Game::from_fen("6k1/5ppp/8/8/8/8/8/R5K1 w - - 99 80").unwrap();
        play_all(&mut game, "Ra8#");
        assert_eq!(
            game.status(),
            GameStatus::Checkmate {
                winner: Color::White
            }
        );
    }

    #[test]
    fn detects_threefold_repetition() {
        let mut game = Game::new();
        play_all(&mut game, "Nf3 Nf6 Ng1 Ng8 Nf3 Nf6 Ng1");
        assert_eq!(game.status(), GameStatus::InProgress);
        play_all(&mut game, "Ng8");
        assert_eq!(game.status(), GameStatus::ThreefoldRepetition);
    }

    #[test]
    fn lost_castling_rights_make_a_position_different() {
        // The start has the same pieces as plies 4 and 8, but only there may White
        // castle, so the first true threefold is the position after ply 1.
        let mut game = Game::from_fen("4k3/8/8/8/8/8/8/R3K3 w Q - 0 1").unwrap();
        play_all(&mut game, "Rb1 Kd8 Ra1 Ke8 Rb1 Kd8 Ra1 Ke8");
        assert_eq!(game.status(), GameStatus::InProgress);
        play_all(&mut game, "Rb1");
        assert_eq!(game.status(), GameStatus::ThreefoldRepetition);
    }

    #[test]
    fn available_en_passant_makes_a_position_different() {
        // After 1. e4 Black may capture en passant; that is gone on plies 5 and 9,
        // so the first true threefold is the position after ply 2.
        let mut game = Game::from_fen("4k3/8/8/8/3p4/8/4P3/4K3 w - - 0 1").unwrap();
        play_all(&mut game, "e4 Ke7 Kd1 Ke8 Ke1 Ke7 Kd1 Ke8 Ke1");
        assert_eq!(game.status(), GameStatus::InProgress);
        play_all(&mut game, "Ke7");
        assert_eq!(game.status(), GameStatus::ThreefoldRepetition);
    }

    #[test]
    fn undo_restores_position_and_status() {
        let mut game = Game::new();
        play_all(&mut game, "f3 e5 g4 Qh4#");
        let mate = game.moves()[3];
        assert_eq!(game.undo(), Some(mate));
        assert_eq!(game.status(), GameStatus::InProgress);
        assert_eq!(game.sans(), ["f3", "e5", "g4"]);
        while game.undo().is_some() {}
        assert_eq!(game.position(), &Chess::default());
    }

    #[test]
    fn rejects_illegal_input() {
        let mut game = Game::new();
        assert!(matches!(
            game.play_san("Ke2"),
            Err(GameError::InvalidSan(_))
        ));
        assert!(matches!(
            game.play_san("garbage!"),
            Err(GameError::InvalidSan(_))
        ));
        let white_pawn_move = Game::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1")
            .unwrap()
            .legal_moves()[0];
        let mut pawnless = Game::from_fen("4k3/8/8/8/8/8/8/R3K3 b Q - 0 1").unwrap();
        assert_eq!(pawnless.play(white_pawn_move), Err(GameError::IllegalMove));
    }

    #[test]
    fn writes_san_for_special_moves() {
        let mut game = Game::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        play_all(&mut game, "O-O O-O-O");
        assert_eq!(game.sans(), ["O-O", "O-O-O"]);

        let mut game = Game::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2").unwrap();
        play_all(&mut game, "exd6");
        assert_eq!(game.sans(), ["exd6"]);

        let mut game = Game::from_fen("8/4P2k/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        play_all(&mut game, "e8=N");
        assert_eq!(game.sans(), ["e8=N"]);

        let mut game = Game::from_fen("4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1").unwrap();
        play_all(&mut game, "Nbd2");
        assert_eq!(game.sans(), ["Nbd2"]);
    }
}
