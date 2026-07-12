#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


@dataclass(frozen=True)
class Example:
    font_id: str
    font_name: str
    sample_id: str
    sample: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build the small comparison figures used by GitHub Pages and the README."
    )
    parser.add_argument("--root", default=".", help="Repository root.")
    parser.add_argument(
        "--output",
        default="site/assets",
        help="Output directory relative to the repository root.",
    )
    return parser.parse_args()


def load_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf" if bold else "",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
    ]
    for candidate in candidates:
        if candidate and Path(candidate).exists():
            return ImageFont.truetype(candidate, size)
    return ImageFont.load_default()


TITLE = load_font(34, bold=True)
SUBTITLE = load_font(21)
HEADER = load_font(24, bold=True)
LABEL = load_font(21, bold=True)
BODY = load_font(18)

INK = (24, 24, 24)
MUTED = (92, 92, 92)
LINE = (205, 205, 205)
PANEL = (249, 249, 249)
ACCENT = (0, 112, 100)


def trim_white(image: Image.Image, pad: int = 12) -> Image.Image:
    rgb = image.convert("RGB")
    pixels = rgb.load()
    xs: list[int] = []
    ys: list[int] = []
    for y in range(rgb.height):
        for x in range(rgb.width):
            red, green, blue = pixels[x, y]
            if red < 245 or green < 245 or blue < 245:
                xs.append(x)
                ys.append(y)
    if not xs:
        return rgb
    return rgb.crop(
        (
            max(0, min(xs) - pad),
            max(0, min(ys) - pad),
            min(rgb.width, max(xs) + pad + 1),
            min(rgb.height, max(ys) + pad + 1),
        )
    )


def fit(image: Image.Image, width: int, height: int) -> Image.Image:
    scale = min(width / image.width, height / image.height, 1.0)
    size = (round(image.width * scale), round(image.height * scale))
    return image.resize(size, Image.Resampling.LANCZOS)


def paste_centered(canvas: Image.Image, image: Image.Image, box: tuple[int, int, int, int]) -> None:
    x0, y0, x1, y1 = box
    fitted = fit(trim_white(image), x1 - x0, y1 - y0)
    x = x0 + (x1 - x0 - fitted.width) // 2
    y = y0 + (y1 - y0 - fitted.height) // 2
    canvas.paste(fitted, (x, y))


def render_path(root: Path, example: Example, filename: str, ligatures: bool = False) -> Path:
    suite = "ligatures-100pt-v25" if ligatures else "no-ligatures-100pt-five-font-v25"
    return (
        root
        / "renders/optical-comparison-suite"
        / suite
        / example.font_id
        / example.sample_id
        / filename
    )


def build_main_comparison(root: Path, output: Path) -> None:
    example = Example("eb-garamond", "EB Garamond", "wavy", "WAVY")
    sources = [
        ("Typst Metric", "Current font-provided spacing", "crops/typst-metric-ink.png"),
        ("InDesign Optical", "External publishing reference", "crops/indesign-optical-ink.png"),
        ("Typst candidate", "Guarded outline-based correction", "crops/typst-guarded-ink.png"),
    ]
    margin = 34
    gap = 20
    panel_width = 560
    panel_height = 270
    heading_height = 108
    note_height = 54
    width = margin * 2 + panel_width * len(sources) + gap * (len(sources) - 1)
    height = heading_height + panel_height + note_height + margin
    canvas = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    draw.text((margin, 20), "The comparison in one example", font=TITLE, fill=INK)
    draw.text(
        (margin, 62),
        "Same static font, same 100 pt word, same shaping settings. Only spacing changes.",
        font=SUBTITLE,
        fill=MUTED,
    )

    x = margin
    for label, description, relative_path in sources:
        draw.rounded_rectangle(
            (x, heading_height, x + panel_width, heading_height + panel_height),
            radius=5,
            fill=PANEL,
            outline=LINE,
            width=1,
        )
        draw.text((x + 18, heading_height + 14), label, font=HEADER, fill=INK)
        draw.text((x + 18, heading_height + 48), description, font=BODY, fill=MUTED)
        image = Image.open(render_path(root, example, relative_path))
        paste_centered(
            canvas,
            image,
            (x + 18, heading_height + 84, x + panel_width - 18, heading_height + panel_height - 18),
        )
        x += panel_width + gap

    note_y = heading_height + panel_height + 22
    draw.text(
        (margin, note_y),
        "The candidate is evaluated against InDesign, not trained to copy it.",
        font=LABEL,
        fill=ACCENT,
    )
    draw.text(
        (margin + 690, note_y + 2),
        "Metric kerning remains the protected baseline.",
        font=BODY,
        fill=MUTED,
    )
    canvas.save(output / "main-comparison.png", optimize=True)


def build_cross_font_grid(root: Path, output: Path) -> None:
    examples = [
        Example("eb-garamond", "EB Garamond", "avatar", "AVATAR"),
        Example("libre-baskerville", "Libre Baskerville", "avatar", "AVATAR"),
        Example("inter", "Inter", "avatar", "AVATAR"),
        Example("eb-garamond", "EB Garamond", "10-000", "10.000"),
        Example("inter", "Inter", "10-000", "10.000"),
        Example("comic-neue", "Comic Neue", "10-000", "10.000"),
    ]
    columns = [
        ("Typst Metric", "crops/typst-metric-ink.png"),
        ("InDesign Optical", "crops/indesign-optical-ink.png"),
        ("Typst candidate", "crops/typst-guarded-ink.png"),
        ("Optical overlay", "overlays/optical-vs-guarded.png"),
    ]
    margin = 30
    row_label_width = 270
    column_width = 410
    header_height = 126
    row_height = 212
    width = margin * 2 + row_label_width + column_width * len(columns)
    height = header_height + row_height * len(examples) + margin
    canvas = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    draw.text((margin, 18), "Cross-font evidence", font=TITLE, fill=INK)
    draw.text(
        (margin, 60),
        "Cyan = InDesign Optical, magenta = Typst candidate, dark ink = agreement.",
        font=SUBTITLE,
        fill=MUTED,
    )

    x = margin + row_label_width
    for label, _ in columns:
        draw.text((x + 14, 94), label, font=LABEL, fill=INK)
        x += column_width

    y = header_height
    for example in examples:
        draw.line((margin, y, width - margin, y), fill=LINE, width=1)
        draw.text((margin + 10, y + 54), example.sample, font=HEADER, fill=INK)
        draw.text((margin + 10, y + 88), example.font_name, font=BODY, fill=MUTED)
        x = margin + row_label_width
        for _, relative_path in columns:
            image = Image.open(render_path(root, example, relative_path))
            paste_centered(canvas, image, (x + 14, y + 24, x + column_width - 14, y + row_height - 24))
            draw.line((x, y, x, y + row_height), fill=LINE, width=1)
            x += column_width
        y += row_height
    draw.line((margin, y, width - margin, y), fill=LINE, width=1)
    canvas.save(output / "cross-font-evidence.png", optimize=True)


def build_ligature_comparison(root: Path, output: Path) -> None:
    example = Example("libre-baskerville", "Libre Baskerville", "final", "final")
    sources = [
        ("InDesign Optical", "crops/indesign-optical-ink.png"),
        ("Typst candidate", "crops/typst-guarded-ink.png"),
        ("Overlay", "overlays/optical-vs-guarded.png"),
    ]
    margin = 30
    panel_width = 550
    panel_height = 220
    gap = 20
    heading_height = 112
    width = margin * 2 + panel_width * len(sources) + gap * (len(sources) - 1)
    height = heading_height + panel_height + margin
    canvas = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    draw.text((margin, 18), "Shaping comes before optical spacing", font=TITLE, fill=INK)
    draw.text(
        (margin, 60),
        "Libre Baskerville, ligatures enabled: the fi glyph is spaced as one shaped glyph.",
        font=SUBTITLE,
        fill=MUTED,
    )
    x = margin
    for label, relative_path in sources:
        draw.text((x, 90), label, font=LABEL, fill=INK)
        draw.rectangle(
            (x, heading_height, x + panel_width, heading_height + panel_height),
            fill=PANEL,
            outline=LINE,
        )
        image = Image.open(render_path(root, example, relative_path, ligatures=True))
        paste_centered(
            canvas,
            image,
            (x + 14, heading_height + 18, x + panel_width - 14, heading_height + panel_height - 18),
        )
        x += panel_width + gap
    canvas.save(output / "ligature-comparison.png", optimize=True)


def main() -> None:
    args = parse_args()
    root = Path(args.root).resolve()
    output = (root / args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    build_main_comparison(root, output)
    build_cross_font_grid(root, output)
    build_ligature_comparison(root, output)
    for path in sorted(output.glob("*.png")):
        image = Image.open(path)
        print(f"{path.relative_to(root)} {image.width}x{image.height} {path.stat().st_size} bytes")


if __name__ == "__main__":
    main()
