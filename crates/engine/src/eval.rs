//! Static evaluation: tapered material and piece-square tables, mobility and king
//! safety.
//!
//! Material and piece-square values are Ronald Friederich's PeSTO tables, listed
//! from a8 to h1 as seen by White.

use shaxmat_core::shakmaty::attacks::{self, king_attacks};
use shaxmat_core::shakmaty::{Bitboard, Board};
use shaxmat_core::{Chess, Color, Position, Role};

const ROLES: usize = 6;
const MAX_PHASE: i32 = 24;

const MG_VALUE: [i32; ROLES] = [82, 337, 365, 477, 1025, 0];
const EG_VALUE: [i32; ROLES] = [94, 281, 297, 512, 936, 0];
const PHASE_WEIGHT: [i32; ROLES] = [0, 1, 1, 2, 4, 0];

/// Mobility per reachable square, and the square count treated as neutral.
const MG_MOBILITY: [i32; ROLES] = [0, 4, 5, 2, 1, 0];
const EG_MOBILITY: [i32; ROLES] = [0, 4, 5, 4, 2, 0];
const NEUTRAL_MOBILITY: [i32; ROLES] = [0, 4, 6, 7, 13, 0];

/// King safety: weight of an attack on the king zone, by attacker.
const KING_ATTACK_WEIGHT: [i32; ROLES] = [0, 2, 2, 3, 5, 0];
const MAX_KING_ATTACK_PENALTY: i32 = 400;
const MISSING_SHIELD_PAWN_PENALTY: i32 = 12;

#[rustfmt::skip]
const MG_PST: [[i32; 64]; ROLES] = [
    [
          0,   0,   0,   0,   0,   0,   0,   0,
         98, 134,  61,  95,  68, 126,  34, -11,
         -6,   7,  26,  31,  65,  56,  25, -20,
        -14,  13,   6,  21,  23,  12,  17, -23,
        -27,  -2,  -5,  12,  17,   6,  10, -25,
        -26,  -4,  -4, -10,   3,   3,  33, -12,
        -35,  -1, -20, -23, -15,  24,  38, -22,
          0,   0,   0,   0,   0,   0,   0,   0,
    ],
    [
        -167, -89, -34, -49,  61, -97, -15, -107,
         -73, -41,  72,  36,  23,  62,   7,  -17,
         -47,  60,  37,  65,  84, 129,  73,   44,
          -9,  17,  19,  53,  37,  69,  18,   22,
         -13,   4,  16,  13,  28,  19,  21,   -8,
         -23,  -9,  12,  10,  19,  17,  25,  -16,
         -29, -53, -12,  -3,  -1,  18, -14,  -19,
        -105, -21, -58, -33, -17, -28, -19,  -23,
    ],
    [
        -29,   4, -82, -37, -25, -42,   7,  -8,
        -26,  16, -18, -13,  30,  59,  18, -47,
        -16,  37,  43,  40,  35,  50,  37,  -2,
         -4,   5,  19,  50,  37,  37,   7,  -2,
         -6,  13,  13,  26,  34,  12,  10,   4,
          0,  15,  15,  15,  14,  27,  18,  10,
          4,  15,  16,   0,   7,  21,  33,   1,
        -33,  -3, -14, -21, -13, -12, -39, -21,
    ],
    [
         32,  42,  32,  51,  63,   9,  31,  43,
         27,  32,  58,  62,  80,  67,  26,  44,
         -5,  19,  26,  36,  17,  45,  61,  16,
        -24, -11,   7,  26,  24,  35,  -8, -20,
        -36, -26, -12,  -1,   9,  -7,   6, -23,
        -45, -25, -16, -17,   3,   0,  -5, -33,
        -44, -16, -20,  -9,  -1,  11,  -6, -71,
        -19, -13,   1,  17,  16,   7, -37, -26,
    ],
    [
        -28,   0,  29,  12,  59,  44,  43,  45,
        -24, -39,  -5,   1, -16,  57,  28,  54,
        -13, -17,   7,   8,  29,  56,  47,  57,
        -27, -27, -16, -16,  -1,  17,  -2,   1,
         -9, -26,  -9, -10,  -2,  -4,   3,  -3,
        -14,   2, -11,  -2,  -5,   2,  14,   5,
        -35,  -8,  11,   2,   8,  15,  -3,   1,
         -1, -18,  -9,  10, -15, -25, -31, -50,
    ],
    [
        -65,  23,  16, -15, -56, -34,   2,  13,
         29,  -1, -20,  -7,  -8,  -4, -38, -29,
         -9,  24,   2, -16, -20,   6,  22, -22,
        -17, -20, -12, -27, -30, -25, -14, -36,
        -49,  -1, -27, -39, -46, -44, -33, -51,
        -14, -14, -22, -46, -44, -30, -15, -27,
          1,   7,  -8, -64, -43, -16,   9,   8,
        -15,  36,  12, -54,   8, -28,  24,  14,
    ],
];

#[rustfmt::skip]
const EG_PST: [[i32; 64]; ROLES] = [
    [
          0,   0,   0,   0,   0,   0,   0,   0,
        178, 173, 158, 134, 147, 132, 165, 187,
         94, 100,  85,  67,  56,  53,  82,  84,
         32,  24,  13,   5,  -2,   4,  17,  17,
         13,   9,  -3,  -7,  -7,  -8,   3,  -1,
          4,   7,  -6,   1,   0,  -5,  -1,  -8,
         13,   8,   8,  10,  13,   0,   2,  -7,
          0,   0,   0,   0,   0,   0,   0,   0,
    ],
    [
        -58, -38, -13, -28, -31, -27, -63, -99,
        -25,  -8, -25,  -2,  -9, -25, -24, -52,
        -24, -20,  10,   9,  -1,  -9, -19, -41,
        -17,   3,  22,  22,  22,  11,   8, -18,
        -18,  -6,  16,  25,  16,  17,   4, -18,
        -23,  -3,  -1,  15,  10,  -3, -20, -22,
        -42, -20, -10,  -5,  -2, -20, -23, -44,
        -29, -51, -23, -15, -22, -18, -50, -64,
    ],
    [
        -14, -21, -11,  -8,  -7,  -9, -17, -24,
         -8,  -4,   7, -12,  -3, -13,  -4, -14,
          2,  -8,   0,  -1,  -2,   6,   0,   4,
         -3,   9,  12,   9,  14,  10,   3,   2,
         -6,   3,  13,  19,   7,  10,  -3,  -9,
        -12,  -3,   8,  10,  13,   3,  -7, -15,
        -14, -18,  -7,  -1,   4,  -9, -15, -27,
        -23,  -9, -23,  -5,  -9, -16,  -5, -17,
    ],
    [
         13,  10,  18,  15,  12,  12,   8,   5,
         11,  13,  13,  11,  -3,   3,   8,   3,
          7,   7,   7,   5,   4,  -3,  -5,  -3,
          4,   3,  13,   1,   2,   1,  -1,   2,
          3,   5,   8,   4,  -5,  -6,  -8, -11,
         -4,   0,  -5,  -1,  -7, -12,  -8, -16,
         -6,  -6,   0,   2,  -9,  -9, -11,  -3,
         -9,   2,   3,  -1,  -5, -13,   4, -20,
    ],
    [
         -9,  22,  22,  27,  27,  19,  10,  20,
        -17,  20,  32,  41,  58,  25,  30,   0,
        -20,   6,   9,  49,  47,  35,  19,   9,
          3,  22,  24,  45,  57,  40,  57,  36,
        -18,  28,  19,  47,  31,  34,  39,  23,
        -16, -27,  15,   6,   9,  17,  10,   5,
        -22, -23, -30, -16, -16, -23, -36, -32,
        -33, -28, -22, -43,  -5, -32, -20, -41,
    ],
    [
        -74, -35, -18, -18, -11,  15,   4, -17,
        -12,  17,  14,  17,  17,  38,  23,  11,
         10,  17,  23,  15,  20,  45,  44,  13,
         -8,  22,  24,  27,  26,  33,  26,   3,
        -18,  -4,  21,  24,  27,  23,   9, -11,
        -19,  -3,  11,  21,  23,  16,   7,  -9,
        -27, -11,   4,  13,  14,   4,  -5, -17,
        -53, -34, -21, -11, -28, -14, -24, -43,
    ],
];

/// Scores the position in centipawns from the side to move's point of view.
pub fn evaluate(position: &Chess) -> i32 {
    let board = position.board();
    let (mut mg, mut eg, mut phase) = (0, 0, 0);
    let mut king_attack = [0; 2];

    let occupied = board.occupied();
    let king_zones = [Color::White, Color::Black].map(|color| king_zone(board, color));

    for (square, piece) in board.iter() {
        let role = piece.role as usize - 1;
        let pst_square = match piece.color {
            Color::White => usize::from(square) ^ 56,
            Color::Black => usize::from(square),
        };
        let sign = sign(piece.color);
        mg += sign * (MG_VALUE[role] + MG_PST[role][pst_square]);
        eg += sign * (EG_VALUE[role] + EG_PST[role][pst_square]);
        phase += PHASE_WEIGHT[role];

        if matches!(piece.role, Role::Pawn | Role::King) {
            continue;
        }
        let reach = attacks::attacks(square, piece, occupied);
        let moves = (reach & !board.by_color(piece.color)).count() as i32;
        mg += sign * MG_MOBILITY[role] * (moves - NEUTRAL_MOBILITY[role]);
        eg += sign * EG_MOBILITY[role] * (moves - NEUTRAL_MOBILITY[role]);

        let enemy_zone = king_zones[(!piece.color) as usize];
        king_attack[(!piece.color) as usize] +=
            KING_ATTACK_WEIGHT[role] * (reach & enemy_zone).count() as i32;
    }

    for color in [Color::White, Color::Black] {
        let attack = king_attack[color as usize];
        let penalty =
            (attack * attack / 4).min(MAX_KING_ATTACK_PENALTY) + shield_penalty(board, color);
        mg -= sign(color) * penalty;
    }

    let phase = phase.min(MAX_PHASE);
    let score = (mg * phase + eg * (MAX_PHASE - phase)) / MAX_PHASE;
    score * sign(position.turn())
}

fn sign(color: Color) -> i32 {
    match color {
        Color::White => 1,
        Color::Black => -1,
    }
}

fn king_zone(board: &Board, color: Color) -> Bitboard {
    match board.king_of(color) {
        Some(king) => king_attacks(king) | Bitboard::from(king),
        None => Bitboard::EMPTY,
    }
}

/// Penalizes missing pawns on the three files around a king that stays near its
/// back rank. A king further up has no shield to speak of, which the piece-square
/// tables already punish.
fn shield_penalty(board: &Board, color: Color) -> i32 {
    let Some(king) = board.king_of(color) else {
        return 0;
    };
    let rank = usize::from(king.rank());
    let ranks_from_home = if color == Color::White {
        rank
    } else {
        7 - rank
    };
    if ranks_from_home > 1 {
        return 0;
    }
    let Some(ahead) = king.offset(8 * sign(color)) else {
        return 0;
    };
    let shield = (king_attacks(ahead) | Bitboard::from(ahead)) & !Bitboard::from(king.rank());
    let pawns = board.by_piece(color.pawn()) & shield;
    MISSING_SHIELD_PAWN_PENALTY * (3 - pawns.count().min(3) as i32)
}

#[cfg(test)]
mod tests {
    use shaxmat_core::parse_fen;

    use super::*;

    fn eval(fen: &str) -> i32 {
        evaluate(&parse_fen(fen).unwrap())
    }

    #[test]
    fn start_position_is_balanced() {
        assert_eq!(
            eval("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
            0
        );
    }

    #[test]
    fn mirrored_positions_score_the_same() {
        for (fen, mirrored) in [
            (
                "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
                "rnbqk2r/pppp1ppp/5n2/2b1p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R b KQkq - 4 4",
            ),
            (
                "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
                "8/4p1p1/8/1r3P1K/kp5R/3P4/2P5/8 b - - 0 1",
            ),
        ] {
            assert_eq!(eval(fen), eval(mirrored), "{fen}");
        }
    }

    #[test]
    fn score_is_from_the_side_to_move() {
        let white = eval("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1");
        let black = eval("4k3/8/8/8/8/8/8/Q3K3 b - - 0 1");
        assert!(white > 700);
        assert_eq!(white, -black);
    }

    #[test]
    fn rewards_mobility() {
        let free = eval("4k3/8/8/8/3B4/8/8/4K3 w - - 0 1");
        let boxed =
            eval("4k3/8/8/8/8/8/1P6/B3K3 w - - 0 1") - eval("4k3/8/8/8/8/8/1P6/4K3 w - - 0 1");
        assert!(free > boxed);
    }

    #[test]
    fn punishes_an_exposed_king() {
        // The same material, with the pawns in front of the white king or not.
        let sheltered = eval("r5k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1");
        let exposed = eval("r5k1/5ppp/8/8/8/8/PPP5/R5K1 w - - 0 1");
        assert!(sheltered > exposed);
    }
}
