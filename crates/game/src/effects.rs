//! Plays the baked effects on the board.

use macroquad::prelude::*;
use shaxmat_core::Square;
use shaxmat_fx::{Cue, Event, Instance, Place, Scene, Sheet, Tag, block_seconds, cues};

use crate::layout::{BOARD_X, CELL, SCREEN_HEIGHT, SCREEN_WIDTH, square_rect};

const MANIFEST: &[(&str, &[u8], &str)] = include!("../../../assets/fx/manifest.rs");

/// Screen effects are 320x180 art, doubled and cropped to stay on whole pixels.
const SCREEN_SCALE: f32 = 2.0;
const TRANSITION_SHEET: &str = "iris";
/// The share of the transition at which the screen is fully covered.
const TRANSITION_COVER: f32 = 0.45;
const BANNER_Y: f32 = 22.0;
/// The toast's text starts this far right of its center, past the medal.
const BANNER_TEXT_X: f32 = -30.0;
const BANNER_TEXT_FROM: f32 = 0.25;
const BANNER_TEXT_TO: f32 = 1.9;

pub struct Effects {
    pub scene: Scene,
    textures: Vec<Texture2D>,
    selection: Option<(Square, Vec<Square>)>,
    transition: usize,
    banner: usize,
    /// The running transition has not swapped the screen yet.
    cover_pending: bool,
}

impl Effects {
    pub fn load() -> Self {
        let mut sheets = Vec::with_capacity(MANIFEST.len());
        let mut textures = Vec::with_capacity(MANIFEST.len());
        for (name, png, json) in MANIFEST {
            let texture = Texture2D::from_file_with_format(png, Some(ImageFormat::Png));
            texture.set_filter(FilterMode::Nearest);
            textures.push(texture);
            let sheet = Sheet::parse(json).expect("baked sheets parse");
            sheets.push(((*name).to_owned(), sheet));
        }
        let scene = Scene::new(sheets);
        Effects {
            transition: scene.sheet_index(TRANSITION_SHEET),
            banner: scene.sheet_index("achievement"),
            scene,
            textures,
            selection: None,
            cover_pending: false,
        }
    }

    pub fn trigger(&mut self, event: &Event, flipped: bool) {
        for cue in cues(event) {
            self.start(&cue, flipped);
        }
        self.scene.block_input(block_seconds(event));
        if *event == Event::Transition {
            self.cover_pending = true;
        }
    }

    fn start(&mut self, cue: &Cue, flipped: bool) {
        let sheet = self.scene.sheet_index(&cue.sheet);
        let mut instance = match cue.place {
            Place::Square(index) => {
                let (x, y) = foot_of(square_from(index), flipped);
                let mut instance = Instance::new(sheet, x, y, Tag::Event);
                instance.square = Some(index);
                instance
            }
            Place::Banner => Instance::new(sheet, BOARD_X + 4.0 * CELL, BANNER_Y, Tag::Event),
            Place::Screen => {
                let x = (SCREEN_WIDTH as f32 - 320.0 * SCREEN_SCALE) / 2.0;
                let y = (SCREEN_HEIGHT as f32 - 180.0 * SCREEN_SCALE) / 2.0;
                let mut instance = Instance::new(sheet, x, y, Tag::Event);
                instance.scale = SCREEN_SCALE;
                instance
            }
        };
        instance.delay = cue.delay;
        instance.hide_before_start = cue.hide_before_start;
        self.scene.spawn(instance);
    }

    /// Keeps the halo and the target ripples on the current selection.
    pub fn sync_selection(&mut self, selected: Option<Square>, targets: &[Square], flipped: bool) {
        let wanted = selected.map(|square| (square, targets.to_vec()));
        if wanted == self.selection {
            return;
        }
        self.scene.remove(Tag::Selection);
        if let Some((square, targets)) = &wanted {
            let halo = self.scene.sheet_index("select-halo");
            let (x, y) = foot_of(*square, flipped);
            self.scene.spawn(Instance::new(halo, x, y, Tag::Selection));
            let ripple = self.scene.sheet_index("target-ripple");
            for &target in targets {
                let (x, y) = foot_of(target, flipped);
                self.scene
                    .spawn(Instance::new(ripple, x, y, Tag::Selection));
            }
        }
        self.selection = wanted;
    }

    /// Draws the hint beam from one square to another.
    pub fn show_hint(&mut self, from: Square, to: Square, flipped: bool) {
        self.clear_hint();
        let (x0, y0) = center_of(from, flipped);
        let (x1, y1) = center_of(to, flipped);
        let beam = self.scene.sheet_index("hint-beam");
        let mut instance = Instance::new(beam, x0, y0, Tag::Hint);
        instance.rotation = (y1 - y0).atan2(x1 - x0);
        instance.reach = Some((x1 - x0).hypot(y1 - y0));
        self.scene.spawn(instance);
    }

    pub fn clear_hint(&mut self) {
        self.scene.remove(Tag::Hint);
    }

    /// True once, at the moment a transition covers the screen.
    pub fn take_cover(&mut self) -> bool {
        let covered = self.scene.find(self.transition).is_some_and(|instance| {
            let duration = self.scene.sheet(self.transition).duration();
            instance.age >= duration * TRANSITION_COVER
        });
        if covered && self.cover_pending {
            self.cover_pending = false;
            return true;
        }
        false
    }

    /// Draws what is playing, oldest first.
    pub fn draw(&self) {
        for instance in self.scene.playing() {
            let sheet = self.scene.sheet(instance.sheet);
            let frame = sheet.frames[sheet.frame_at(instance.age)];
            let (pivot_x, pivot_y) = sheet.pivot;
            let width = instance.reach.map_or(frame.w as f32, |reach| {
                (pivot_x + reach).min(frame.w as f32)
            });
            draw_texture_ex(
                &self.textures[instance.sheet],
                instance.x - pivot_x * instance.scale,
                instance.y - pivot_y * instance.scale,
                WHITE,
                DrawTextureParams {
                    source: Some(macroquad::math::Rect::new(
                        frame.x as f32,
                        frame.y as f32,
                        width,
                        frame.h as f32,
                    )),
                    dest_size: Some(vec2(
                        width * instance.scale,
                        frame.h as f32 * instance.scale,
                    )),
                    rotation: instance.rotation,
                    pivot: (instance.rotation != 0.0).then(|| vec2(instance.x, instance.y)),
                    ..Default::default()
                },
            );
        }
    }

    /// Where to write on the achievement toast, while there is one.
    pub fn banner_text_at(&self) -> Option<(f32, f32)> {
        let toast = self.scene.find(self.banner)?;
        let visible = (BANNER_TEXT_FROM..BANNER_TEXT_TO).contains(&toast.age);
        visible.then_some((toast.x + BANNER_TEXT_X, toast.y + 5.0))
    }
}

fn square_from(index: u8) -> Square {
    Square::new(u32::from(index))
}

/// Where a piece stands in its square: effects have their origin at the feet.
fn foot_of(square: Square, flipped: bool) -> (f32, f32) {
    let rect = square_rect(square, flipped);
    (rect.x + CELL / 2.0, rect.y + CELL - 1.0)
}

fn center_of(square: Square, flipped: bool) -> (f32, f32) {
    let rect = square_rect(square, flipped);
    (rect.x + CELL / 2.0, rect.y + CELL / 2.0)
}
