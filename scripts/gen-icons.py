"""Generate placeholder PNG + ICO files for Tauri build.

We hand-roll a tiny 32x32 RGBA PNG (a blue square with white "AS" written as
solid pixels — no Pillow dependency). Then build a 1-image ICO that contains
the same pixels, plus 128 and 128@2x PNGs for the other declared sizes.

This is enough for tauri-build's Windows Resource generation to succeed.
"""
import os
import struct
import zlib

OUT = r"C:\Users\19114\.minimax-agent-cn\projects\admin-suite\src-tauri\icons"
os.makedirs(OUT, exist_ok=True)


def make_png(width: int, height: int) -> bytes:
    """Build a minimal RGBA PNG: blue background, white pixel cross in the centre."""
    bg = (37, 99, 235, 255)       # indigo-600
    fg = (255, 255, 255, 255)

    rows = []
    cx, cy = width // 2, height // 2
    arm = max(1, min(width, height) // 6)

    for y in range(height):
        row = bytearray([0])  # filter byte (None)
        for x in range(width):
            in_cross = (abs(x - cx) <= arm) or (abs(y - cy) <= arm)
            r, g, b, a = fg if in_cross else bg
            row.extend((r, g, b, a))
        rows.append(bytes(row))

    raw = b"".join(rows)

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)  # 8-bit RGBA
    idat = zlib.compress(raw, 9)
    return sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")


def make_ico(width: int, height: int, png_bytes: bytes) -> bytes:
    """Wrap a PNG into a 1-image ICO container (Vista+ supports PNG-in-ICO)."""
    # ICONDIR
    header = struct.pack("<HHH", 0, 1, 1)
    # ICONDIRENTRY
    w = 0 if width >= 256 else width
    h = 0 if height >= 256 else height
    entry = struct.pack(
        "<BBBBHHII",
        w, h, 0, 0, 1, 32,
        len(png_bytes),
        6 + 16,  # offset to image data (header + 1 entry)
    )
    return header + entry + png_bytes


# Sizes that tauri.conf.json lists.
sizes = {"32x32.png": 32, "128x128.png": 128, "128x128@2x.png": 256}
for name, size in sizes.items():
    data = make_png(size, size)
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)
    print(f"wrote {name} ({size}x{size}, {len(data)} bytes)")

# icon.ico (use 64x64 for compatibility)
ico_png = make_png(64, 64)
ico_bytes = make_ico(64, 64, ico_png)
with open(os.path.join(OUT, "icon.ico"), "wb") as f:
    f.write(ico_bytes)
print(f"wrote icon.ico (64x64 PNG-in-ICO, {len(ico_bytes)} bytes)")

# macOS ICNS is optional on non-mac builds; tauri-build skips it when on Windows.
# Provide a minimal placeholder just in case the bundler is invoked.
icns_header = b"icns\x00\x00\x00\x08"
with open(os.path.join(OUT, "icon.icns"), "wb") as f:
    f.write(icns_header)
print("wrote icon.icns (8-byte stub)")

print("\nAll icons generated in:", OUT)