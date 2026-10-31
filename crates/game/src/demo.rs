//! Scripted runs for screenshots: the real frame loop and click handling, with
//! the mouse replaced by a text file. Lines, `#` starting a comment:
//!
//!   game <level> <white|black> [fen]   start a game
//!   theme 1                            pick a board theme
//!   language en|ru|uz                  pick a language
//!   panel                              hide or show the panel
//!   click e2                           click a square
//!   promote q                          pick from the promotion menu
//!   press new|resign|draw              press a panel button
//!   point 100 120                      click a buffer position
//!   wait-bot                           let the bot answer
//!   fx hint e2 e4 | blunder e4 | brilliant e4 | clear-hint
//!   frames 10                          let frames pass
//!   shot records/name.png              save the buffer
//!   assert-status Your move            fail unless the status line says this
//!
//! At the end it prints the frame rate over the run.

use std::collections::VecDeque;

use macroquad::prelude::RenderTarget;
use shaxmat_core::{Color, Game, Role, Square};
use shaxmat_fx::Event;

use crate::app::{App, Setup};
use crate::effects::Effects;
use crate::i18n::Language;
use crate::layout::{PANEL_TAB, PanelButton, promotion_menu, square_rect};

/// A frame slower than this missed 60 fps by more than a little.
const LATE_FRAME_SECONDS: f32 = 0.02;

pub struct Demo {
    lines: VecDeque<String>,
    frames_left: u32,
    waiting_for_bot: bool,
    frame_times: Vec<f32>,
}

impl Demo {
    pub fn load(path: &str) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned)
            .collect();
        Ok(Demo {
            lines,
            frames_left: 0,
            waiting_for_bot: false,
            frame_times: Vec::new(),
        })
    }

    /// Runs lines until one has to wait for a later frame. Returns `false` at the
    /// end of the script.
    pub fn step(&mut self, app: &mut App, effects: &mut Effects, buffer: &RenderTarget) -> bool {
        self.frame_times.push(macroquad::time::get_frame_time());
        if self.frames_left > 0 {
            self.frames_left -= 1;
            return true;
        }
        if self.waiting_for_bot {
            if app.session.is_bot_turn() {
                return true;
            }
            self.waiting_for_bot = false;
        }
        while let Some(line) = self.lines.pop_front() {
            println!("demo: {line}");
            let (command, rest) = line.split_once(' ').unwrap_or((&line, ""));
            match command {
                "game" => start_game(app, rest),
                "fx" => play_effect(app, effects, rest),
                "panel" => app.click(PANEL_TAB.x + 1.0, PANEL_TAB.y + 1.0),
                "language" => {
                    app.language = match rest {
                        "en" => Language::English,
                        "uz" => Language::Uzbek,
                        _ => Language::Russian,
                    };
                }
                "theme" => app.theme = rest.parse().expect("theme takes a number"),
                "click" => click_square(app, rest),
                "promote" => promote(app, rest),
                "press" => press(app, rest),
                "point" => {
                    let numbers: Vec<f32> = rest
                        .split_whitespace()
                        .map(|n| n.parse().expect("point takes numbers"))
                        .collect();
                    app.click(numbers[0], numbers[1]);
                }
                "wait-bot" => {
                    self.waiting_for_bot = true;
                    return true;
                }
                "frames" => {
                    self.frames_left = rest.parse().expect("frames takes a number");
                    return true;
                }
                "shot" => {
                    buffer.texture.get_texture_data().export_png(rest);
                    // The buffer is stored bottom-up; `export_png` flips it.
                }
                "assert-status" => assert_eq!(app.status_line(), rest, "at line: {line}"),
                other => panic!("unknown demo command: {other}"),
            }
            // Clicks show on the next frame.
            if command != "assert-status" && command != "shot" {
                self.frames_left = 1;
                return true;
            }
        }
        self.report();
        false
    }

    /// Prints the frame rate, leaving out the slow first frames of loading.
    fn report(&self) {
        let times = &self.frame_times[self.frame_times.len().min(5)..];
        let total: f32 = times.iter().sum();
        let worst = times.iter().copied().fold(0.0, f32::max);
        let average_fps = times.len() as f32 / total;
        let late: Vec<usize> = (0..times.len())
            .filter(|&i| times[i] > LATE_FRAME_SECONDS)
            .collect();
        println!(
            "frames: {}, average {average_fps:.1} fps, worst frame {:.1} ms, late frames at {late:?}",
            times.len(),
            worst * 1000.0
        );
    }
}

fn play_effect(app: &mut App, effects: &mut Effects, args: &str) {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let flipped = app.session.flipped();
    let square = |name: &str| name.parse::<Square>().expect("a square name");
    let index = |name: &str| usize::from(square(name)) as u8;
    match parts.as_slice() {
        ["hint", from, to] => effects.show_hint(square(from), square(to), flipped),
        ["clear-hint"] => effects.clear_hint(),
        ["blunder", at] => effects.trigger(&Event::Blunder { square: index(at) }, flipped),
        ["brilliant", at] => effects.trigger(&Event::Brilliant { square: index(at) }, flipped),
        _ => panic!("unknown effect: {args}"),
    }
}

fn start_game(app: &mut App, args: &str) {
    let mut parts = args.splitn(3, ' ');
    let level = parts.next().and_then(|l| l.parse().ok()).expect("a level");
    let human = match parts.next() {
        Some("black") => Color::Black,
        _ => Color::White,
    };
    let game = parts.next().map_or_else(Game::new, |fen| {
        Game::from_fen(fen).expect("the script's FEN is valid")
    });
    app.start_game(game, Setup { level, human });
}

fn click_square(app: &mut App, name: &str) {
    let square: Square = name.parse().expect("a square name");
    let rect = square_rect(square, app.session.flipped());
    let shift = app.board_offset();
    app.click(rect.x + rect.w / 2.0 + shift, rect.y + rect.h / 2.0);
}

fn promote(app: &mut App, name: &str) {
    let wanted = Role::from_char(name.chars().next().expect("a piece letter"));
    let (_, to) = app.session.promotion().expect("a promotion is pending");
    for (role, rect) in promotion_menu(to, app.session.flipped()) {
        if Some(role) == wanted {
            app.click(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        }
    }
}

fn press(app: &mut App, name: &str) {
    app.press(match name {
        "new" => PanelButton::NewGame,
        "resign" => PanelButton::Resign,
        "draw" => PanelButton::Draw,
        other => panic!("unknown button: {other}"),
    });
}
