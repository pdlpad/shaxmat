#!/usr/bin/env python3
"""Bakes the window icons from assets/brand/icon.webp. Needs Pillow.

The source has white corners outside its rounded square; they become transparent.
"""

import os

from PIL import Image, ImageDraw

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "brand")
SIZES = (16, 32, 64)
CORNER_TOLERANCE = 60


def main():
    source = Image.open(os.path.join(ROOT, "icon.webp")).convert("RGBA")
    width, height = source.size
    # Flood fill marks the outside, starting from the four corners.
    outside = (255, 0, 255, 0)
    for corner in ((0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1)):
        ImageDraw.floodfill(source, corner, outside, thresh=CORNER_TOLERANCE)
    pixels = source.load()
    for y in range(height):
        for x in range(width):
            if pixels[x, y] == outside:
                pixels[x, y] = (0, 0, 0, 0)

    # Downscaling with alpha premultiplied keeps the white fringe out of the edge.
    premultiplied = source.convert("RGBa")
    for size in SIZES:
        icon = premultiplied.resize((size, size), Image.Resampling.LANCZOS).convert("RGBA")
        icon.save(os.path.join(ROOT, f"icon-{size}.png"))


if __name__ == "__main__":
    main()
