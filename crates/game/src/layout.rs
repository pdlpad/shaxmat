//! Where everything is, in the coordinates of the 480x270 internal buffer.

use shaxmat_core::shakmaty::{File, Rank};
use shaxmat_core::{Color, Role, Square};

use crate::i18n::{Language, Strings};
use crate::session::PROMOTION_ROLES;

pub const SCREEN_WIDTH: u32 = 480;
pub const SCREEN_HEIGHT: u32 = 270;
pub const CELL: f32 = 24.0;
pub const BOARD_SIZE: f32 = CELL * 8.0;
pub const BOARD_X: f32 = 24.0;
pub const BOARD_Y: f32 = 39.0;
pub const PANEL_X: f32 = 232.0;
pub const PANEL_WIDTH: f32 = 224.0;

/// The glass card behind the panel.
pub const PANEL_CARD: Rect = Rect::new(
    PANEL_X - 6.0,
    BOARD_Y - 6.0,
    PANEL_WIDTH + 12.0,
    BOARD_SIZE + 12.0,
);
/// The tab that hides and shows the panel, always on the right edge.
pub const PANEL_TAB: Rect = Rect::new(466.0, 119.0, 11.0, 32.0);
/// How far the panel slides out when hidden.
pub const PANEL_SLIDE: f32 = 250.0;
/// How far the board slides to sit in the middle of the screen.
pub const BOARD_SLIDE: f32 = (SCREEN_WIDTH as f32 - BOARD_SIZE) / 2.0 - BOARD_X;

const BUTTON_HEIGHT: f32 = 18.0;
const BUTTON_GAP: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// How the buffer sits in the window: scaled by a whole number and centered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub scale: f32,
    pub x: f32,
    pub y: f32,
}

impl Viewport {
    pub fn fit(window_width: f32, window_height: f32) -> Self {
        let fit = (window_width / SCREEN_WIDTH as f32).min(window_height / SCREEN_HEIGHT as f32);
        let scale = fit.floor().max(1.0);
        Viewport {
            scale,
            x: ((window_width - SCREEN_WIDTH as f32 * scale) / 2.0).floor(),
            y: ((window_height - SCREEN_HEIGHT as f32 * scale) / 2.0).floor(),
        }
    }

    /// Converts window coordinates to buffer coordinates.
    pub fn to_buffer(&self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.x) / self.scale, (y - self.y) / self.scale)
    }
}

/// The on-screen rectangle of a square.
pub fn square_rect(square: Square, flipped: bool) -> Rect {
    let file = usize::from(square.file()) as f32;
    let rank = usize::from(square.rank()) as f32;
    let (column, row) = if flipped {
        (7.0 - file, rank)
    } else {
        (file, 7.0 - rank)
    };
    Rect::new(BOARD_X + column * CELL, BOARD_Y + row * CELL, CELL, CELL)
}

/// The square under a buffer position.
pub fn square_at(x: f32, y: f32, flipped: bool) -> Option<Square> {
    if !Rect::new(BOARD_X, BOARD_Y, BOARD_SIZE, BOARD_SIZE).contains(x, y) {
        return None;
    }
    let column = ((x - BOARD_X) / CELL) as u32;
    let row = ((y - BOARD_Y) / CELL) as u32;
    let (file, rank) = if flipped {
        (7 - column, row)
    } else {
        (column, 7 - row)
    };
    Some(Square::from_coords(File::new(file), Rank::new(rank)))
}

/// The promotion menu: four cells running down the board from the target square.
pub fn promotion_menu(to: Square, flipped: bool) -> [(Role, Rect); 4] {
    let first = square_rect(to, flipped);
    let step = if first.y <= BOARD_Y { CELL } else { -CELL };
    let mut menu = [(Role::Queen, first); 4];
    for (i, entry) in menu.iter_mut().enumerate() {
        *entry = (
            PROMOTION_ROLES[i],
            Rect {
                y: first.y + step * i as f32,
                ..first
            },
        );
    }
    menu
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelButton {
    NewGame,
    Resign,
    Draw,
}

/// The buttons under the move list.
pub fn panel_buttons(strings: &Strings) -> [(PanelButton, &'static str, Rect); 3] {
    let y = BOARD_Y + BOARD_SIZE - BUTTON_HEIGHT;
    let new_game = 92.0;
    let others = (PANEL_WIDTH - new_game - 2.0 * BUTTON_GAP) / 2.0;
    let resign_x = PANEL_X + new_game + BUTTON_GAP;
    let draw_x = resign_x + others + BUTTON_GAP;
    [
        (
            PanelButton::NewGame,
            strings.new_game,
            Rect::new(PANEL_X, y, new_game, BUTTON_HEIGHT),
        ),
        (
            PanelButton::Resign,
            strings.resign,
            Rect::new(resign_x, y, others, BUTTON_HEIGHT),
        ),
        (
            PanelButton::Draw,
            strings.draw,
            Rect::new(draw_x, y, others, BUTTON_HEIGHT),
        ),
    ]
}

pub const DIALOG: Rect = Rect::new(120.0, 51.0, 240.0, 168.0);

/// The level buttons of the new game dialog.
pub fn level_buttons() -> [(usize, Rect); 8] {
    let size = 24.0;
    let gap = (DIALOG.w - 16.0 - 8.0 * size) / 7.0;
    std::array::from_fn(|i| {
        let x = DIALOG.x + 8.0 + i as f32 * (size + gap);
        (i + 1, Rect::new(x, DIALOG.y + 34.0, size, size))
    })
}

/// The side buttons of the new game dialog.
pub fn side_buttons(strings: &Strings) -> [(Color, &'static str, Rect); 2] {
    let y = DIALOG.y + 78.0;
    let width = (DIALOG.w - 16.0 - BUTTON_GAP) / 2.0;
    let x = DIALOG.x + 8.0;
    [
        (
            Color::White,
            strings.white,
            Rect::new(x, y, width, BUTTON_HEIGHT),
        ),
        (
            Color::Black,
            strings.black,
            Rect::new(x + width + BUTTON_GAP, y, width, BUTTON_HEIGHT),
        ),
    ]
}

/// The language buttons of the new game dialog.
pub fn language_buttons() -> [(Language, Rect); 3] {
    let y = DIALOG.y + 114.0;
    let width = (DIALOG.w - 16.0 - 2.0 * BUTTON_GAP) / 3.0;
    Language::ALL.map(|language| {
        let index = Language::ALL
            .iter()
            .position(|known| *known == language)
            .unwrap_or(0) as f32;
        let x = DIALOG.x + 8.0 + index * (width + BUTTON_GAP);
        (language, Rect::new(x, y, width, BUTTON_HEIGHT))
    })
}

pub const DIALOG_START: Rect = Rect::new(
    DIALOG.x + DIALOG.w / 2.0 + 2.0,
    DIALOG.y + DIALOG.h - 26.0,
    DIALOG.w / 2.0 - 10.0,
    BUTTON_HEIGHT,
);

pub const DIALOG_CANCEL: Rect = Rect::new(
    DIALOG.x + 8.0,
    DIALOG.y + DIALOG.h - 26.0,
    DIALOG.w / 2.0 - 10.0,
    BUTTON_HEIGHT,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_by_whole_numbers_and_centers() {
        let exact = Viewport::fit(1440.0, 810.0);
        assert_eq!((exact.scale, exact.x, exact.y), (3.0, 0.0, 0.0));
        let wide = Viewport::fit(1920.0, 1080.0);
        assert_eq!(wide.scale, 4.0);
        let between = Viewport::fit(1000.0, 800.0);
        assert_eq!((between.scale, between.x, between.y), (2.0, 20.0, 130.0));
        assert_eq!(Viewport::fit(100.0, 100.0).scale, 1.0);
    }

    #[test]
    fn window_coordinates_map_to_the_buffer() {
        let viewport = Viewport::fit(1000.0, 800.0);
        assert_eq!(viewport.to_buffer(20.0, 130.0), (0.0, 0.0));
        assert_eq!(viewport.to_buffer(120.0, 230.0), (50.0, 50.0));
    }

    #[test]
    fn squares_round_trip_on_both_sides() {
        for flipped in [false, true] {
            for square in Square::ALL {
                let rect = square_rect(square, flipped);
                assert_eq!(
                    square_at(rect.x + 1.0, rect.y + 23.0, flipped),
                    Some(square)
                );
            }
        }
    }

    #[test]
    fn white_sits_at_the_bottom_unless_flipped() {
        assert_eq!(square_rect(Square::A1, false).y, BOARD_Y + 7.0 * CELL);
        assert_eq!(square_rect(Square::A1, false).x, BOARD_X);
        assert_eq!(square_rect(Square::A1, true).y, BOARD_Y);
        assert_eq!(square_rect(Square::A1, true).x, BOARD_X + 7.0 * CELL);
        assert_eq!(square_at(0.0, 0.0, false), None);
    }

    #[test]
    fn the_promotion_menu_stays_on_the_board() {
        for (to, flipped) in [
            (Square::A8, false),
            (Square::H1, false),
            (Square::A8, true),
            (Square::H1, true),
        ] {
            for (_, rect) in promotion_menu(to, flipped) {
                assert!(rect.y >= BOARD_Y && rect.y + rect.h <= BOARD_Y + BOARD_SIZE);
            }
        }
    }

    #[test]
    fn dialog_controls_fit_and_do_not_overlap() {
        let levels = level_buttons();
        for pair in levels.windows(2) {
            assert!(pair[0].1.x + pair[0].1.w <= pair[1].1.x);
        }
        let last = levels[7].1;
        assert!(last.x + last.w <= DIALOG.x + DIALOG.w);
        let side = side_buttons(Language::English.strings())[0].2;
        assert!(side.y + BUTTON_HEIGHT <= language_buttons()[0].1.y);
        assert!(language_buttons()[0].1.y + BUTTON_HEIGHT <= DIALOG_START.y);
    }
}
