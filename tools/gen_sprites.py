#!/usr/bin/env python3
"""Generates the piece sprites and board tiles in assets/ from the masks below.

Uses only the standard library. Each mask is a silhouette; the outline and the
shading come from fixed rules, so that all pieces share one light source (top
left) and one line weight.

Mask characters: '.' empty, '#' body, 'x' inner line in the outline color.
Symmetric pieces give the left 12 columns; the knight gives all 24.

    python tools/gen_sprites.py            writes assets/
    python tools/gen_sprites.py --preview records/stage4-style-a.png a
    python tools/gen_sprites.py --board records/stage4-board.png a classic
"""

import os
import struct
import sys
import tomllib
import zlib

SIZE = 24
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def load_palette():
    with open(os.path.join(ROOT, "assets", "palette.toml"), "rb") as file:
        data = tomllib.load(file)
    colors = {name: parse_color(value) for name, value in data["colors"].items()}
    return colors, [parse_color(value) for value in data["ramp"]["light"]]


def parse_color(text):
    text = text.lstrip("#")
    return tuple(int(text[i : i + 2], 16) for i in (0, 2, 4)) + (255,)


PALETTE, RAMP = load_palette()
C = PALETTE
HIGH = RAMP[3]

PAWN = """
............
............
............
............
............
............
............
..........##
.........###
.........###
.........###
..........##
.........###
..........##
..........##
.........###
.........###
........####
.......#####
.......#####
......######
......######
............
............
"""

ROOK = """
............
............
............
.....##..###
.....##..###
.....#######
.....#######
......######
.......#####
.......#####
.......#####
.......#####
.......#####
.......#####
.......#####
.......#####
......######
......######
.....#######
.....#######
....########
....########
............
............
"""

BISHOP = """
............
............
...........#
..........##
...........#
.........###
.........###
........####
........###x
........##xx
........###x
........####
.........###
.......#####
.........###
.........###
.........###
........####
.......#####
......######
.....#######
.....#######
............
............
"""

QUEEN = """
............
...........#
...##.....##
...##..#...#
....#..##..#
....#..##.##
....##.##.##
....########
....########
.....#######
.....#######
......######
.....#######
.......#####
.......#####
.......#####
.......#####
......######
.....#######
.....#######
....########
....########
............
............
"""

KING = """
............
...........#
.........###
...........#
...........#
.........###
.......#####
......######
......###x##
.......##x##
.......##x##
........####
.......#####
.........###
.........###
.........###
.........###
........####
.......#####
......######
....########
....########
............
............
"""

KNIGHT = """
........................
...........##...........
..........###...........
.........######.........
........########........
.......###x######.......
......##xx#######.......
.....##############.....
....################....
...##################...
..##x################...
..###########x#######...
...####xxxxxxx#######...
....###......#########..
............##########..
...........###########..
..........############..
.........#############..
........##############..
.......###############..
......################..
......################..
........................
........................
"""


def parse_mask(text, symmetric):
    rows = [line for line in text.split("\n") if line]
    assert len(rows) == SIZE, len(rows)
    if symmetric:
        rows = [row + row[::-1] for row in rows]
    assert all(len(row) == SIZE for row in rows), text
    return rows


MASKS = {
    "P": parse_mask(PAWN, True),
    "N": parse_mask(KNIGHT, False),
    "B": parse_mask(BISHOP, True),
    "R": parse_mask(ROOK, True),
    "Q": parse_mask(QUEEN, True),
    "K": parse_mask(KING, True),
}


def shade(mask, style):
    """Colors a silhouette: outline around it, light from the top left."""
    outline, dark, body, light = style

    def solid(x, y):
        return 0 <= x < SIZE and 0 <= y < SIZE and mask[y][x] in "#x"

    pixels = [[(0, 0, 0, 0)] * SIZE for _ in range(SIZE)]
    for y in range(SIZE):
        for x in range(SIZE):
            if mask[y][x] == "x":
                pixels[y][x] = outline
            elif mask[y][x] == "#":
                if not solid(x + 1, y) or not solid(x, y + 1):
                    pixels[y][x] = dark
                elif not solid(x - 1, y) or not solid(x, y - 1):
                    pixels[y][x] = light
                else:
                    pixels[y][x] = body
            elif any(
                solid(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
            ):
                pixels[y][x] = outline
    return pixels


# Candidate styles: (outline, shade, body, highlight) for white, then black.
STYLES = {
    "a": ((C["ink"], C["amber"], C["gold"], HIGH), (C["cream"], C["ink"], C["navy"], C["blue"])),
    "c": ((C["ink"], C["amber"], C["gold"], HIGH), (C["ink"], C["navy"], C["blue"], C["sky"])),
}
CHOSEN = "a"

# Board themes: (light square, dark square).
THEMES = {
    "classic": (C["cream"], C["blue"]),
    "night": (C["blue"], C["navy"]),
}


def blit(canvas, pixels, left, top):
    for y, row in enumerate(pixels):
        for x, color in enumerate(row):
            if color[3]:
                canvas[top + y][left + x] = color


def write_png(path, pixels, scale=1):
    height, width = len(pixels) * scale, len(pixels[0]) * scale
    raw = bytearray()
    for row in pixels:
        line = bytearray()
        for color in row:
            line += bytes(color) * scale
        for _ in range(scale):
            raw.append(0)
            raw += line

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as file:
        file.write(b"\x89PNG\r\n\x1a\n")
        file.write(chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)))
        file.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        file.write(chunk(b"IEND", b""))


def sets(style_name):
    white, black = STYLES[style_name]
    return {
        "w": {name: shade(mask, white) for name, mask in MASKS.items()},
        "b": {name: shade(mask, black) for name, mask in MASKS.items()},
    }


def preview(path, style_name):
    """Every piece on both square colors of every theme, at four times the size."""
    pieces = sets(style_name)
    order = "PNBRQK"
    columns, rows = 12, 2 * len(THEMES)
    canvas = [[C["ink"]] * (columns * SIZE) for _ in range(rows * SIZE)]
    for t, (light, dark) in enumerate(THEMES.values()):
        for side, color in enumerate("wb"):
            for i in range(columns):
                top, left = (2 * t + side) * SIZE, i * SIZE
                base = light if (i + side) % 2 == 0 else dark
                blit(canvas, [[base] * SIZE] * SIZE, left, top)
                blit(canvas, pieces[color][order[i % 6]], left, top)
    write_png(path, canvas, 4)


def board_preview(path, style_name, theme):
    """The start position at game size, scaled three times."""
    pieces = sets(style_name)
    light, dark = THEMES[theme]
    back = "RNBQKBNR"
    canvas = [[C["navy"]] * (8 * SIZE) for _ in range(8 * SIZE)]
    for rank in range(8):
        for file in range(8):
            left, top = file * SIZE, rank * SIZE
            base = light if (rank + file) % 2 == 0 else dark
            blit(canvas, [[base] * SIZE] * SIZE, left, top)
            name = {0: back[file], 1: "P", 6: "P", 7: back[file]}.get(rank)
            if name:
                blit(canvas, pieces["b" if rank < 2 else "w"][name], left, top)
    write_png(path, canvas, 3)


def write_assets():
    pieces = sets(CHOSEN)
    atlas = [[(0, 0, 0, 0)] * (6 * SIZE) for _ in range(2 * SIZE)]
    for row, color in enumerate("wb"):
        for column, name in enumerate("PNBRQK"):
            sprite = pieces[color][name]
            write_png(os.path.join(ROOT, "assets", "pieces", f"{color}{name}.png"), sprite)
            blit(atlas, sprite, column * SIZE, row * SIZE)
    write_png(os.path.join(ROOT, "assets", "pieces.png"), atlas)


def main():
    if len(sys.argv) == 4 and sys.argv[1] == "--preview":
        preview(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 5 and sys.argv[1] == "--board":
        board_preview(sys.argv[2], sys.argv[3], sys.argv[4])
    else:
        write_assets()


if __name__ == "__main__":
    main()
