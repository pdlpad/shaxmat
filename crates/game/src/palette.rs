//! The project palette from `assets/palette.toml`, the one source of colors.

use macroquad::color::Color;

use crate::assets::PALETTE_TOML;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub light: Color,
    pub dark: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub ink: Color,
    pub navy: Color,
    pub blue: Color,
    pub sky: Color,
    pub cream: Color,
    pub gold: Color,
    pub amber: Color,
    pub red: Color,
    pub themes: Vec<Theme>,
}

impl Palette {
    pub fn load() -> Self {
        Self::parse(PALETTE_TOML)
    }

    fn parse(text: &str) -> Self {
        let root: toml::Table = text.parse().expect("the palette is valid TOML");
        let colors = root["colors"].as_table().expect("[colors] is a table");
        let named = |name: &str| {
            let value = colors[name].as_str().expect("colors are strings");
            parse_color(value)
        };
        let themes = root["themes"].as_table().expect("[themes] is a table");
        Palette {
            ink: named("ink"),
            navy: named("navy"),
            blue: named("blue"),
            sky: named("sky"),
            cream: named("cream"),
            gold: named("gold"),
            amber: named("amber"),
            red: named("red"),
            themes: ["classic", "night"]
                .map(|name| {
                    let theme = themes[name].as_table().expect("themes are tables");
                    let color =
                        |square: &str| named(theme[square].as_str().expect("squares name a color"));
                    Theme {
                        light: color("light"),
                        dark: color("dark"),
                    }
                })
                .to_vec(),
        }
    }
}

/// Parses `#RRGGBB`.
pub fn parse_color(text: &str) -> Color {
    let digits = text.strip_prefix('#').expect("colors start with #");
    let value = u32::from_str_radix(digits, 16).expect("colors are hexadecimal");
    Color::from_rgba((value >> 16) as u8, (value >> 8) as u8, value as u8, 255)
}

/// The same color at another opacity.
pub fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors() {
        let color = parse_color("#FF0080");
        assert_eq!((color.r, color.g), (1.0, 0.0));
        assert!((color.b - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn the_project_palette_loads() {
        let palette = Palette::load();
        assert_eq!(palette.themes.len(), 2);
        assert_eq!(palette.gold, parse_color("#FCCB3A"));
        assert_eq!(palette.themes[0].light, palette.cream);
        assert_eq!(palette.themes[1].dark, palette.navy);
    }
}
