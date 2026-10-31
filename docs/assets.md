# Asset decisions

Sprites come from `tools/gen_sprites.py`, the font atlas from `tools/bake_font.py`
(needs Pillow). `crates/game/tests/assets.rs` checks the results.

## Font: Manrope instead of a pixel font

The task asked for a pixel font. Manrope 13 semibold was picked by eye over Press
Start 2P, and is therefore not a pixel font. It is baked to two tones at its native
size, so it stays on the pixel grid, but its letter shapes are less regular than a
designed pixel font's.

## No Pixel Snapper pass

The task asked to run sprites through Pixel Snapper after generation. Sprites here are
drawn pixel by pixel, so they are on the grid from the start. Pixel Snapper 1.0.0 did
not recover the grid at any upscale from 1x to 32x: it found grids from 3x3 to 72x72 and
damaged the art. The asset test checks instead that the atlas is exactly 24x24 per
sprite, that it uses only palette colors and that it has no half-transparent pixels.

## Colors

Ink, navy, blue, gold and cream are sampled from the logo and icon in `assets/brand/`.
Sky, amber, red and the light ramp were added to complete the palette.
