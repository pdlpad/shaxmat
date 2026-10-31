use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess, EnPassantMode};

use crate::GameError;

/// Parses a FEN string into a legal position.
pub fn parse_fen(fen: &str) -> Result<Chess, GameError> {
    let invalid = |reason: String| GameError::InvalidFen(reason);
    fen.parse::<Fen>()
        .map_err(|e| invalid(e.to_string()))?
        .into_position(CastlingMode::Standard)
        .map_err(|e| invalid(e.to_string()))
}

/// Serializes a position to FEN. The en passant square is written only if the
/// capture is actually legal.
pub fn to_fen(position: &Chess) -> String {
    Fen::from_position(position, EnPassantMode::Legal).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_fen() {
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 3 17";
        assert_eq!(to_fen(&parse_fen(fen).unwrap()), fen);
    }

    #[test]
    fn rejects_malformed_and_illegal_fen() {
        assert!(matches!(
            parse_fen("not a fen"),
            Err(GameError::InvalidFen(_))
        ));
        // Two white kings.
        assert!(matches!(
            parse_fen("4k3/8/8/8/8/8/8/K3K3 w - - 0 1"),
            Err(GameError::InvalidFen(_))
        ));
    }
}
