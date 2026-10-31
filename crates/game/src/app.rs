//! The running application: the game, the bot and the new game dialog.

use shaxmat_core::shakmaty::{Move, Piece};
use shaxmat_core::{Color, Game, GameStatus, Position, Role, Square};
use shaxmat_fx::Event;

use crate::bot::Bot;
use crate::i18n::{Language, Strings};
use crate::layout::{
    DIALOG_CANCEL, DIALOG_START, PanelButton, language_buttons, level_buttons, panel_buttons,
    promotion_menu, side_buttons, square_at,
};
use crate::session::{Session, destination};

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
    pub language: Language,
    /// Index of the board theme in the palette.
    pub theme: usize,
    /// Game events for the effects to pick up.
    events: Vec<Event>,
    /// Moves already turned into events.
    moves_seen: usize,
    /// The game to start once the transition covers the screen.
    pending_start: Option<Setup>,
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
            language: Language::Russian,
            theme: 0,
            events: Vec::new(),
            moves_seen: 0,
            pending_start: None,
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
        self.moves_seen = self.session.game.moves().len();
        self.pending_start = None;
    }

    /// The interface texts in the current language.
    pub fn strings(&self) -> &'static Strings {
        self.language.strings()
    }

    /// Starts the game chosen in the dialog; called when the screen is covered.
    pub fn finish_transition(&mut self) {
        if let Some(setup) = self.pending_start.take() {
            self.start_game(Game::new(), setup);
        }
    }

    /// The game events since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Advances the bot; call every frame.
    pub fn update(&mut self) {
        if self.setup.is_some() || self.pending_start.is_some() {
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
        self.note_new_moves();
    }

    /// Turns moves played since the last look into events for the effects.
    fn note_new_moves(&mut self) {
        let game = &self.session.game;
        let moves = game.moves();
        for (index, &mv) in moves.iter().enumerate().skip(self.moves_seen) {
            let before = &game.positions()[index];
            let after = &game.positions()[index + 1];
            move_events(mv, before, after, &mut self.events);
        }
        self.moves_seen = moves.len();
    }

    /// Handles a click at a buffer position.
    pub fn click(&mut self, x: f32, y: f32) {
        if self.pending_start.is_some() {
            return;
        }
        if self.setup.is_some() {
            self.click_dialog(x, y);
            return;
        }
        if let Some((button, _, _)) = panel_buttons(self.strings())
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
            self.notice = Some(self.strings().draw_declined);
        }
    }

    fn click_dialog(&mut self, x: f32, y: f32) {
        for (language, rect) in language_buttons() {
            if rect.contains(x, y) {
                self.language = language;
            }
        }
        let strings = self.strings();
        let Some(setup) = self.setup.as_mut() else {
            return;
        };
        for (level, rect) in level_buttons() {
            if rect.contains(x, y) {
                setup.level = level;
            }
        }
        for (color, _, rect) in side_buttons(strings) {
            if rect.contains(x, y) {
                setup.human = color;
            }
        }
        let setup = *setup;
        if DIALOG_START.contains(x, y) {
            self.setup = None;
            self.pending_start = Some(setup);
            self.events.push(Event::Transition);
        } else if DIALOG_CANCEL.contains(x, y) {
            self.setup = None;
        }
    }

    /// The line telling whose decision it is, or how the game ended.
    pub fn status_line(&self) -> String {
        let session = &self.session;
        let s = self.strings();
        let winner_line = |winner: Color| {
            if winner == session.human {
                s.you_won
            } else {
                s.bot_won
            }
        };
        match session.game.status() {
            GameStatus::InProgress if session.promotion().is_some() => s.choose_piece.to_owned(),
            GameStatus::InProgress => {
                let check = if session.game.position().is_check() {
                    s.check_suffix
                } else {
                    ""
                };
                let line = if session.is_human_turn() {
                    s.your_move
                } else {
                    s.bot_thinking
                };
                format!("{line}{check}")
            }
            GameStatus::Checkmate { winner } => {
                format!("{} - {}", s.checkmate, winner_line(winner))
            }
            GameStatus::Resigned { winner } => {
                format!("{} - {}", s.resigned, winner_line(winner))
            }
            GameStatus::Stalemate => s.stalemate.to_owned(),
            GameStatus::InsufficientMaterial => s.insufficient_material.to_owned(),
            GameStatus::FiftyMoves => s.fifty_moves.to_owned(),
            GameStatus::ThreefoldRepetition => s.repetition.to_owned(),
            GameStatus::DrawAgreed => s.draw_agreed.to_owned(),
        }
    }

    /// Move list rows as `(number, white, black)`, the last `count` of them.
    pub fn move_rows(&self, count: usize) -> Vec<(u32, String, String)> {
        let game = &self.session.game;
        let start = game.start_position();
        let black_first = start.turn() == Color::Black;
        let mut sans = game.sans();
        if sans.is_empty() {
            return Vec::new();
        }
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

/// The name of a piece in the effect sheets, like `wQ`.
fn piece_name(piece: Piece) -> String {
    let color = if piece.color == Color::White {
        'w'
    } else {
        'b'
    };
    format!("{color}{}", piece.role.upper_char())
}

fn square_index(square: Square) -> u8 {
    usize::from(square) as u8
}

/// What a single move sets off.
fn move_events(
    mv: Move,
    before: &shaxmat_core::Chess,
    after: &shaxmat_core::Chess,
    out: &mut Vec<Event>,
) {
    let mover = before.turn();
    let to = destination(mv, mover);
    if mv.is_castle() {
        let from = mv.from().expect("a castling king has a square");
        out.push(Event::Castle {
            from: square_index(from),
            to: square_index(to),
            king: piece_name(Piece {
                color: mover,
                role: Role::King,
            }),
        });
    }
    if let Some(role) = victim(mv) {
        let square = if mv.is_en_passant() {
            Square::from_coords(to.file(), mv.from().expect("pawns have a square").rank())
        } else {
            to
        };
        out.push(Event::Capture {
            square: square_index(square),
            piece: piece_name(Piece {
                color: !mover,
                role,
            }),
        });
    }
    if let Some(role) = mv.promotion() {
        out.push(Event::Promotion {
            square: square_index(to),
            piece: piece_name(Piece { color: mover, role }),
        });
    }
    let king_square = after.board().king_of(!mover).map(square_index);
    if let Some(king_square) = king_square {
        if after.is_checkmate() {
            out.push(Event::Mate {
                king_square,
                king: piece_name(Piece {
                    color: !mover,
                    role: Role::King,
                }),
            });
        } else if after.is_check() {
            out.push(Event::Check { king_square });
        }
    }
}

fn victim(mv: Move) -> Option<Role> {
    if mv.is_en_passant() {
        Some(Role::Pawn)
    } else {
        mv.capture()
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
        let (x, y) = center(side_buttons(app.strings())[1].2);
        app.click(x, y);
        let (x, y) = center(DIALOG_START);
        app.click(x, y);
        assert_eq!(app.take_events(), [Event::Transition]);
        assert!(app.setup.is_none());
        app.finish_transition();
        assert_eq!(app.session.level, 6);
        assert_eq!(app.session.human, Color::Black);
        assert!(app.session.flipped());
    }

    fn events_after(fen: &str, san: &str) -> Vec<Event> {
        let mut app = App::new(1);
        app.start_game(
            Game::from_fen(fen).unwrap(),
            Setup {
                level: 1,
                human: Color::White,
            },
        );
        app.session.game.play_san(san).unwrap();
        app.note_new_moves();
        app.take_events()
    }

    #[test]
    fn moves_set_off_their_events() {
        assert_eq!(
            events_after("4k3/8/8/3n4/4P3/8/8/4K3 w - - 0 1", "exd5"),
            [Event::Capture {
                square: 35,
                piece: "bN".to_owned()
            }]
        );
        assert_eq!(
            events_after("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2", "exd6"),
            [Event::Capture {
                square: 35,
                piece: "bP".to_owned()
            }]
        );
        assert_eq!(
            events_after("4k3/8/8/8/8/8/8/4K2R w K - 0 1", "O-O"),
            [Event::Castle {
                from: 4,
                to: 6,
                king: "wK".to_owned()
            }]
        );
        assert_eq!(
            events_after("7k/P7/8/8/8/8/8/4K3 w - - 0 1", "a8=N"),
            [Event::Promotion {
                square: 56,
                piece: "wN".to_owned()
            }]
        );
        assert_eq!(
            events_after("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", "Ra8+"),
            [Event::Check { king_square: 60 }]
        );
        assert_eq!(
            events_after("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", "Ra8#"),
            [Event::Mate {
                king_square: 62,
                king: "bK".to_owned()
            }]
        );
    }

    #[test]
    fn a_game_start_is_not_a_move() {
        let mut app = App::new(1);
        let mut game = Game::new();
        game.play_san("e4").unwrap();
        app.start_game(
            game,
            Setup {
                level: 1,
                human: Color::White,
            },
        );
        app.note_new_moves();
        assert!(app.take_events().is_empty());
    }

    #[test]
    fn the_dialog_switches_the_language() {
        let mut app = App::new(1);
        let (x, y) = center(language_buttons()[2].1);
        app.click(x, y);
        assert_eq!(app.language, Language::Uzbek);
        assert_eq!(app.status_line(), "Sizning yurishingiz");
        let (x, y) = center(language_buttons()[1].1);
        app.click(x, y);
        assert_eq!(app.status_line(), "Your move");
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
        assert_eq!(app.status_line(), "Сдача - бот выиграл");
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
        assert_eq!(app.notice, Some("Ничья отклонена"));
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
