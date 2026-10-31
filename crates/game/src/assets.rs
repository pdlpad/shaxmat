//! The game's art, built into the binary so that the desktop and WASM builds load
//! it the same way, without a file system.

use std::collections::HashMap;

use macroquad::prelude::*;

const BACKDROP_PNG: &[u8] = include_bytes!("../../../assets/backdrop.png");
const BACKDROP_BLUR_PNG: &[u8] = include_bytes!("../../../assets/backdrop-blur.png");
const PIECES_PNG: &[u8] = include_bytes!("../../../assets/pieces.png");
const FONT_PNG: &[u8] = include_bytes!("../../../assets/font/manrope13.png");
pub const FONT_METRICS: &str = include_str!("../../../assets/font/manrope13.txt");
pub const PALETTE_TOML: &str = include_str!("../../../assets/palette.toml");

const ICON_16_PNG: &[u8] = include_bytes!("../../../assets/brand/icon-16.png");
const ICON_32_PNG: &[u8] = include_bytes!("../../../assets/brand/icon-32.png");
const ICON_64_PNG: &[u8] = include_bytes!("../../../assets/brand/icon-64.png");

/// The icon for the title bar and the taskbar.
pub fn window_icon() -> miniquad::conf::Icon {
    fn rgba<const N: usize>(png: &[u8]) -> [u8; N] {
        let image = Image::from_file_with_format(png, Some(ImageFormat::Png))
            .expect("the icons are valid PNG");
        image.bytes.try_into().expect("the icons have fixed sizes")
    }
    miniquad::conf::Icon {
        small: rgba(ICON_16_PNG),
        medium: rgba(ICON_32_PNG),
        big: rgba(ICON_64_PNG),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub advance: u16,
}

/// Where the glyphs of the baked font are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontMetrics {
    pub ascent: u16,
    pub height: u16,
    pub glyphs: HashMap<char, Glyph>,
}

impl FontMetrics {
    /// Parses the output of `tools/bake_font.py`.
    pub fn parse(text: &str) -> Self {
        let mut lines = text.lines();
        let mut header = |name: &str| -> u16 {
            let line = lines.next().expect("the metrics have a header");
            let value = line.strip_prefix(name).expect("the header is in order");
            value.trim().parse().expect("header values are numbers")
        };
        let ascent = header("ascent");
        let height = header("height");
        let glyphs = lines
            .map(|line| {
                let numbers: Vec<u32> = line
                    .split_whitespace()
                    .map(|n| n.parse().expect("glyph values are numbers"))
                    .collect();
                let [code, x, y, width, advance] = numbers[..] else {
                    panic!("a glyph line has five numbers: {line}");
                };
                let char = char::from_u32(code).expect("glyph codes are characters");
                let glyph = Glyph {
                    x: x as u16,
                    y: y as u16,
                    width: width as u16,
                    advance: advance as u16,
                };
                (char, glyph)
            })
            .collect();
        FontMetrics {
            ascent,
            height,
            glyphs,
        }
    }

    /// Width of a line of text, in pixels.
    pub fn measure(&self, text: &str) -> f32 {
        text.chars()
            .filter_map(|char| self.glyphs.get(&char))
            .map(|glyph| f32::from(glyph.advance))
            .sum()
    }
}

/// The loaded textures.
pub struct Assets {
    pub backdrop: Texture2D,
    pub backdrop_blur: Texture2D,
    pub pieces: Texture2D,
    pub font: Texture2D,
    pub metrics: FontMetrics,
}

impl Assets {
    pub fn load() -> Self {
        let texture = |png: &[u8]| {
            let texture = Texture2D::from_file_with_format(png, Some(ImageFormat::Png));
            texture.set_filter(FilterMode::Nearest);
            texture
        };
        Assets {
            backdrop: texture(BACKDROP_PNG),
            backdrop_blur: texture(BACKDROP_BLUR_PNG),
            pieces: texture(PIECES_PNG),
            font: texture(FONT_PNG),
            metrics: FontMetrics::parse(FONT_METRICS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_cover_the_ui_alphabets() {
        let metrics = FontMetrics::parse(FONT_METRICS);
        for char in "SHAXMAT Nf3 exd8=Q+ O-O-O 0123456789 - Ходы: шах! Чёрные Ёё".chars()
        {
            assert!(metrics.glyphs.contains_key(&char), "no glyph for {char}");
        }
        assert!(metrics.height >= 13);
        assert!(metrics.measure("Шах") > metrics.measure("Ш"));
    }

    #[test]
    fn the_window_icon_decodes_with_transparent_corners() {
        let icon = window_icon();
        assert_eq!(icon.big[3], 0);
        let center = (32 * 64 + 32) * 4;
        assert_eq!(icon.big[center + 3], 255);
    }

    #[test]
    fn parses_a_small_font() {
        let metrics = FontMetrics::parse("ascent 10\nheight 13\n65 1 2 8 7\n");
        assert_eq!(metrics.ascent, 10);
        assert_eq!(
            metrics.glyphs[&'A'],
            Glyph {
                x: 1,
                y: 2,
                width: 8,
                advance: 7
            }
        );
        assert_eq!(metrics.measure("AA?"), 14.0);
    }
}
