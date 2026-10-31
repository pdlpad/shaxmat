//! Whole games through the click handling: a level plays the human's side by
//! clicking squares, against the bot as the app runs it.

use shaxmat_core::{Color, Game, Move, Position};
use shaxmat_engine::{Engine, LevelEngine, Limits, NativeClock, SearchPosition};
use shaxmat_game::app::{App, Setup};
use shaxmat_game::layout::{Rect, promotion_menu, square_rect};
use shaxmat_game::session::destination;

/// Longest game accepted, in plies, before something must be stuck.
const MAX_PLIES: usize = 1000;

fn click_rect(app: &mut App, rect: Rect) {
    app.click(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
}

fn play_by_clicks(app: &mut App, mv: Move) {
    let flipped = app.session.flipped();
    let from = mv.from().expect("standard chess moves have a source");
    let to = destination(mv, app.session.human);
    click_rect(app, square_rect(from, flipped));
    click_rect(app, square_rect(to, flipped));
    if let Some(promotion) = mv.promotion() {
        let (_, rect) = promotion_menu(to, flipped)
            .into_iter()
            .find(|(role, _)| *role == promotion)
            .expect("the menu offers every promotion");
        click_rect(app, rect);
    }
}

fn play_game(seed: u64, human: Color) -> Vec<String> {
    let mut app = App::new(seed);
    app.start_game(Game::new(), Setup { level: 2, human });
    let mut player = LevelEngine::new(3, seed, Box::new(NativeClock::default()));

    while !app.session.game.status().is_over() {
        assert!(app.session.game.moves().len() < MAX_PLIES, "game is stuck");
        if app.session.is_human_turn() {
            let position = SearchPosition::from(&app.session.game);
            let mv = player
                .best_move(&position, &Limits::default())
                .expect("the game is running");
            let plies = app.session.game.moves().len();
            play_by_clicks(&mut app, mv);
            assert_eq!(
                app.session.game.moves().len(),
                plies + 1,
                "clicks did not play {mv:?}"
            );
        } else {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    app.session.game.sans()
}

#[test]
fn games_finish_through_clicks_with_every_special_move() {
    let games: Vec<Vec<String>> = (0..12)
        .map(|seed| {
            let human = if seed % 2 == 0 {
                Color::White
            } else {
                Color::Black
            };
            play_game(seed, human)
        })
        .collect();
    let any = |test: fn(&str) -> bool| games.iter().flatten().any(|san| test(san));
    assert!(any(|san| san.starts_with("O-O")));
    assert!(any(|san| san.contains('=')));
    assert!(any(|san| san.contains('#')));
}

#[test]
fn en_passant_works_through_clicks() {
    let mut app = App::new(1);
    let game = Game::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2").unwrap();
    app.start_game(
        game,
        Setup {
            level: 1,
            human: Color::White,
        },
    );
    let mv = app
        .session
        .game
        .legal_moves()
        .iter()
        .copied()
        .find(|mv| mv.is_en_passant())
        .unwrap();
    play_by_clicks(&mut app, mv);
    assert_eq!(app.session.game.sans(), ["exd6"]);
    assert!(app.session.game.position().board().pawns().count() == 1);
}
