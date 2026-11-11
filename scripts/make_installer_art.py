"""Draw the installer's pictures (build-release.ps1 runs this; the outputs are build
files in target/installer, not committed):

- OnionLauncher.ico                the setup's icon, the same picture as the app's (build.rs)
- box-{dark,light}-{100..200}.png  the launcher box (docs/images/launcher-*.png) at each
                                   display scale, shown on the right of the setup's pages

    python scripts/make_installer_art.py
Needs Pillow (pip install pillow).
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target" / "installer"
ORANGE = (242, 140, 40)
WHITE = (255, 255, 255)
BOX = 220  # the launcher picture's size on the pages at 100 % scale, in pixels
SCALES = (100, 125, 150, 175, 200)
SS = 4  # drawn this many times bigger, then scaled down, for smooth edges


def icon(size: int) -> Image.Image:
    """build.rs's icon: an orange rounded square (corners 22 %) with 2 x 2 white tiles
    (margin 20 %, gap 8 %)."""
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((0, 0, s - 1, s - 1), radius=s * 0.22, fill=ORANGE)
    m, g = s * 0.2, s * 0.08
    a, b = (m, s / 2 - g / 2), (s / 2 + g / 2, s - m)
    for x0, x1 in (a, b):
        for y0, y1 in (a, b):
            d.rectangle((x0, y0, x1, y1), fill=WHITE)
    return img.resize((size, size), Image.LANCZOS)


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    icon(256).save(OUT / "OnionLauncher.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    # pre-sized, so Setup never stretches them: it picks the one for the screen's scale
    for theme in ("dark", "light"):
        box = Image.open(ROOT / "docs" / "images" / f"launcher-{theme}.png").convert("RGBA")
        for pct in SCALES:
            side = BOX * pct // 100
            box.resize((side, side), Image.LANCZOS).save(OUT / f"box-{theme}-{pct}.png")
    print(f"wrote the icon and pictures to {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
