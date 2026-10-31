//! Drawing into the 480x270 buffer.

use macroquad::prelude::*;
use shaxmat_core::shakmaty::Piece;
use shaxmat_core::{Color as Side, Position, Square};

use crate::app::{App, Setup};
use crate::assets::Assets;
use crate::layout::{
    self, BOARD_SIZE, BOARD_X, BOARD_Y, CELL, DIALOG, DIALOG_CANCEL, DIALOG_START, PANEL_WIDTH,
    PANEL_X, Rect as Area, SCREEN_HEIGHT, SCREEN_WIDTH,
};
use crate::palette::{Palette, with_alpha};
use crate::session::destination;

const MOVE_ROWS: usize = 9;
const ROW_HEIGHT: f32 = 14.0;
const NUMBER_COLUMN: f32 = 0.0;
const WHITE_COLUMN: f32 = 32.0;
const BLACK_COLUMN: f32 = 112.0;
const TARGET_DOT: f32 = 6.0;

const SELECTED_ALPHA: f32 = 0.7;
const LAST_MOVE_ALPHA: f32 = 0.55;
const CHECK_ALPHA: f32 = 0.75;
const DIM_ALPHA: f32 = 0.6;

/// Everything the drawing needs that does not change during a game.
pub struct Renderer {
    pub assets: Assets,
    pub palette: Palette,
    buffer: RenderTarget,
}

impl Renderer {
    pub fn new() -> Self {
        // Unfiltered, so that pixels stay square.
        let buffer = render_target(SCREEN_WIDTH, SCREEN_HEIGHT);
        buffer.texture.set_filter(FilterMode::Nearest);
        Renderer {
            assets: Assets::load(),
            palette: Palette::load(),
            buffer,
        }
    }

    pub fn buffer(&self) -> &RenderTarget {
        &self.buffer
    }

    /// Draws a frame into the buffer.
    pub fn draw(&self, app: &App) {
        let mut camera = Camera2D::from_display_rect(macroquad::math::Rect::new(
            0.0,
            0.0,
            SCREEN_WIDTH as f32,
            SCREEN_HEIGHT as f32,
        ));
        camera.render_target = Some(self.buffer.clone());
        set_camera(&camera);

        clear_background(self.palette.navy);
        self.draw_board(app);
        self.draw_panel(app);
        if let Some((_, to)) = app.session.promotion() {
            self.draw_promotion_menu(app, to);
        }
        if let Some(setup) = &app.setup {
            self.draw_dialog(setup);
        }
    }

    /// Shows the buffer in the window, scaled by a whole number.
    pub fn present(&self) {
        set_default_camera();
        clear_background(self.palette.ink);
        let viewport = layout::Viewport::fit(screen_width(), screen_height());
        draw_texture_ex(
            &self.buffer.texture,
            viewport.x,
            viewport.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(
                    SCREEN_WIDTH as f32 * viewport.scale,
                    SCREEN_HEIGHT as f32 * viewport.scale,
                )),
                flip_y: true,
                ..Default::default()
            },
        );
    }

    fn draw_board(&self, app: &App) {
        let session = &app.session;
        let palette = &self.palette;
        let flipped = session.flipped();
        let position = session.game.position();
        let theme = &palette.themes[app.theme];

        for square in Square::ALL {
            let color = if square.is_light() {
                theme.light
            } else {
                theme.dark
            };
            fill(layout::square_rect(square, flipped), color);
        }
        if let Some(&mv) = session.game.moves().last() {
            let mover = !position.turn();
            for square in [mv.from(), Some(destination(mv, mover))]
                .into_iter()
                .flatten()
            {
                let color = with_alpha(palette.sky, LAST_MOVE_ALPHA);
                fill(layout::square_rect(square, flipped), color);
            }
        }
        if position.is_check()
            && let Some(king) = position.board().king_of(position.turn())
        {
            let color = with_alpha(palette.red, CHECK_ALPHA);
            fill(layout::square_rect(king, flipped), color);
        }
        if let Some(square) = session.selected() {
            let color = with_alpha(palette.amber, SELECTED_ALPHA);
            fill(layout::square_rect(square, flipped), color);
        }
        for (square, piece) in position.board().iter() {
            self.draw_piece(layout::square_rect(square, flipped), piece);
        }
        for &square in session.targets() {
            let rect = layout::square_rect(square, flipped);
            if position.board().piece_at(square).is_some() {
                outline(rect, palette.gold);
            } else {
                let offset = (CELL - TARGET_DOT) / 2.0;
                let (x, y) = (rect.x + offset, rect.y + offset);
                draw_rectangle(x, y, TARGET_DOT, TARGET_DOT, palette.ink);
                let inner = TARGET_DOT - 2.0;
                draw_rectangle(x + 1.0, y + 1.0, inner, inner, palette.gold);
            }
        }
        self.draw_coordinates(flipped);
    }

    fn draw_coordinates(&self, flipped: bool) {
        let color = self.palette.sky;
        for i in 0..8 {
            let index = if flipped { 7 - i } else { i };
            let file = ((b'a' + index) as char).to_string();
            let rank = ((b'8' - index) as char).to_string();
            let offset = f32::from(i) * CELL;
            let file_x = BOARD_X + offset + (CELL - self.width(&file)) / 2.0;
            self.text(&file, file_x, BOARD_Y + BOARD_SIZE + 12.0, color);
            self.text(&rank, BOARD_X - 12.0, BOARD_Y + offset + 16.0, color);
        }
    }

    fn draw_piece(&self, rect: Area, piece: Piece) {
        let column = piece.role as usize - 1;
        let row = usize::from(piece.color == Side::Black);
        draw_texture_ex(
            &self.assets.pieces,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                source: Some(macroquad::math::Rect::new(
                    column as f32 * CELL,
                    row as f32 * CELL,
                    CELL,
                    CELL,
                )),
                ..Default::default()
            },
        );
    }

    fn draw_panel(&self, app: &App) {
        let session = &app.session;
        let palette = &self.palette;
        let dim = with_alpha(palette.sky, 1.0);
        self.text(
            &format!("SHAXMAT, уровень {}", session.level),
            PANEL_X,
            BOARD_Y + 10.0,
            palette.gold,
        );
        let side = match session.human {
            Side::White => "белыми",
            Side::Black => "чёрными",
        };
        self.text(&format!("Вы играете {side}"), PANEL_X, BOARD_Y + 24.0, dim);
        self.text(&app.status_line(), PANEL_X, BOARD_Y + 38.0, palette.cream);
        if let Some(notice) = app.notice {
            let x = PANEL_X + PANEL_WIDTH - self.width(notice);
            self.text(notice, x, BOARD_Y + 24.0, palette.gold);
        }

        let list_y = BOARD_Y + 54.0;
        for (i, (number, white, black)) in app.move_rows(MOVE_ROWS).iter().enumerate() {
            let y = list_y + i as f32 * ROW_HEIGHT;
            let label = format!("{number}.");
            self.text(&label, PANEL_X + NUMBER_COLUMN, y, dim);
            self.text(white, PANEL_X + WHITE_COLUMN, y, palette.cream);
            self.text(black, PANEL_X + BLACK_COLUMN, y, palette.cream);
        }

        for (_, label, rect) in layout::panel_buttons() {
            self.button(rect, label, false);
        }
    }

    fn draw_promotion_menu(&self, app: &App, to: Square) {
        let color = app.session.human;
        for (role, rect) in layout::promotion_menu(to, app.session.flipped()) {
            fill(rect, self.palette.navy);
            outline(rect, self.palette.gold);
            self.draw_piece(rect, Piece { color, role });
        }
    }

    fn draw_dialog(&self, setup: &Setup) {
        let palette = &self.palette;
        let screen = Area::new(0.0, 0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
        fill(screen, with_alpha(palette.ink, DIM_ALPHA));
        fill(DIALOG, palette.navy);
        outline(DIALOG, palette.gold);
        self.text(
            "Новая партия",
            DIALOG.x + 8.0,
            DIALOG.y + 15.0,
            palette.gold,
        );
        self.text("Уровень бота", DIALOG.x + 8.0, DIALOG.y + 30.0, palette.sky);
        for (level, rect) in layout::level_buttons() {
            self.button(rect, &level.to_string(), level == setup.level);
        }
        self.text("Ваш цвет", DIALOG.x + 8.0, DIALOG.y + 73.0, palette.sky);
        for (side, label, rect) in layout::side_buttons() {
            self.button(rect, label, side == setup.human);
        }
        self.button(DIALOG_CANCEL, "Отмена", false);
        self.button(DIALOG_START, "Начать", false);
    }

    fn button(&self, rect: Area, label: &str, active: bool) {
        let palette = &self.palette;
        let (body, ink) = if active {
            (palette.gold, palette.ink)
        } else {
            (palette.blue, palette.cream)
        };
        fill(rect, body);
        outline(rect, palette.sky);
        let x = rect.x + (rect.w - self.width(label)) / 2.0;
        self.text(label, x, rect.y + rect.h / 2.0 + 5.0, ink);
    }

    fn width(&self, text: &str) -> f32 {
        self.assets.metrics.measure(text)
    }

    /// Draws a line of text with its baseline at `y`.
    fn text(&self, label: &str, x: f32, y: f32, color: Color) {
        let metrics = &self.assets.metrics;
        let mut x = x.floor();
        let top = y.floor() - f32::from(metrics.ascent);
        for char in label.chars() {
            let Some(glyph) = metrics.glyphs.get(&char) else {
                continue;
            };
            draw_texture_ex(
                &self.assets.font,
                x,
                top,
                color,
                DrawTextureParams {
                    source: Some(macroquad::math::Rect::new(
                        f32::from(glyph.x),
                        f32::from(glyph.y),
                        f32::from(glyph.width),
                        f32::from(metrics.height),
                    )),
                    ..Default::default()
                },
            );
            x += f32::from(glyph.advance);
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

fn fill(rect: Area, color: Color) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
}

fn outline(rect: Area, color: Color) {
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, color);
}
