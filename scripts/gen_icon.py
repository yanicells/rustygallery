#!/usr/bin/env python3
"""Draw the gallery mark at several sizes. No third-party deps."""

from __future__ import annotations

import pathlib
import struct
import subprocess
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "assets" / "icon"

RUST = (196, 92, 56)
CREAM = (247, 239, 228)
INK = (36, 28, 24)
WHITE = (255, 255, 255)


def png(path: pathlib.Path, pixels: list[tuple[int, int, int, int]], size: int) -> None:
    raw = b"".join(b"\x00" + b"".join(struct.pack("BBBB", *px) for px in row) for row in (pixels[y * size : (y + 1) * size] for y in range(size)))

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def inside_round_rect(x: float, y: float, size: int, radius: float) -> bool:
    if x < 0 or y < 0 or x >= size or y >= size:
        return False
    r = min(radius, size / 2)
    if x < r and y < r:
        return (x - r) ** 2 + (y - r) ** 2 <= r * r
    if x > size - r and y < r:
        return (x - (size - r)) ** 2 + (y - r) ** 2 <= r * r
    if x < r and y > size - r:
        return (x - r) ** 2 + (y - (size - r)) ** 2 <= r * r
    if x > size - r and y > size - r:
        return (x - (size - r)) ** 2 + (y - (size - r)) ** 2 <= r * r
    return True


def paint(size: int, template: bool) -> list[tuple[int, int, int, int]]:
    cx = cy = size / 2
    outer = size * 0.32
    pupil = size * 0.13
    radius = size * 0.22
    out = []
    for y in range(size):
        for x in range(size):
            px = x + 0.5 - cx
            py = y + 0.5 - cy
            in_tile = inside_round_rect(x + 0.5, y + 0.5, size, radius)
            in_lens = px * px + py * py <= outer * outer
            dx = px + outer * 0.22
            dy = py + outer * 0.18
            in_pupil = dx * dx + dy * dy <= pupil * pupil
            if template:
                if in_lens and not in_pupil:
                    out.append((*WHITE, 255))
                else:
                    out.append((0, 0, 0, 0))
            elif not in_tile:
                out.append((0, 0, 0, 0))
            elif in_pupil:
                out.append((*INK, 255))
            elif in_lens:
                out.append((*CREAM, 255))
            else:
                out.append((*RUST, 255))
    return out


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    sizes = (16, 32, 64, 128, 256, 512, 1024)
    for size in sizes:
        png(OUT / f"icon_{size}.png", paint(size, False), size)
    png(OUT / "icon_1024.png", paint(1024, False), 1024)
    png(OUT / "tray_template.png", paint(32, True), 32)
    png(OUT / "app.png", paint(256, False), 256)

    iconset = OUT / "gallery.iconset"
    iconset.mkdir(exist_ok=True)
    mapping = {
        16: ["icon_16x16.png"],
        32: ["icon_16x16@2x.png", "icon_32x32.png"],
        64: ["icon_32x32@2x.png"],
        128: ["icon_128x128.png"],
        256: ["icon_128x128@2x.png", "icon_256x256.png"],
        512: ["icon_256x256@2x.png", "icon_512x512.png"],
        1024: ["icon_512x512@2x.png"],
    }
    for size, names in mapping.items():
        src = OUT / f"icon_{size}.png"
        for name in names:
            (iconset / name).write_bytes(src.read_bytes())
    icns = OUT / "gallery.icns"
    try:
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(icns)], check=True)
    except (OSError, subprocess.CalledProcessError) as exc:
        print(f"iconutil skipped: {exc}")
    print(f"wrote icons in {OUT}")


if __name__ == "__main__":
    main()
