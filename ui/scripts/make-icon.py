"""Draws the TVPN Pro app icon (1024x1024 PNG) for the Tauri icon generator.

    python ui/scripts/make-icon.py ui/static/app-icon.png

Then: bunx tauri icon ui/static/app-icon.png
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw

SIZE = 1024
SCALE = 2  # supersampling


def gradient(size: int, start: tuple[int, int, int], end: tuple[int, int, int]) -> Image.Image:
    image = Image.new("RGB", (size, size))
    pixels = image.load()
    for y in range(size):
        t = y / (size - 1)
        color = tuple(round(start[i] + (end[i] - start[i]) * t) for i in range(3))
        for x in range(size):
            pixels[x, y] = color
    return image


def shield_points(size: int) -> list[tuple[float, float]]:
    s = size / 1024
    points = [
        (512, 248),
        (700, 318),
        (700, 512),
        (692, 592),
        (668, 660),
        (630, 716),
        (580, 758),
        (512, 786),
        (444, 758),
        (394, 716),
        (356, 660),
        (332, 592),
        (324, 512),
        (324, 318),
    ]
    return [(x * s, y * s) for x, y in points]


def main() -> int:
    target = Path(sys.argv[1] if len(sys.argv) > 1 else "ui/static/app-icon.png")
    size = SIZE * SCALE

    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, size - 1, size - 1), radius=round(220 * SCALE), fill=255
    )

    base = gradient(size, (10, 132, 255), (124, 92, 255)).convert("RGBA")
    base.putalpha(mask)

    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)

    points = shield_points(size)
    draw.line(points + [points[0]], fill=(255, 255, 255, 255), width=round(34 * SCALE), joint="curve")
    # Close the sharp top vertex: line joins leave a notch there.
    for x, y in points:
        r = round(17 * SCALE)
        draw.ellipse((x - r, y - r, x + r, y + r), fill=(255, 255, 255, 255))

    check = [(430 * SCALE, 512 * SCALE), (492 * SCALE, 576 * SCALE), (606 * SCALE, 438 * SCALE)]
    draw.line(check, fill=(255, 255, 255, 255), width=round(44 * SCALE), joint="curve")
    for x, y in check:
        r = round(22 * SCALE)
        draw.ellipse((x - r, y - r, x + r, y + r), fill=(255, 255, 255, 255))

    composed = Image.alpha_composite(base, layer).resize((SIZE, SIZE), Image.LANCZOS)
    target.parent.mkdir(parents=True, exist_ok=True)
    composed.save(target)
    print(f"wrote {target} ({SIZE}x{SIZE})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
