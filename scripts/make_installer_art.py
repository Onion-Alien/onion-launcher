"""Draw the installer's pictures (build-release.ps1 runs this; the outputs are build
files in target/installer, not committed):

- box-light-{100..200}.png  the launcher box (docs/images/launcher-light.png) at each
                            display scale, shown on the right of the setup's pages

Setup has no icon of its own (installer/OnionLauncher.iss says why), so none is drawn here.

    python scripts/make_installer_art.py
Needs Pillow (pip install pillow).
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target" / "installer"
BOX = 220  # the launcher picture's size on the pages at 100 % scale, in pixels
SCALES = (100, 125, 150, 175, 200)


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    # pre-sized, so Setup never stretches them: it picks the one for the screen's scale
    box = Image.open(ROOT / "docs" / "images" / "launcher-light.png").convert("RGBA")
    for pct in SCALES:
        side = BOX * pct // 100
        box.resize((side, side), Image.LANCZOS).save(OUT / f"box-light-{pct}.png")
    print(f"wrote the pictures to {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
