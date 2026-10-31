//! Property tests: on positions reached by random play, every engine and level
//! returns a legal move, and analysis only lists legal moves.

use proptest::prelude::*;
use shaxmat_core::{Game, Position};
use shaxmat_engine::{Engine, LEVEL_COUNT, LevelEngine, Limits, NativeClock, Searcher};

/// Plays moves picked by `choices`, stopping before the game ends.
fn random_game(choices: &[usize]) -> Game {
    let mut game = Game::new();
    for &choice in choices {
        let moves = game.legal_moves();
        if moves.is_empty() {
            break;
        }
        let mv = moves[choice % moves.len()];
        game.play(mv).unwrap();
        if game.status().is_over() {
            game.undo();
            break;
        }
    }
    game
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn levels_play_legal_moves(
        choices in prop::collection::vec(any::<usize>(), 0..120),
        level in 1..=LEVEL_COUNT,
        seed in any::<u64>(),
    ) {
        let game = random_game(&choices);
        let mut engine = LevelEngine::new(level, seed, Box::new(NativeClock::default()));
        // The depth cap keeps the upper levels quick; legality does not depend on it.
        let mv = engine.best_move(&(&game).into(), &Limits::depth(2).with_top_n(0));
        let mv = mv.expect("the game is not over, so a move exists");
        prop_assert!(game.position().is_legal(mv));
    }

    #[test]
    fn analysis_lists_distinct_legal_moves_best_first(
        choices in prop::collection::vec(any::<usize>(), 0..120),
        top_n in 1usize..6,
    ) {
        let game = random_game(&choices);
        let mut engine = Searcher::new(Box::new(NativeClock::default()));
        let scored = engine.analyze(&(&game).into(), &Limits::depth(2).with_top_n(top_n));
        prop_assert_eq!(scored.len(), top_n.min(game.legal_moves().len()));
        for (i, entry) in scored.iter().enumerate() {
            prop_assert!(game.position().is_legal(entry.mv));
            prop_assert!(scored[i + 1..].iter().all(|later| later.mv != entry.mv));
        }
        prop_assert!(scored.windows(2).all(|pair| pair[0].score >= pair[1].score));
    }

    #[test]
    fn a_tiny_time_budget_still_gives_a_legal_move(
        choices in prop::collection::vec(any::<usize>(), 0..120),
    ) {
        let game = random_game(&choices);
        let mut engine = Searcher::new(Box::new(NativeClock::default()));
        let mv = engine.best_move(&(&game).into(), &Limits::time_ms(0)).unwrap();
        prop_assert!(game.position().is_legal(mv));
    }
}
