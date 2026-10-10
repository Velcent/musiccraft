#!/usr/bin/env python3
"""Generates MusicCraft's placeholder app icon (original artwork, drawn here).

Writes musiccraft-1024.png, musiccraft-512.png and musiccraft.ico next to this
script. Requires Pillow. Replace with real artwork whenever you have it; every
packaging script only reads these three files.
"""
from pathlib import Path

from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parent
SIZE = 1024
BG_TOP, BG_BOTTOM = (38, 44, 58), (20, 23, 31)
LIT = (255, 150, 40)
DIM = (74, 84, 104)
# 4 rows x 8 steps of a drum-machine style grid; True = lit step.
PATTERN = [
    "x...x...",
    "..x...x.",
    "x.x.x.x.",
    "....x..x",
]


def vertical_gradient() -> Image.Image:
    img = Image.new("RGB", (SIZE, SIZE))
    px = img.load()
    for y in range(SIZE):
        t = y / (SIZE - 1)
        row = tuple(round(a + (b - a) * t) for a, b in zip(BG_TOP, BG_BOTTOM))
        for x in range(SIZE):
            px[x, y] = row
    return img


def render() -> Image.Image:
    base = vertical_gradient().convert("RGBA")
    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, SIZE - 1, SIZE - 1), radius=SIZE // 5, fill=255)
    base.putalpha(mask)
    d = ImageDraw.Draw(base)
    cols, rows = 8, len(PATTERN)
    margin = SIZE * 0.14
    gap = SIZE * 0.025
    cell_w = (SIZE - 2 * margin - gap * (cols - 1)) / cols
    cell_h = cell_w
    grid_h = rows * cell_h + (rows - 1) * gap
    top = (SIZE - grid_h) / 2
    for r, line in enumerate(PATTERN):
        for c, ch in enumerate(line):
            x0 = margin + c * (cell_w + gap)
            y0 = top + r * (cell_h + gap)
            d.rounded_rectangle(
                (x0, y0, x0 + cell_w, y0 + cell_h),
                radius=cell_w * 0.22,
                fill=LIT if ch == "x" else DIM,
            )
    return base


def main() -> None:
    icon = render()
    icon.save(OUT / "musiccraft-1024.png")
    icon.resize((512, 512), Image.LANCZOS).save(OUT / "musiccraft-512.png")
    icon.save(OUT / "musiccraft.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    print("wrote icons to", OUT)


if __name__ == "__main__":
    main()
