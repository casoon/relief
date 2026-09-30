#!/usr/bin/env python3
# Relief. MIT-Lizenz wie das Relief-Repository.
"""Platzhaltersymbol für Relief (Paket 111) in einen Chromium-Checkout.

    python3 fork/branding/make_icon.py <chromium-src>

Zeichnet ein „R“ auf abgerundetem Quadrat (PIL), ersetzt die PNGs in
chrome/app/theme/chromium/mac/Assets.xcassets, entfernt AppIcon.icon (Icon
Composer; sonst gewinnt auf neuem macOS das Chromium-Symbol) und erzeugt
app.icns (iconutil) und Assets.car (actool). Das Ergebnis gehört in den
Patch „Relief: Platzhaltersymbol“ (scripts/fork-export.sh); das endgültige
Symbol ist eine Gestaltungsfrage.
"""

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFont

BACKGROUND = (31, 95, 115)
FOREGROUND = (255, 255, 255)
FONT = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"


def draw(size: int) -> Image.Image:
    scale = 4  # überabtasten, dann verkleinern: glatte Kanten
    big = size * scale
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(image)
    # macOS-Raster: Fläche mit Rand (824 von 1024), Radius etwa 22 %.
    margin = big * 100 // 1024
    d.rounded_rectangle(
        (margin, margin, big - margin, big - margin),
        radius=big * 185 // 1024,
        fill=BACKGROUND,
    )
    font = ImageFont.truetype(FONT, big * 560 // 1024)
    d.text((big / 2, big / 2), "R", font=font, fill=FOREGROUND, anchor="mm")
    return image.resize((size, size), Image.LANCZOS)


def main() -> None:
    src = pathlib.Path(sys.argv[1])
    mac = src / "chrome/app/theme/chromium/mac"
    appiconset = mac / "Assets.xcassets/AppIcon.appiconset"
    contents = json.loads((appiconset / "Contents.json").read_text())
    for name in {i["filename"] for i in contents["images"] if "filename" in i}:
        size = int(name.split("_")[1].split(".")[0])
        draw(size).save(appiconset / name)
    iconset = mac / "Assets.xcassets/Icon.iconset"
    draw(256).save(iconset / "icon_256x256.png")
    draw(512).save(iconset / "icon_256x256@2x.png")
    shutil.rmtree(mac / "AppIcon.icon", ignore_errors=True)

    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        icns = tmp / "app.iconset"
        icns.mkdir()
        for size in (16, 32, 128, 256, 512):
            draw(size).save(icns / f"icon_{size}x{size}.png")
            draw(size * 2).save(icns / f"icon_{size}x{size}@2x.png")
        subprocess.run(
            ["iconutil", "-c", "icns", icns, "-o", mac / "app.icns"], check=True
        )
        out = tmp / "car"
        out.mkdir()
        subprocess.run(
            ["xcrun", "actool", "--compile", out, "--platform", "macosx",
             "--minimum-deployment-target", "13.0", "--app-icon", "AppIcon",
             "--output-partial-info-plist", tmp / "partial.plist",
             "--output-format", "human-readable-text",
             mac / "Assets.xcassets"],
            check=True, stdout=subprocess.DEVNULL,
        )
        shutil.copy(out / "Assets.car", mac / "Assets.car")
    print(f"Platzhaltersymbol nach {mac}")


if __name__ == "__main__":
    main()
