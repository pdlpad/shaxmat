//! The UCI client against the binary's own UCI mode, the stand-in for Stockfish.

use std::process::Command;

use shaxmat_core::Game;
use shaxmat_engine_cli::uci::UciEngine;

#[test]
fn plays_a_short_game_over_uci() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    command.args(["uci", "--level", "1"]);
    let mut engine = UciEngine::spawn(command, 10);
    let mut game = Game::new();
    for _ in 0..12 {
        let mv = engine.best_move(&game).expect("the engine answers");
        game.play(mv).expect("the engine plays legal moves");
    }
    assert_eq!(game.moves().len(), 12);
}
