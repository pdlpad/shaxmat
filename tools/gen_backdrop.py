#!/usr/bin/env python3
"""Generates the screen backdrop and its blurred twin, which glass panels show.

Uses only the standard library:

    python tools/gen_backdrop.py
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_sprites import C, ROOT, write_png

WIDTH, HEIGHT = 480, 270
TILE = 24
BLUR_RADIUS = 6
# Soft lights behind the glass: x, y, radius, color, strength.
LIGHTS = (
    (372, 60, 150, C["blue"], 0.85),
    (448, 232, 120, C["amber"], 0.3),
    (60, 250, 170, C["blue"], 0.5),
    (300, 250, 90, C["sky"], 0.22),
)
# Ordered dither, so that the glow stays on the pixel grid.
BAYER = ((0, 8, 2, 10), (12, 4, 14, 6), (3, 11, 1, 9), (15, 7, 13, 5))
LEVELS = 5


def mix(a, b, k):
    return tuple(a[i] + (b[i] - a[i]) * k for i in range(3))


def base_color(x, y):
    color = C["navy"][:3]
    for lx, ly, radius, light, strength in LIGHTS:
        d = math.hypot(x - lx, y - ly) / radius
        if d < 1:
            k = (1 - d) ** 2 * strength
            # Fixed steps with a dither between them, not a smooth gradient.
            step = k * LEVELS + BAYER[y % 4][x % 4] / 16
            color = mix(color, light[:3], int(step) / LEVELS)
    # The icon's checker, barely there.
    if (x // TILE + y // TILE) % 2 == 0:
        color = mix(color, C["blue"][:3], 0.1)
    return color


def blur(pixels):
    """A box blur, run twice, with one pass per axis."""

    def blur_rows(source, width, height):
        out = [[None] * width for _ in range(height)]
        for y in range(height):
            for x in range(width):
                total, count = [0.0, 0.0, 0.0], 0
                for dx in range(-BLUR_RADIUS, BLUR_RADIUS + 1):
                    sample = source[y][min(width - 1, max(0, x + dx))]
                    total = [t + s for t, s in zip(total, sample)]
                    count += 1
                out[y][x] = tuple(t / count for t in total)
        return out

    def transpose(source):
        return [list(row) for row in zip(*source)]

    for _ in range(2):
        pixels = blur_rows(pixels, WIDTH, HEIGHT)
        pixels = transpose(blur_rows(transpose(pixels), HEIGHT, WIDTH))
    return pixels


def to_png(pixels):
    return [[tuple(int(v) for v in color) + (255,) for color in row] for row in pixels]


def main():
    sharp = [[base_color(x, y) for x in range(WIDTH)] for y in range(HEIGHT)]
    write_png(os.path.join(ROOT, "assets", "backdrop.png"), to_png(sharp))
    write_png(os.path.join(ROOT, "assets", "backdrop-blur.png"), to_png(blur(sharp)))


if __name__ == "__main__":
    main()
