//! The running application: the game, the bot and the new game dialog.

use shaxmat_core::{Color, Game, GameStatus, Position};

use crate::bot::Bot;
use crate::layout::{
    DIALOG_CANCEL, DIALOG_START, PanelButton, level_buttons, panel_buttons, promotion_menu,
    side_buttons, square_at,
};
use crate::session::Session;

pub const DEFAULT_LEVEL: usize = 3;

/// The choices in the new game dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setup {
    pub level: usize,
    pub human: Color,
}

pub struct App {
    pub session: Session,
    pub bot: Bot,
    /// The new game dialog, when open.
    pub setup: Option<Setup>,
    /// A one-off message, such as a declined draw.
    pub notice: Option<&'static str>,
    next_seed: u64,
}

impl App {
    pub fn new(seed: u64) -> Self {
        App {
            session: Session::new(Game::new(), Color::White, DEFAULT_LEVEL),
            bot: Bot::new(DEFAULT_LEVEL, seed),
            setup: Some(Setup {
                level: DEFAULT_LEVEL,
                human: Color::White,
            }),
            notice: None,
            next_seed: seed.wrapping_add(1),
        }
    }

    /// Replaces the game. A bot still thinking about the old one is dropped.
    pub fn start_game(&mut self, game: Game, setup: Setup) {
        self.session = Session::new(game, setup.human, setup.level);
        self.bot = Bot::new(setup.level, self.next_seed);
        self.next_seed = self.next_seed.wrapping_add(1);
        self.setup = None;
        self.notice = None;
    }

    /// Advances the bot; call every frame.
    pub fn update(&mut self) {
        if self.setup.is_some() {
            return;
        }
        // A move that arrives after the player resigned has nothing left to do.
        if let Some(mv) = self.bot.poll()
            && self.session.is_bot_turn()
        {
            self.session
                .game
                .play(mv)
                .expect("the bot answers the current position");
        }
        if self.session.is_bot_turn() && !self.bot.is_thinking() {
            self.bot.think(&self.session.game);
        }
    }

    /// Handles a click at a buffer position.
    pub fn click(&mut self, x: f32, y: f32) {
        if self.setup.is_some() {
            self.click_dialog(x, y);
            return;
        }
        if let Some((button, _, _)) = panel_buttons()
            .into_iter()
            .find(|(_, _, rect)| rect.contains(x, y))
        {
            self.press(button);
            return;
        }
        let flipped = self.session.flipped();
        if let Some((_, to)) = self.session.promotion() {
            let choice = promotion_menu(to, flipped)
                .into_iter()
                .find(|(_, rect)| rect.contains(x, y))
                .map(|(role, _)| role);
            self.session.choose_promotion(choice);
            return;
        }
        if let Some(square) = square_at(x, y, flipped) {
            let moves_before = self.session.game.moves().len();
            self.session.click(square);
            if self.session.game.moves().len() != moves_before {
                self.notice = None;
            }
        }
    }

    pub fn press(&mut self, button: PanelButton) {
        match button {
            PanelButton::NewGame => {
                self.setup = Some(Setup {
                    level: self.session.level,
                    human: self.session.human,
                });
            }
            PanelButton::Resign => {
                if self.session.is_human_turn() || self.session.is_bot_turn() {
                    self.session
                        .game
                        .resign(self.session.human)
                        .expect("the game is running");
                }
            }
            PanelButton::Draw => self.offer_draw(),
        }
    }

    fn offer_draw(&mut self) {
        if !self.session.is_human_turn() || self.bot.is_thinking() {
            return;
        }
        if self.bot.accepts_draw(&self.session.game) {
            self.session.game.agree_draw().expect("the game is running");
        } else {
            self.notice = Some("Draw declined");
        }
    }

    fn click_dialog(&mut self, x: f32, y: f32) {
        let Some(setup) = self.setup.as_mut() else {
            return;
        };
        for (level, rect) in level_buttons() {
            if rect.contains(x, y) {
                setup.level = level;
            }
        }
        for (color, _, rect) in side_buttons() {
            if rect.contains(x, y) {
                setup.human = color;
            }
        }
        let setup = *setup;
        if DIALOG_START.contains(x, y) {
            self.start_game(Game::new(), setup);
        } else if DIALOG_CANCEL.contains(x, y) {
            self.setup = None;
        }
    }

    /// The line telling whose decision it is, or how the game ended.
    pub fn status_line(&self) -> String {
        let session = &self.session;
        let name = |color: Color| if color == session.human { "You" } else { "Bot" };
        match session.game.status() {
            GameStatus::InProgress if session.promotion().is_some() => "Choose a piece".to_owned(),
            GameStatus::InProgress => {
                let check = if session.game.position().is_check() {
                    " - check!"
                } else {
                    ""
                };
                if session.is_human_turn() {
                    format!("Your move{check}")
                } else {
                    format!("Bot is thinking{check}")
                }
            }
            GameStatus::Checkmate { winner } => format!("Checkmate - {} won", name(winner)),
            GameStatus::Resigned { winner } => format!("Resigned - {} won", name(winner)),
            GameStatus::Stalemate => "Draw - stalemate".to_owned(),
            GameStatus::InsufficientMaterial => "Draw - insufficient material".to_owned(),
            GameStatus::FiftyMoves => "Draw - fifty moves".to_owned(),
            GameStatus::ThreefoldRepetition => "Draw - repetition".to_owned(),
            GameStatus::DrawAgreed => "Draw agreed".to_owned(),
        }
    }

    /// Move list rows as `(number, white, black)`, the last `count` of them.
    pub fn move_rows(&self, count: usize) -> Vec<(u32, String, String)> {
        let game = &self.session.game;
        let start = game.start_position();
        let black_first = start.turn() == Color::Black;
        let mut sans = game.sans();
        if black_first {
            sans.insert(0, "...".to_owned());
        }
        let first_number = start.fullmoves().get();
        let rows: Vec<_> = sans
            .chunks(2)
            .enumerate()
            .map(|(i, pair)| {
                let black = pair.get(1).cloned().unwrap_or_default();
                (first_number + i as u32, pair[0].clone(), black)
            })
            .collect();
        rows[rows.len().saturating_sub(count)..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(rect: crate::layout::Rect) -> (f32, f32) {
        (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
    }

    #[test]
    fn the_dialog_opens_first_and_starts_the_chosen_game() {
        let mut app = App::new(1);
        assert!(app.setup.is_some());
        let (x, y) = center(level_buttons()[5].1);
        app.click(x, y);
        let (x, y) = center(side_buttons()[1].2);
        app.click(x, y);
        let (x, y) = center(DIALOG_START);
        app.click(x, y);
        assert!(app.setup.is_none());
        assert_eq!(app.session.level, 6);
        assert_eq!(app.session.human, Color::Black);
        assert!(app.session.flipped());
    }

    #[test]
    fn resigning_ends_the_game() {
        let mut app = App::new(1);
        app.start_game(
            Game::new(),
            Setup {
                level: 1,
                human: Color::White,
            },
        );
        app.press(PanelButton::Resign);
        assert_eq!(
            app.session.game.status(),
            GameStatus::Resigned {
                winner: Color::Black
            }
        );
        assert_eq!(app.status_line(), "Resigned - Bot won");
    }

    #[test]
    fn resigning_while_the_bot_thinks_ignores_its_late_move() {
        let mut app = App::new(1);
        app.start_game(
            Game::new(),
            Setup {
                level: 1,
                human: Color::Black,
            },
        );
        app.update();
        assert!(app.bot.is_thinking());
        app.press(PanelButton::Resign);
        while app.bot.is_thinking() {
            app.update();
        }
        assert!(app.session.game.moves().is_empty());
        assert!(app.session.game.status().is_over());
    }

    #[test]
    fn a_draw_offer_can_be_declined_and_the_message_goes_away() {
        let mut app = App::new(1);
        app.start_game(
            Game::from_fen("7k/8/8/8/8/8/3q4/K7 w - - 0 1").unwrap(),
            Setup {
                level: 1,
                human: Color::White,
            },
        );
        app.press(PanelButton::Draw);
        assert_eq!(app.notice, Some("Draw declined"));
        assert_eq!(app.session.game.status(), GameStatus::InProgress);
    }

    #[test]
    fn move_rows_pair_moves_and_handle_a_black_start() {
        let mut app = App::new(1);
        for san in ["e4", "e5", "Nf3"] {
            app.session.game.play_san(san).unwrap();
        }
        assert_eq!(
            app.move_rows(10),
            [
                (1, "e4".to_owned(), "e5".to_owned()),
                (2, "Nf3".to_owned(), String::new())
            ]
        );
        assert_eq!(app.move_rows(1).len(), 1);

        let mut game = Game::from_fen("4k3/8/8/8/8/8/4P3/4K3 b - - 0 7").unwrap();
        game.play_san("Kd7").unwrap();
        let app = {
            let mut app = App::new(1);
            app.session = Session::new(game, Color::White, 1);
            app
        };
        assert_eq!(app.move_rows(10), [(7, "...".to_owned(), "Kd7".to_owned())]);
    }
}
