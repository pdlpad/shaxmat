use shakmaty::{Chess, Color, Position};

use crate::{Game, to_fen};

const MAX_LINE_WIDTH: usize = 80;

/// PGN header values. Anything left unset is written as unknown, as the PGN
/// standard prescribes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PgnTags {
    pub event: Option<String>,
    pub site: Option<String>,
    pub date: Option<String>,
    pub round: Option<String>,
    pub white: Option<String>,
    pub black: Option<String>,
}

impl Game {
    /// Exports the game as PGN. A game not started from the standard position
    /// carries `SetUp` and `FEN` tags.
    pub fn to_pgn(&self, tags: &PgnTags) -> String {
        let or_unknown = |value: &Option<String>| value.as_deref().unwrap_or("?").to_owned();
        let date = tags.date.as_deref().unwrap_or("????.??.??").to_owned();

        let mut pgn = String::new();
        for (name, value) in [
            ("Event", or_unknown(&tags.event)),
            ("Site", or_unknown(&tags.site)),
            ("Date", date),
            ("Round", or_unknown(&tags.round)),
            ("White", or_unknown(&tags.white)),
            ("Black", or_unknown(&tags.black)),
            ("Result", self.status().result().to_owned()),
        ] {
            push_tag(&mut pgn, name, &value);
        }
        let start = self.start_position();
        if *start != Chess::default() {
            push_tag(&mut pgn, "SetUp", "1");
            push_tag(&mut pgn, "FEN", &to_fen(start));
        }
        pgn.push('\n');
        pgn.push_str(&wrap(&self.movetext_tokens()));
        pgn.push('\n');
        pgn
    }

    fn movetext_tokens(&self) -> Vec<String> {
        let start = self.start_position();
        let mut number = start.fullmoves().get();
        let mut turn = start.turn();
        let mut tokens = Vec::new();
        for (i, san) in self.sans().into_iter().enumerate() {
            if turn == Color::White {
                tokens.push(format!("{number}."));
            } else if i == 0 {
                tokens.push(format!("{number}..."));
            }
            tokens.push(san);
            if turn == Color::Black {
                number += 1;
            }
            turn = !turn;
        }
        tokens.push(self.status().result().to_owned());
        tokens
    }
}

fn push_tag(pgn: &mut String, name: &str, value: &str) {
    let value = value.replace('\\', "\\\\").replace('"', "\\\"");
    pgn.push_str(&format!("[{name} \"{value}\"]\n"));
}

fn wrap(tokens: &[String]) -> String {
    let mut text = String::new();
    let mut width = 0;
    for token in tokens {
        if width > 0 && width + 1 + token.len() > MAX_LINE_WIDTH {
            text.push('\n');
            width = 0;
        } else if width > 0 {
            text.push(' ');
            width += 1;
        }
        text.push_str(token);
        width += token.len();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_finished_game() {
        let mut game = Game::new();
        for san in ["f3", "e5", "g4", "Qh4#"] {
            game.play_san(san).unwrap();
        }
        let tags = PgnTags {
            white: Some("Player".to_owned()),
            black: Some("Bot \"3\"".to_owned()),
            ..PgnTags::default()
        };
        assert_eq!(
            game.to_pgn(&tags),
            "[Event \"?\"]\n[Site \"?\"]\n[Date \"????.??.??\"]\n[Round \"?\"]\n\
             [White \"Player\"]\n[Black \"Bot \\\"3\\\"\"]\n[Result \"0-1\"]\n\n\
             1. f3 e5 2. g4 Qh4# 0-1\n"
        );
    }

    #[test]
    fn writes_setup_and_numbers_moves_from_a_black_start() {
        let fen = "4k3/8/8/8/8/8/4P3/4K3 b - - 3 12";
        let mut game = Game::from_fen(fen).unwrap();
        for san in ["Kd7", "e4", "Kc6"] {
            game.play_san(san).unwrap();
        }
        let pgn = game.to_pgn(&PgnTags::default());
        assert!(pgn.contains("[SetUp \"1\"]\n"));
        assert!(pgn.contains(&format!("[FEN \"{fen}\"]\n")));
        assert!(pgn.ends_with("\n12... Kd7 13. e4 Kc6 *\n"));
    }

    #[test]
    fn wraps_long_movetext() {
        let mut game = Game::new();
        for san in [
            "a3", "a6", "b3", "b6", "c3", "c6", "d3", "d6", "e3", "e6", "f3", "f6", "g3", "g6",
            "h3", "h6", "a4", "a5", "b4", "b5", "c4", "c5", "d4", "d5", "e4", "e5", "f4", "f5",
        ] {
            game.play_san(san).unwrap();
        }
        let pgn = game.to_pgn(&PgnTags::default());
        let movetext = pgn.split("\n\n").nth(1).unwrap();
        assert!(movetext.lines().count() > 1);
        assert!(movetext.lines().all(|line| line.len() <= MAX_LINE_WIDTH));
    }
}
