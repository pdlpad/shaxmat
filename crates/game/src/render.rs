//! Drawing into the 480x270 buffer. Pieces and colors here are placeholders until
//! the assets arrive.

use macroquad::prelude::*;
use shaxmat_core::shakmaty::Piece;
use shaxmat_core::{Color as Side, Position, Square};

use crate::app::{App, Setup};
use crate::layout::{
    self, BOARD_SIZE, BOARD_X, BOARD_Y, CELL, DIALOG, DIALOG_CANCEL, DIALOG_START, PANEL_X,
    Rect as Area, SCREEN_HEIGHT, SCREEN_WIDTH,
};

const FONT_SIZE: u16 = 16;
const MOVE_ROWS: usize = 11;
const ROW_HEIGHT: f32 = 12.0;

const BACKGROUND: Color = color_u8!(16, 20, 44, 255);
const LIGHT_SQUARE: Color = color_u8!(202, 190, 160, 255);
const DARK_SQUARE: Color = color_u8!(84, 104, 150, 255);
const SELECTED: Color = color_u8!(255, 200, 60, 160);
const TARGET: Color = color_u8!(255, 200, 60, 220);
const LAST_MOVE: Color = color_u8!(120, 200, 120, 90);
const CHECK: Color = color_u8!(220, 60, 60, 170);
const TEXT: Color = color_u8!(236, 232, 220, 255);
const DIM_TEXT: Color = color_u8!(150, 156, 176, 255);
const BUTTON: Color = color_u8!(44, 54, 96, 255);
const BUTTON_ACTIVE: Color = color_u8!(200, 150, 40, 255);
const WHITE_PIECE: Color = color_u8!(240, 236, 224, 255);
const BLACK_PIECE: Color = color_u8!(30, 30, 40, 255);

/// Creates the buffer the game draws into, unfiltered so that pixels stay square.
pub fn create_buffer() -> RenderTarget {
    let target = render_target(SCREEN_WIDTH, SCREEN_HEIGHT);
    target.texture.set_filter(FilterMode::Nearest);
    target
}

/// Draws a frame into `buffer`.
pub fn draw_buffer(app: &App, buffer: &RenderTarget) {
    let mut camera = Camera2D::from_display_rect(macroquad::math::Rect::new(
        0.0,
        0.0,
        SCREEN_WIDTH as f32,
        SCREEN_HEIGHT as f32,
    ));
    camera.render_target = Some(buffer.clone());
    set_camera(&camera);

    clear_background(BACKGROUND);
    draw_board(app);
    draw_panel(app);
    if let Some((_, to)) = app.session.promotion() {
        draw_promotion_menu(app, to);
    }
    if let Some(setup) = &app.setup {
        draw_dialog(setup);
    }
}

/// Shows `buffer` in the window, scaled by a whole number.
pub fn present(buffer: &RenderTarget) {
    set_default_camera();
    clear_background(BLACK);
    let viewport = layout::Viewport::fit(screen_width(), screen_height());
    draw_texture_ex(
        &buffer.texture,
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

fn draw_board(app: &App) {
    let session = &app.session;
    let flipped = session.flipped();
    let position = session.game.position();

    for square in Square::ALL {
        let color = if square.is_light() {
            LIGHT_SQUARE
        } else {
            DARK_SQUARE
        };
        fill(layout::square_rect(square, flipped), color);
    }
    if let Some(&mv) = session.game.moves().last() {
        let mover = position.turn();
        let highlighted = [mv.from(), Some(crate::session::destination(mv, !mover))];
        for square in highlighted.into_iter().flatten() {
            fill(layout::square_rect(square, flipped), LAST_MOVE);
        }
    }
    if position.is_check()
        && let Some(king) = position.board().king_of(position.turn())
    {
        fill(layout::square_rect(king, flipped), CHECK);
    }
    if let Some(square) = session.selected() {
        fill(layout::square_rect(square, flipped), SELECTED);
    }
    for (square, piece) in position.board().iter() {
        draw_piece(layout::square_rect(square, flipped), piece);
    }
    for &square in session.targets() {
        let rect = layout::square_rect(square, flipped);
        if position.board().piece_at(square).is_some() {
            outline(rect, TARGET);
        } else {
            draw_rectangle(rect.x + 9.0, rect.y + 9.0, 6.0, 6.0, TARGET);
        }
    }
    draw_coordinates(flipped);
}

fn draw_coordinates(flipped: bool) {
    for i in 0..8 {
        let index = if flipped { 7 - i } else { i };
        let file = (b'a' + index) as char;
        let rank = (b'8' - index) as char;
        let offset = i as f32 * CELL;
        text(
            &file.to_string(),
            BOARD_X + offset + 9.0,
            BOARD_Y + BOARD_SIZE + 11.0,
            DIM_TEXT,
        );
        text(
            &rank.to_string(),
            BOARD_X - 11.0,
            BOARD_Y + offset + 16.0,
            DIM_TEXT,
        );
    }
}

/// A placeholder piece: a block in the side's color with the piece's letter.
fn draw_piece(rect: Area, piece: Piece) {
    let (body, letter) = match piece.color {
        Side::White => (WHITE_PIECE, BLACK_PIECE),
        Side::Black => (BLACK_PIECE, WHITE_PIECE),
    };
    let border = letter;
    draw_rectangle(rect.x + 3.0, rect.y + 3.0, 18.0, 18.0, border);
    draw_rectangle(rect.x + 4.0, rect.y + 4.0, 16.0, 16.0, body);
    let label = piece.role.upper_char().to_string();
    text(&label, rect.x + 9.0, rect.y + 16.0, letter);
}

fn draw_panel(app: &App) {
    let session = &app.session;
    text(
        &format!("SHAXMAT  level {}", session.level),
        PANEL_X,
        BOARD_Y + 8.0,
        TEXT,
    );
    let side = match session.human {
        Side::White => "white",
        Side::Black => "black",
    };
    text(
        &format!("You play {side}"),
        PANEL_X,
        BOARD_Y + 20.0,
        DIM_TEXT,
    );
    text(&app.status_line(), PANEL_X, BOARD_Y + 34.0, TEXT);
    if let Some(notice) = app.notice {
        text(notice, PANEL_X + 130.0, BOARD_Y + 20.0, TARGET);
    }

    let list_y = BOARD_Y + 50.0;
    for (i, (number, white, black)) in app.move_rows(MOVE_ROWS).iter().enumerate() {
        let y = list_y + i as f32 * ROW_HEIGHT;
        text(&format!("{number}."), PANEL_X, y, DIM_TEXT);
        text(white, PANEL_X + 32.0, y, TEXT);
        text(black, PANEL_X + 104.0, y, TEXT);
    }

    for (_, label, rect) in layout::panel_buttons() {
        button(rect, label, false);
    }
}

fn draw_promotion_menu(app: &App, to: Square) {
    let side = app.session.human;
    for (role, rect) in layout::promotion_menu(to, app.session.flipped()) {
        fill(rect, BUTTON);
        outline(rect, TARGET);
        draw_piece(rect, Piece { color: side, role });
    }
}

fn draw_dialog(setup: &Setup) {
    fill(
        Area::new(0.0, 0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32),
        color_u8!(0, 0, 0, 150),
    );
    fill(DIALOG, BACKGROUND);
    outline(DIALOG, TEXT);
    text("New game", DIALOG.x + 8.0, DIALOG.y + 13.0, TEXT);
    text("Bot level", DIALOG.x + 8.0, DIALOG.y + 29.0, DIM_TEXT);
    for (level, rect) in layout::level_buttons() {
        button(rect, &level.to_string(), level == setup.level);
    }
    text("Your side", DIALOG.x + 8.0, DIALOG.y + 73.0, DIM_TEXT);
    for (side, label, rect) in layout::side_buttons() {
        button(rect, label, side == setup.human);
    }
    button(DIALOG_CANCEL, "Cancel", false);
    button(DIALOG_START, "Start", false);
}

fn button(rect: Area, label: &str, active: bool) {
    fill(rect, if active { BUTTON_ACTIVE } else { BUTTON });
    outline(rect, DIM_TEXT);
    let width = measure_text(label, None, FONT_SIZE, 1.0).width;
    text(
        label,
        (rect.x + (rect.w - width) / 2.0).floor(),
        rect.y + rect.h / 2.0 + 4.0,
        TEXT,
    );
}

fn fill(rect: Area, color: Color) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
}

fn outline(rect: Area, color: Color) {
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, color);
}

fn text(label: &str, x: f32, y: f32, color: Color) {
    draw_text(label, x.floor(), y.floor(), f32::from(FONT_SIZE), color);
}
