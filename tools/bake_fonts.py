#!/usr/bin/env python3
"""Bake the OFL fonts into BMFont atlases: game-toolkit's TrueType text cannot pick a family.

usage: bake_fonts.py CAVEAT_DIR NEWSREADER_DIR OUT_DIR   (the @fontsource files/ folders)
"""
import io
import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from PIL import Image, ImageFont

CHARS = [chr(c) for c in range(32, 127)]
ATLAS_W = 1024


def load(path, size):
    ttf = TTFont(path)
    ttf.flavor = None
    data = io.BytesIO()
    ttf.save(data)
    data.seek(0)
    return ImageFont.truetype(data, size)


def bake(src, size, out, name):
    font = load(src, size)
    ascent, descent = font.getmetrics()
    cells, x, y, row = [], 2, 2, 0
    for c in CHARS:
        box = font.getbbox(c, anchor="ls")
        w, h = box[2] - box[0], box[3] - box[1]
        if x + w + 2 > ATLAS_W:
            x, y, row = 2, y + row + 2, 0
        cells.append((c, x, y, w, h, box))
        x += w + 2
        row = max(row, h)
    atlas = Image.new("RGBA", (ATLAS_W, y + row + 2), (255, 255, 255, 0))
    fnt = [
        f'info face="{name}" size={size}',
        f"common lineHeight={ascent + descent} base={ascent} scaleW={atlas.width} "
        f"scaleH={atlas.height} pages=1",
        f'page id=0 file="{name}.png"',
        f"chars count={len(cells)}",
    ]
    for c, cx, cy, w, h, box in cells:
        if w and h:
            glyph = Image.new("L", (w, h), 0)
            from PIL import ImageDraw

            ImageDraw.Draw(glyph).text((-box[0], -box[1]), c, font=font, fill=255, anchor="ls")
            atlas.paste((255, 255, 255, 255), (cx, cy), glyph)
        fnt.append(
            f"char id={ord(c)} x={cx} y={cy} width={w} height={h} xoffset={box[0]} "
            f"yoffset={ascent + box[1]} xadvance={round(font.getlength(c))} page=0 chnl=15"
        )
    kerns = []
    for a in CHARS:
        for b in CHARS:
            k = round(font.getlength(a + b) - font.getlength(a) - font.getlength(b))
            if k:
                kerns.append(f"kerning first={ord(a)} second={ord(b)} amount={k}")
    fnt.append(f"kernings count={len(kerns)}")
    atlas.save(out / f"{name}.png")
    (out / f"{name}.fnt").write_text("\n".join(fnt + kerns) + "\n")


caveat, newsreader, out = map(Path, sys.argv[1:4])
out.mkdir(parents=True, exist_ok=True)
bake(caveat / "caveat-latin-400-normal.woff", 29, out, "hand")
bake(caveat / "caveat-latin-700-normal.woff", 50, out, "heading")
bake(newsreader / "newsreader-latin-500-normal.woff", 17, out, "print")
