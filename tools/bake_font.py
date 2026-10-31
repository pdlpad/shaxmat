#!/usr/bin/env python3
"""Bakes the UI font into assets/font/: a white-on-transparent atlas plus metrics.

The result is two-tone on purpose, so the text sits on the pixel grid like the
sprites. Needs Pillow: `pip install pillow`.

Metrics format: an `ascent N` line, a `height N` line, then one line per glyph:
`codepoint x y width advance`.
"""

import math
import os

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "font")
SIZE = 13
ALPHA_THRESHOLD = 128
ATLAS_WIDTH = 256
CHARS = (
    [chr(c) for c in range(0x20, 0x7F)]
    + [chr(c) for c in range(0x410, 0x450)]
    + ["Ё", "ё", "—", "…", "№", "‘", "’"]
)


def font_for(char, latin, cyrillic):
    return cyrillic if "Ѐ" <= char <= "ӿ" else latin


def main():
    latin = ImageFont.truetype(os.path.join(ROOT, "manrope-latin-600.woff"), SIZE)
    cyrillic = ImageFont.truetype(os.path.join(ROOT, "manrope-cyrillic-600.woff"), SIZE)
    ascent, descent = latin.getmetrics()
    height = ascent + descent

    rows = []
    x, y = 0, 0
    atlas = Image.new("L", (ATLAS_WIDTH, ATLAS_WIDTH), 0)
    draw = ImageDraw.Draw(atlas)
    for char in CHARS:
        font = font_for(char, latin, cyrillic)
        advance = round(font.getlength(char))
        width = max(math.ceil(font.getlength(char)) + 1, 1)
        if x + width > ATLAS_WIDTH:
            x, y = 0, y + height
        draw.text((x, y + ascent), char, font=font, fill=255, anchor="ls")
        rows.append(f"{ord(char)} {x} {y} {width} {advance}")
        x += width

    used_height = y + height
    mask = atlas.crop((0, 0, ATLAS_WIDTH, used_height))
    mask = mask.point(lambda value: 255 if value >= ALPHA_THRESHOLD else 0)
    image = Image.new("RGBA", mask.size, (255, 255, 255, 0))
    image.putalpha(mask)
    image.save(os.path.join(ROOT, "manrope13.png"))

    with open(os.path.join(ROOT, "manrope13.txt"), "w", encoding="ascii", newline="\n") as file:
        file.write(f"ascent {ascent}\nheight {height}\n")
        file.write("\n".join(rows) + "\n")


if __name__ == "__main__":
    main()
