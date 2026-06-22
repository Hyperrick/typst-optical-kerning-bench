#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build small committed paper figures from generated V24 renders."
    )
    parser.add_argument(
        "--output",
        default="docs/figures",
        help="Directory for generated PNG figures. Default: docs/figures.",
    )
    parser.add_argument(
        "--root",
        default=".",
        help="Repository root. Default: current directory.",
    )
    return parser.parse_args()


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    names = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf" if bold else "",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
    ]
    for name in names:
        if not name:
            continue
        path = Path(name)
        if path.exists():
            return ImageFont.truetype(str(path), size)
    return ImageFont.load_default()


TITLE = font(28, bold=True)
LABEL = font(26, bold=True)
SMALL = font(18)


def trim_white(image: Image.Image, pad: int = 16) -> Image.Image:
    rgba = image.convert("RGBA")
    pixels = rgba.load()
    width, height = rgba.size
    xs: list[int] = []
    ys: list[int] = []
    for y in range(height):
        for x in range(width):
            red, green, blue, alpha = pixels[x, y]
            if alpha and (red < 245 or green < 245 or blue < 245):
                xs.append(x)
                ys.append(y)
    if not xs:
        return rgba
    box = (
        max(0, min(xs) - pad),
        max(0, min(ys) - pad),
        min(width, max(xs) + pad + 1),
        min(height, max(ys) + pad + 1),
    )
    return rgba.crop(box)


def fit(image: Image.Image, max_width: int, max_height: int) -> Image.Image:
    rgba = image.convert("RGBA")
    scale = min(max_width / rgba.width, max_height / rgba.height, 1.0)
    size = (round(rgba.width * scale), round(rgba.height * scale))
    return rgba.resize(size, Image.Resampling.LANCZOS)


def panel(label: str, path: Path, max_width: int = 560, max_height: int = 260) -> Image.Image:
    image = fit(trim_white(Image.open(path)), max_width, max_height)
    height = image.height + 58
    canvas = Image.new("RGBA", (max_width, height), "white")
    draw = ImageDraw.Draw(canvas)
    draw.text((0, 0), label, fill=(20, 20, 20), font=LABEL)
    x = (max_width - image.width) // 2
    canvas.alpha_composite(image, (x, 46))
    draw.rectangle((0, 42, max_width - 1, height - 1), outline=(210, 210, 210), width=1)
    return canvas


def compose_triptych(
    root: Path,
    output: Path,
    title: str,
    subtitle: str,
    paths: list[str],
    labels: list[str],
    filename: str,
) -> None:
    panels = [panel(label, root / path) for label, path in zip(labels, paths)]
    gap = 22
    margin = 30
    title_height = 76
    width = margin * 2 + sum(item.width for item in panels) + gap * (len(panels) - 1)
    height = title_height + max(item.height for item in panels) + margin
    canvas = Image.new("RGBA", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    draw.text((margin, 18), title, fill=(10, 10, 10), font=TITLE)
    draw.text((margin, 50), subtitle, fill=(90, 90, 90), font=SMALL)
    x = margin
    for item in panels:
        canvas.alpha_composite(item, (x, title_height))
        x += item.width + gap
    canvas.convert("RGB").save(output / filename, optimize=True)


def crop_contact(
    root: Path,
    output: Path,
    source: str,
    filename: str,
    title: str,
    subtitle: str,
    y0: int,
    y1: int,
    max_width: int = 1200,
) -> None:
    image = Image.open(root / source).convert("RGB")
    crop = image.crop((0, y0, image.width, min(image.height, y1)))
    scale = min(max_width / crop.width, 1.0)
    crop = crop.resize((round(crop.width * scale), round(crop.height * scale)), Image.Resampling.LANCZOS)
    margin = 24
    title_height = 72
    canvas = Image.new("RGB", (crop.width + margin * 2, crop.height + title_height + margin), "white")
    draw = ImageDraw.Draw(canvas)
    draw.text((margin, 18), title, font=TITLE, fill=(10, 10, 10))
    draw.text((margin, 50), subtitle, font=SMALL, fill=(90, 90, 90))
    canvas.paste(crop, (margin, title_height))
    canvas.save(output / filename, optimize=True)


def main() -> None:
    args = parse_args()
    root = Path(args.root).resolve()
    output = (root / args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)

    compose_triptych(
        root,
        output,
        "Short serif ligature word: V24 target case",
        "Libre Baskerville, ligatures enabled, text: final",
        [
            "renders/optical-comparison-suite/ligatures-100pt-v24/libre-baskerville/final/crops/indesign-optical-ink.png",
            "renders/optical-comparison-suite/ligatures-100pt-v24/libre-baskerville/final/crops/typst-guarded-ink.png",
            "renders/optical-comparison-suite/ligatures-100pt-v24/libre-baskerville/final/overlays/optical-vs-guarded.png",
        ],
        ["InDesign Optical", "Typst Guarded Optical", "Overlay"],
        "v24-libre-final-ligature.png",
    )
    compose_triptych(
        root,
        output,
        "No-ligature control: unchanged by V24",
        "EB Garamond, ligatures disabled, text: ToTaL",
        [
            "renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24/eb-garamond/total/crops/indesign-optical-ink.png",
            "renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24/eb-garamond/total/crops/typst-guarded-ink.png",
            "renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24/eb-garamond/total/overlays/optical-vs-guarded.png",
        ],
        ["InDesign Optical", "Typst Guarded Optical", "Overlay"],
        "v24-eb-total-no-ligature-control.png",
    )
    crop_contact(
        root,
        output,
        "renders/optical-comparison-suite/ligatures-100pt-v24/contact-sheet.png",
        "v24-ligature-sheet-excerpt.png",
        "Ligature-capable suite excerpt",
        "Columns: InDesign Optical, Typst Guarded Optical, overlay",
        0,
        760,
    )
    crop_contact(
        root,
        output,
        "renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24/contact-sheet.png",
        "v24-no-ligature-sheet-excerpt.png",
        "No-ligature suite excerpt",
        "Columns: InDesign Optical, Typst Guarded Optical, overlay",
        0,
        760,
    )

    for path in sorted(output.glob("*.png")):
        print(f"{path.relative_to(root)} {Image.open(path).size} {path.stat().st_size} bytes")


if __name__ == "__main__":
    main()
