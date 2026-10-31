//! A game against the bot as the player sees it: selection, legal targets and the
//! promotion choice, driven by clicks.

use shaxmat_core::shakmaty::Move;
use shaxmat_core::{Color, Game, Position, Role, Square};

/// Pieces offered on promotion, in menu order.
pub const PROMOTION_ROLES: [Role; 4] = [Role::Queen, Role::Rook, Role::Bishop, Role::Knight];

pub struct Session {
    pub game: Game,
    pub human: Color,
    pub level: usize,
    selected: Option<Square>,
    /// Where the selected piece can go.
    targets: Vec<Square>,
    /// The from and to squares of a move waiting for the promotion choice.
    promotion: Option<(Square, Square)>,
}

impl Session {
    pub fn new(game: Game, human: Color, level: usize) -> Self {
        Session {
            game,
            human,
            level,
            selected: None,
            targets: Vec::new(),
            promotion: None,
        }
    }

    pub fn selected(&self) -> Option<Square> {
        self.selected
    }

    pub fn targets(&self) -> &[Square] {
        &self.targets
    }

    pub fn promotion(&self) -> Option<(Square, Square)> {
        self.promotion
    }

    pub fn is_human_turn(&self) -> bool {
        !self.game.status().is_over() && self.game.position().turn() == self.human
    }

    pub fn is_bot_turn(&self) -> bool {
        !self.game.status().is_over() && self.game.position().turn() != self.human
    }

    /// The board is drawn from the player's side.
    pub fn flipped(&self) -> bool {
        self.human == Color::Black
    }

    /// Handles a click on a square: selects a piece, moves the selected one, or
    /// clears the selection.
    pub fn click(&mut self, square: Square) {
        if !self.is_human_turn() || self.promotion.is_some() {
            return;
        }
        if let Some(from) = self.selected
            && self.targets.contains(&square)
        {
            self.move_to(from, square);
            return;
        }
        let own_piece = self.game.position().board().color_at(square) == Some(self.human);
        if own_piece && self.selected != Some(square) {
            self.selected = Some(square);
            self.targets = self
                .game
                .legal_moves()
                .iter()
                .filter(|mv| mv.from() == Some(square))
                .map(|&mv| destination(mv, self.human))
                .collect();
        } else {
            self.clear_selection();
        }
    }

    /// Answers the promotion question. `None` takes the move back.
    pub fn choose_promotion(&mut self, role: Option<Role>) {
        let Some((from, to)) = self.promotion.take() else {
            return;
        };
        if let Some(mv) = role.and_then(|role| self.find_move(from, to, Some(role))) {
            self.play(mv);
        }
        self.clear_selection();
    }

    fn move_to(&mut self, from: Square, to: Square) {
        if self.find_move(from, to, Some(Role::Queen)).is_some() {
            self.promotion = Some((from, to));
        } else if let Some(mv) = self.find_move(from, to, None) {
            self.play(mv);
        }
        self.selected = self.promotion.map(|(from, _)| from);
        self.targets.clear();
    }

    fn find_move(&self, from: Square, to: Square, promotion: Option<Role>) -> Option<Move> {
        self.game.legal_moves().iter().copied().find(|&mv| {
            mv.from() == Some(from)
                && destination(mv, self.human) == to
                && mv.promotion() == promotion
        })
    }

    fn play(&mut self, mv: Move) {
        self.game.play(mv).expect("moves come from the legal list");
        self.clear_selection();
    }

    fn clear_selection(&mut self) {
        self.selected = None;
        self.targets.clear();
    }
}

/// The square the player clicks for a move. Castling is shown as the king's two
/// squares, not as `shakmaty`'s king-takes-rook.
pub fn destination(mv: Move, color: Color) -> Square {
    match mv.castling_side() {
        Some(side) => side.king_to(color),
        None => mv.to(),
    }
}

#[cfg(test)]
mod tests {
    use shaxmat_core::GameStatus;

    use super::*;

    fn session(fen: &str, human: Color) -> Session {
        Session::new(Game::from_fen(fen).unwrap(), human, 1)
    }

    fn last_san(session: &Session) -> String {
        session.game.sans().pop().unwrap()
    }

    #[test]
    fn selects_shows_targets_and_moves() {
        let mut s = Session::new(Game::new(), Color::White, 1);
        s.click(Square::E2);
        assert_eq!(s.selected(), Some(Square::E2));
        assert_eq!(s.targets().len(), 2);
        s.click(Square::E4);
        assert_eq!(last_san(&s), "e4");
        assert_eq!(s.selected(), None);
        assert!(s.is_bot_turn());
    }

    #[test]
    fn reselects_deselects_and_ignores_foreign_pieces() {
        let mut s = Session::new(Game::new(), Color::White, 1);
        s.click(Square::E7);
        assert_eq!(s.selected(), None);
        s.click(Square::E2);
        s.click(Square::G1);
        assert_eq!(s.selected(), Some(Square::G1));
        s.click(Square::G1);
        assert_eq!(s.selected(), None);
        s.click(Square::E2);
        s.click(Square::A5);
        assert_eq!(s.selected(), None);
        assert!(s.game.moves().is_empty());
    }

    #[test]
    fn ignores_clicks_on_the_bots_turn() {
        let mut s = Session::new(Game::new(), Color::Black, 1);
        s.click(Square::E7);
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn castles_both_ways() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let mut s = session(fen, Color::White);
        s.click(Square::E1);
        assert!(s.targets().contains(&Square::G1));
        s.click(Square::G1);
        assert_eq!(last_san(&s), "O-O");

        let mut s = session(fen, Color::White);
        s.click(Square::E1);
        s.click(Square::C1);
        assert_eq!(last_san(&s), "O-O-O");

        let mut s = session("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1", Color::Black);
        s.click(Square::E8);
        s.click(Square::G8);
        assert_eq!(last_san(&s), "O-O");
    }

    #[test]
    fn captures_en_passant() {
        let mut s = session("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2", Color::White);
        s.click(Square::E5);
        assert!(s.targets().contains(&Square::D6));
        s.click(Square::D6);
        assert_eq!(last_san(&s), "exd6");
        assert!(s.game.position().board().role_at(Square::D5).is_none());
    }

    #[test]
    fn asks_before_promoting_and_plays_the_choice() {
        for (role, san) in [(Role::Queen, "a8=Q+"), (Role::Knight, "a8=N")] {
            let mut s = session("6k1/P7/8/8/8/8/8/4K3 w - - 0 1", Color::White);
            s.click(Square::A7);
            s.click(Square::A8);
            assert!(s.promotion().is_some());
            assert!(s.game.moves().is_empty());
            // Board clicks do nothing until the choice is made.
            s.click(Square::E1);
            assert_eq!(s.selected(), Some(Square::A7));
            s.choose_promotion(Some(role));
            assert_eq!(last_san(&s), san);
            assert_eq!(s.promotion(), None);
        }
    }

    #[test]
    fn cancelling_the_promotion_keeps_the_turn() {
        let mut s = session("6k1/P7/8/8/8/8/8/4K3 w - - 0 1", Color::White);
        s.click(Square::A7);
        s.click(Square::A8);
        s.choose_promotion(None);
        assert_eq!(s.promotion(), None);
        assert!(s.game.moves().is_empty());
        assert!(s.is_human_turn());
    }

    #[test]
    fn no_turn_once_the_game_is_over() {
        let mut s = session("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", Color::White);
        s.click(Square::A1);
        s.click(Square::A8);
        assert!(matches!(s.game.status(), GameStatus::Checkmate { .. }));
        assert!(!s.is_human_turn());
        assert!(!s.is_bot_turn());
    }
}
