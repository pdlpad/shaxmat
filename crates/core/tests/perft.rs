//! Perft against the published reference counts from
//! <https://www.chessprogramming.org/Perft_Results>.

use shaxmat_core::{Game, Move, parse_fen, perft};

const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
const POSITION_3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
const POSITION_4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
const POSITION_5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
const POSITION_6: &str = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

fn assert_perft(fen: &str, expected: &[u64]) {
    let position = parse_fen(fen).unwrap();
    for (i, &nodes) in expected.iter().enumerate() {
        let depth = i as u32 + 1;
        assert_eq!(perft(&position, depth), nodes, "{fen} at depth {depth}");
    }
}

#[test]
fn start_position() {
    assert_perft(START, &[20, 400, 8_902, 197_281, 4_865_609]);
}

#[test]
fn kiwipete() {
    assert_perft(KIWIPETE, &[48, 2_039, 97_862, 4_085_603]);
}

#[test]
fn rook_endgame_with_en_passant_pins() {
    assert_perft(POSITION_3, &[14, 191, 2_812, 43_238, 674_624, 11_030_083]);
}

#[test]
fn promotions_and_castling() {
    assert_perft(POSITION_4, &[6, 264, 9_467, 422_333, 15_833_292]);
}

#[test]
fn promotion_with_discovered_checks() {
    assert_perft(POSITION_5, &[44, 1_486, 62_379, 2_103_487]);
}

#[test]
fn symmetrical_middlegame() {
    assert_perft(POSITION_6, &[46, 2_079, 89_890, 3_894_594]);
}

/// Deep run: `cargo test --release -- --ignored`.
#[test]
#[ignore = "slow"]
fn deep() {
    assert_eq!(perft(&parse_fen(START).unwrap(), 6), 119_060_324);
    assert_eq!(perft(&parse_fen(KIWIPETE).unwrap(), 5), 193_690_690);
    assert_eq!(perft(&parse_fen(POSITION_3).unwrap(), 7), 178_633_661);
    assert_eq!(perft(&parse_fen(POSITION_4).unwrap(), 6), 706_045_033);
    assert_eq!(perft(&parse_fen(POSITION_5).unwrap(), 5), 89_941_194);
    assert_eq!(perft(&parse_fen(POSITION_6).unwrap(), 5), 164_075_551);
}

/// The same count through `Game::play` and `Game::undo`, so the wrapper is
/// covered and not just `shakmaty`.
fn game_perft(game: &mut Game, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves: Vec<Move> = game.legal_moves().to_vec();
    let mut nodes = 0;
    for mv in moves {
        game.play(mv).unwrap();
        nodes += game_perft(game, depth - 1);
        game.undo().unwrap();
    }
    nodes
}

#[test]
fn game_history_matches_perft() {
    for (fen, depth, nodes) in [
        (START, 4, 197_281),
        (KIWIPETE, 3, 97_862),
        (POSITION_3, 4, 43_238),
        (POSITION_4, 3, 9_467),
        (POSITION_5, 3, 62_379),
    ] {
        let mut game = Game::from_fen(fen).unwrap();
        assert_eq!(game_perft(&mut game, depth), nodes, "{fen}");
        assert_eq!(game.position(), &parse_fen(fen).unwrap());
    }
}
