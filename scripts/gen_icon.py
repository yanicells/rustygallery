#!/usr/bin/env python3
"""Draw the gallery mark at several sizes. No third-party deps."""

from __future__ import annotations

import math
import pathlib
import struct
import subprocess
import sys
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "assets" / "icon"

RUST = (196, 92, 56)
CREAM = (247, 239, 228)


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


def coverage(distance: float) -> float:
    """One-pixel antialiasing around a shape's signed distance boundary."""
    return max(0.0, min(1.0, 0.5 - distance))


def paint(size: int, template: bool) -> list[tuple[int, int, int, int]]:
    half = size / 2
    outer = size * 0.32
    inner = size * 0.17
    radius = size * 0.22
    out = []
    for y in range(size):
        for x in range(size):
            px = x + 0.5 - half
            py = y + 0.5 - half
            distance = math.hypot(px, py)
            ring = coverage(distance - outer) * (1 - coverage(distance - inner))
            if template:
                out.append((0, 0, 0, round(255 * ring)))
                continue
            qx = abs(px) - (half - radius)
            qy = abs(py) - (half - radius)
            tile = coverage(math.hypot(max(qx, 0), max(qy, 0)) + min(max(qx, qy), 0) - radius)
            color = tuple(round(rust + (cream - rust) * ring) for rust, cream in zip(RUST, CREAM))
            out.append((*color, round(255 * tile)))
    return out


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    sizes = (16, 32, 64, 128, 256, 512, 1024)
    for size in sizes:
        png(OUT / f"icon_{size}.png", paint(size, False), size)
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
    if sys.platform == "darwin":
        # A failed conversion must not leave an older ICNS looking successful.
        icns.unlink(missing_ok=True)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(icns)], check=True)
    else:
        print("ICNS generation requires macOS iconutil; wrote PNGs only")
    print(f"wrote icons in {OUT}")


if __name__ == "__main__":
    main()
