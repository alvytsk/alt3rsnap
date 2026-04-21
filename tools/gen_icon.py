#!/usr/bin/env python3
"""Generate assets/icon.ico as a multi-size (16/32/48/256) ICO with BMP
payloads. Stdlib-only. Draws a stylized "A" on a gradient background so the
icon is obviously *not* the Windows default at any shell size.

Run: python3 tools/gen_icon.py
Output: assets/icon.ico
"""
from __future__ import annotations
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "icon.ico"

SIZES = [16, 32, 48, 256]

# Accent palette: indigo -> violet gradient, white glyph, black outline.
BG_TOP = (0x2A, 0x1B, 0x5E)     # BGR: deep indigo
BG_BOT = (0xC2, 0x2B, 0x9B)     # BGR: violet-magenta
GLYPH  = (0xFF, 0xFF, 0xFF)     # white
SHADOW = (0x00, 0x00, 0x00)     # black


def lerp(a: int, b: int, t: float) -> int:
    return int(round(a + (b - a) * t))


def bg_pixel(x: int, y: int, size: int) -> tuple[int, int, int]:
    t = y / max(1, size - 1)
    return (
        lerp(BG_TOP[0], BG_BOT[0], t),
        lerp(BG_TOP[1], BG_BOT[1], t),
        lerp(BG_TOP[2], BG_BOT[2], t),
    )


def draw_a(size: int) -> list[list[tuple[int, int, int, int]]]:
    """Return size×size BGRA rows (top-down)."""
    img = [[(0, 0, 0, 0)] * size for _ in range(size)]

    # Rounded square mask -> fills background with alpha.
    radius = max(2, size // 8)
    for y in range(size):
        for x in range(size):
            # Rounded-rect distance test
            dx = max(0, radius - x, x - (size - 1 - radius))
            dy = max(0, radius - y, y - (size - 1 - radius))
            if dx * dx + dy * dy <= radius * radius:
                b, g, r = bg_pixel(x, y, size)
                img[y][x] = (b, g, r, 0xFF)

    # Stylized "A": two diagonal bars + horizontal crossbar.
    cx = size / 2.0
    top_y = size * 0.18
    bot_y = size * 0.82
    half_top = size * 0.04
    half_bot = size * 0.32
    stroke = max(1.0, size / 12.0)

    # Outline color: black shadow with 2px offset, then white.
    def paint_bar(x0, y0, x1, y1, color):
        # Paint a thick line from (x0,y0) to (x1,y1).
        steps = int(max(abs(x1 - x0), abs(y1 - y0)) * 2) + 1
        for i in range(steps + 1):
            t = i / steps
            px = x0 + (x1 - x0) * t
            py = y0 + (y1 - y0) * t
            r0 = int(stroke + 0.5)
            for oy in range(-r0, r0 + 1):
                for ox in range(-r0, r0 + 1):
                    if ox * ox + oy * oy <= stroke * stroke:
                        xi = int(px + ox)
                        yi = int(py + oy)
                        if 0 <= xi < size and 0 <= yi < size and img[yi][xi][3] == 0xFF:
                            img[yi][xi] = (*color, 0xFF)

    # Shadow pass (offset down-right by ~stroke/3)
    off = max(1, int(stroke / 3))
    paint_bar(cx - half_top + off, top_y + off, cx - half_bot + off, bot_y + off, SHADOW)
    paint_bar(cx + half_top + off, top_y + off, cx + half_bot + off, bot_y + off, SHADOW)
    paint_bar(cx - half_bot * 0.55 + off, size * 0.62 + off,
              cx + half_bot * 0.55 + off, size * 0.62 + off, SHADOW)

    # Glyph pass
    paint_bar(cx - half_top, top_y, cx - half_bot, bot_y, GLYPH)
    paint_bar(cx + half_top, top_y, cx + half_bot, bot_y, GLYPH)
    paint_bar(cx - half_bot * 0.55, size * 0.62,
              cx + half_bot * 0.55, size * 0.62, GLYPH)

    return img


def encode_bmp_icon_image(img: list[list[tuple[int, int, int, int]]]) -> bytes:
    """Encode image as DIB (BITMAPINFOHEADER + XOR BGRA + AND mask), as used
    inside ICO containers. Height field is doubled per ICO spec."""
    h = len(img)
    w = len(img[0])
    # BITMAPINFOHEADER (40 bytes)
    header = struct.pack(
        "<IiiHHIIiiII",
        40,          # biSize
        w,           # biWidth
        h * 2,       # biHeight (ICO quirk: includes AND mask)
        1,           # biPlanes
        32,          # biBitCount
        0,           # biCompression (BI_RGB)
        0,           # biSizeImage
        0, 0,        # PPM x/y
        0, 0,        # colors used/important
    )
    # XOR bitmap: BGRA, bottom-up
    xor = bytearray()
    for y in range(h - 1, -1, -1):
        row = img[y]
        for px in row:
            b, g, r, a = px
            xor += bytes((b, g, r, a))
    # AND mask: 1 bit per pixel, rows padded to 4 bytes, bottom-up.
    # 0 = show XOR pixel, 1 = transparent. We set 1 where alpha == 0.
    row_bits = w
    row_bytes = ((row_bits + 31) // 32) * 4
    and_mask = bytearray()
    for y in range(h - 1, -1, -1):
        row = img[y]
        bits = bytearray(row_bytes)
        for x in range(w):
            if row[x][3] == 0:
                bits[x // 8] |= 0x80 >> (x % 8)
        and_mask += bits
    return header + bytes(xor) + bytes(and_mask)


def encode_png(img: list[list[tuple[int, int, int, int]]]) -> bytes:
    """Encode as RGBA PNG using stdlib zlib. Filter type 0 (None) per scanline."""
    h = len(img)
    w = len(img[0])
    sig = b"\x89PNG\r\n\x1a\n"

    def chunk(ctype: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(ctype + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + ctype + data + struct.pack(">I", crc)

    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)  # 8-bit RGBA
    raw = bytearray()
    for row in img:
        raw.append(0)  # filter: None
        for b, g, r, a in row:
            raw += bytes((r, g, b, a))
    idat = zlib.compress(bytes(raw), level=9)
    return sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")


def build_ico() -> bytes:
    images = []
    for s in SIZES:
        rgba = draw_a(s)
        # PNG for 256 (Windows Vista+ convention, far smaller); BMP for the rest.
        if s >= 256:
            images.append((s, encode_png(rgba)))
        else:
            images.append((s, encode_bmp_icon_image(rgba)))
    # ICONDIR
    out = bytearray()
    out += struct.pack("<HHH", 0, 1, len(images))
    # ICONDIRENTRY table
    entry_size = 16
    offset = 6 + entry_size * len(images)
    for size, payload in images:
        w = 0 if size == 256 else size  # 0 means 256
        h = 0 if size == 256 else size
        out += struct.pack(
            "<BBBBHHII",
            w, h,
            0,       # color count
            0,       # reserved
            1,       # planes
            32,      # bpp
            len(payload),
            offset,
        )
        offset += len(payload)
    # Image payloads
    for _, payload in images:
        out += payload
    return bytes(out)


def main() -> None:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    data = build_ico()
    OUT.write_bytes(data)
    print(f"wrote {OUT} ({len(data)} bytes, sizes={SIZES})")


if __name__ == "__main__":
    main()
