//! The art is checked, not trusted: sprites sit on the 24x24 grid and use only
//! palette colors, with no half-transparent pixels. This stands in for the Pixel
//! Snapper step, which suits generated art, not sprites drawn pixel by pixel.

use std::collections::HashSet;

use macroquad::prelude::{Image, ImageFormat};

const PALETTE: &str = include_str!("../../../assets/palette.toml");
const ATLAS: &[u8] = include_bytes!("../../../assets/pieces.png");
const FONT: &[u8] = include_bytes!("../../../assets/font/manrope13.png");

const SPRITE: usize = 24;

fn palette_colors() -> HashSet<[u8; 3]> {
    PALETTE
        .split('"')
        .filter(|part| part.len() == 7)
        .filter_map(|part| part.strip_prefix('#'))
        .map(|hex| {
            let value = u32::from_str_radix(hex, 16).unwrap();
            [(value >> 16) as u8, (value >> 8) as u8, value as u8]
        })
        .collect()
}

#[test]
fn the_atlas_holds_twelve_sprites() {
    let image = Image::from_file_with_format(ATLAS, Some(ImageFormat::Png)).unwrap();
    assert_eq!(usize::from(image.width), 6 * SPRITE);
    assert_eq!(usize::from(image.height), 2 * SPRITE);
}

#[test]
fn sprites_use_only_palette_colors_and_hard_edges() {
    let image = Image::from_file_with_format(ATLAS, Some(ImageFormat::Png)).unwrap();
    let palette = palette_colors();
    for pixel in image.bytes.as_chunks::<4>().0 {
        match pixel[3] {
            0 => {}
            255 => assert!(
                palette.contains(&[pixel[0], pixel[1], pixel[2]]),
                "{pixel:?}"
            ),
            alpha => panic!("half-transparent pixel: {alpha}"),
        }
    }
}

#[test]
fn every_sprite_is_centered_in_its_cell_with_a_margin() {
    let image = Image::from_file_with_format(ATLAS, Some(ImageFormat::Png)).unwrap();
    let opaque = |x: usize, y: usize| image.bytes[(y * usize::from(image.width) + x) * 4 + 3] != 0;
    for row in 0..2 {
        for column in 0..6 {
            let filled = (0..SPRITE)
                .flat_map(|y| (0..SPRITE).map(move |x| (x, y)))
                .filter(|&(x, y)| opaque(column * SPRITE + x, row * SPRITE + y))
                .count();
            assert!(filled > 60, "sprite {column},{row} is nearly empty");
            for i in 0..SPRITE {
                assert!(!opaque(column * SPRITE, row * SPRITE + i), "left edge");
                assert!(
                    !opaque(column * SPRITE + 23, row * SPRITE + i),
                    "right edge"
                );
            }
        }
    }
}

#[test]
fn the_font_is_two_tone() {
    let image = Image::from_file_with_format(FONT, Some(ImageFormat::Png)).unwrap();
    assert!(
        image
            .bytes
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[3] == 0 || p[3] == 255)
    );
}
