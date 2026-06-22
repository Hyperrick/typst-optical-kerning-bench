#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import quote

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore

from PIL import Image, ImageDraw, ImageFont


@dataclass(frozen=True)
class FontSpec:
    font_id: str
    family: str


@dataclass(frozen=True)
class GlyphSample:
    index: int
    text: str

    @property
    def slug(self) -> str:
        codepoints = "-".join(f"u{ord(char):04x}" for char in self.text)
        readable = re.sub(r"[^a-z0-9]+", "-", self.text.lower()).strip("-")
        label = readable or codepoints
        return f"{self.index:02d}-{label}-{codepoints}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render individual glyphs in InDesign and Typst for shape parity."
    )
    parser.add_argument("--fonts", default="eb-garamond,libre-baskerville,inter")
    parser.add_argument("--glyphs", default="G,o,l,d,f,i,s,h")
    parser.add_argument("--point-size", default="100")
    parser.add_argument("--dpi", default="300")
    parser.add_argument(
        "--output",
        default="renders/glyph-shape-parity/goldfish-glyphs-100pt-no-ligatures",
    )
    parser.add_argument(
        "--baseline-output",
        default="",
        help="Optional compact JSON snapshot path.",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def load_font_specs(root: Path, requested: list[str]) -> list[FontSpec]:
    manifest = tomllib.loads((root / "corpus/fonts.toml").read_text(encoding="utf-8"))
    by_id = {
        str(item["id"]): FontSpec(str(item["id"]), str(item["family"]))
        for item in manifest["fonts"]
    }
    missing = [font_id for font_id in requested if font_id not in by_id]
    if missing:
        raise SystemExit(f"Unknown font id(s): {', '.join(missing)}")
    return [by_id[font_id] for font_id in requested]


def parse_glyphs(raw: str) -> list[GlyphSample]:
    values = [value for value in raw.split(",") if value != ""]
    if not values:
        raise SystemExit("--glyphs must contain at least one glyph")
    return [GlyphSample(i + 1, value) for i, value in enumerate(values)]


def typ_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def typ_content(value: str) -> str:
    return (
        value
        .replace("\\", "\\\\")
        .replace("#", "\\#")
        .replace("[", "\\[")
        .replace("]", "\\]")
    )


def dark(pixel: tuple[int, int, int, int]) -> bool:
    red, green, blue, alpha = pixel
    return alpha > 0 and red + green + blue < 660


def crop_ink(src: Path, out: Path) -> dict:
    img = Image.open(src).convert("RGBA")
    min_x, min_y = img.width, img.height
    max_x = max_y = -1
    for y in range(img.height):
        for x in range(img.width):
            if dark(img.getpixel((x, y))):
                min_x = min(min_x, x)
                min_y = min(min_y, y)
                max_x = max(max_x, x)
                max_y = max(max_y, y)
    if max_x < 0:
        raise RuntimeError(f"no ink found in {src}")
    crop = img.crop((min_x, min_y, max_x + 1, max_y + 1))
    out.parent.mkdir(parents=True, exist_ok=True)
    crop.save(out)
    return {
        "x0": min_x,
        "y0": min_y,
        "x1": max_x,
        "y1": max_y,
        "width": crop.width,
        "height": crop.height,
    }


def overlay(ref_path: Path, cand_path: Path, out: Path, normalize_height: bool) -> dict:
    ref = Image.open(ref_path).convert("RGBA")
    cand = Image.open(cand_path).convert("RGBA")
    scale = ref.height / cand.height if normalize_height and cand.height else 1.0
    if scale != 1.0:
        cand = cand.resize(
            (max(1, round(cand.width * scale)), ref.height),
            Image.Resampling.LANCZOS,
        )
    canvas = Image.new(
        "RGBA",
        (max(ref.width, cand.width), max(ref.height, cand.height)),
        (255, 255, 255, 255),
    )
    pixels = canvas.load()
    overlap = 0
    ref_ink = 0
    cand_ink = 0
    for y in range(ref.height):
        for x in range(ref.width):
            if dark(ref.getpixel((x, y))):
                ref_ink += 1
                pixels[x, y] = (0, 170, 220, 255)
    for y in range(cand.height):
        for x in range(cand.width):
            if dark(cand.getpixel((x, y))):
                cand_ink += 1
                if pixels[x, y][:3] == (0, 170, 220):
                    overlap += 1
                    pixels[x, y] = (10, 10, 10, 255)
                else:
                    pixels[x, y] = (220, 0, 170, 255)
    out.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(out)
    union = ref_ink + cand_ink - overlap
    return {
        "path": str(out),
        "normalizeHeight": normalize_height,
        "referencePx": [ref.width, ref.height],
        "candidatePx": [cand.width, cand.height],
        "heightScale": scale,
        "widthDeltaPx": ref.width - cand.width,
        "heightDeltaPx": ref.height - cand.height,
        "overlapRatio": 1.0 if union == 0 else overlap / union,
    }


def render_typst(root: Path, font: FontSpec, sample: GlyphSample, args: argparse.Namespace, out: Path) -> Path:
    typ_path = out / "typst/glyph.typ"
    png_path = out / "typst/glyph.png"
    typ_path.parent.mkdir(parents=True, exist_ok=True)
    margin = float(args.point_size) * 0.8
    typ_path.write_text(
        f"""#set page(width: auto, height: auto, margin: {margin:.4f}pt)
#set text(
  font: "{typ_string(font.family)}",
  size: {float(args.point_size):.4f}pt,
  kerning: false,
  ligatures: false,
  features: (liga: 0, clig: 0),
)
{typ_content(sample.text)}
""",
        encoding="utf-8",
    )
    subprocess.run(
        [
            "typst",
            "compile",
            "--font-path",
            "corpus/fonts",
            "--ignore-system-fonts",
            "--format",
            "png",
            "--ppi",
            args.dpi,
            str(typ_path.relative_to(root)),
            str(png_path.relative_to(root)),
        ],
        cwd=root,
        check=True,
    )
    return png_path


def render_indesign(root: Path, font: FontSpec, sample: GlyphSample, args: argparse.Namespace, out: Path) -> tuple[Path, dict]:
    font_path = root / f"corpus/fonts/{font.font_id}.ttf"
    document_fonts = out / "indesign/Document fonts"
    document_fonts.mkdir(parents=True, exist_ok=True)
    shutil.copy2(font_path, document_fonts / f"{font.font_id}.ttf")

    pdf_path = out / "indesign/glyph.tmp.pdf"
    indd_path = out / "indesign/glyph.indd"
    json_path = out / "metrics/indesign.json"
    subprocess.run(
        [
            str(root / "scripts/render-indesign-outlined-text.sh"),
            "--font-family",
            font.family,
            "--text",
            sample.text,
            "--kerning",
            "none",
            "--ligatures",
            "false",
            "--point-size",
            args.point_size,
            "--output-pdf",
            str(pdf_path),
            "--output-indd",
            str(indd_path),
            "--output-json",
            str(json_path),
        ],
        cwd=root,
        check=True,
    )
    subprocess.run(
        ["pdftoppm", "-png", "-r", args.dpi, str(pdf_path), str(out / "indesign/glyph")],
        cwd=root,
        check=True,
    )
    pdf_path.unlink(missing_ok=True)
    for lock in (out / "indesign").glob("*.idlk"):
        lock.unlink(missing_ok=True)
    sidecar = json.loads(json_path.read_text(encoding="utf-8"))
    return out / "indesign/glyph-1.png", sidecar


def render_sample(root: Path, font: FontSpec, sample: GlyphSample, args: argparse.Namespace, base: Path) -> dict:
    sample_out = base / font.font_id / sample.slug
    if sample_out.exists():
        shutil.rmtree(sample_out)
    sample_out.mkdir(parents=True, exist_ok=True)
    entry: dict = {
        "fontId": font.font_id,
        "fontFamily": font.family,
        "glyph": sample.text,
        "slug": sample.slug,
        "directory": str(sample_out.relative_to(root)),
        "status": "ok",
    }
    try:
        id_png, id_sidecar = render_indesign(root, font, sample, args, sample_out)
        typst_png = render_typst(root, font, sample, args, sample_out)
        id_crop = sample_out / "crops/indesign-ink.png"
        typst_crop = sample_out / "crops/typst-ink.png"
        id_box = crop_ink(id_png, id_crop)
        typst_box = crop_ink(typst_png, typst_crop)
        raw = overlay(id_crop, typst_crop, sample_out / "overlays/raw-shape-parity.png", False)
        normalized = overlay(id_crop, typst_crop, sample_out / "overlays/height-normalized-shape-parity.png", True)
        entry.update(
            {
                "indesign": {
                    "sidecar": id_sidecar,
                    "crop": id_box,
                },
                "typst": {
                    "crop": typst_box,
                },
                "comparisons": {
                    "rawShapeParity": raw,
                    "heightNormalizedShapeParity": normalized,
                },
                "images": {
                    "indesign": relative_to_base(id_crop, base),
                    "typst": relative_to_base(typst_crop, base),
                    "rawOverlay": relative_to_base(sample_out / "overlays/raw-shape-parity.png", base),
                    "normalizedOverlay": relative_to_base(sample_out / "overlays/height-normalized-shape-parity.png", base),
                },
            }
        )
    except Exception as error:  # noqa: BLE001 - persisted in report for debugging.
        entry["status"] = "error"
        entry["error"] = str(error)
    return entry


def relative_to_base(path: Path, base: Path) -> str:
    return str(path.relative_to(base)).replace("\\", "/")


def write_reports(root: Path, args: argparse.Namespace, entries: list[dict]) -> dict:
    out = root / args.output
    report = {
        "schemaVersion": 1,
        "purpose": "Individual glyph shape parity before word and kerning comparison.",
        "pointSize": float(args.point_size),
        "dpi": int(args.dpi),
        "kerning": "none",
        "ligatures": False,
        "sampleCount": len(entries),
        "okCount": sum(1 for entry in entries if entry["status"] == "ok"),
        "errorCount": sum(1 for entry in entries if entry["status"] != "ok"),
        "samples": entries,
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "summary.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    write_html(out, report)
    write_contact_sheet(out, report)
    if args.baseline_output:
        write_baseline_snapshot(root / args.baseline_output, report)
    return report


def write_baseline_snapshot(path: Path, report: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    snapshot = {
        "schemaVersion": 1,
        "pointSize": report["pointSize"],
        "dpi": report["dpi"],
        "kerning": report["kerning"],
        "ligatures": report["ligatures"],
        "samples": [
            {
                "fontId": entry["fontId"],
                "fontFamily": entry["fontFamily"],
                "glyph": entry["glyph"],
                "status": entry["status"],
                "appliedFont": entry.get("indesign", {})
                .get("sidecar", {})
                .get("appliedFont"),
                "rawShapeParity": compact_comparison(entry, "rawShapeParity"),
                "heightNormalizedShapeParity": compact_comparison(entry, "heightNormalizedShapeParity"),
            }
            for entry in report["samples"]
        ],
    }
    path.write_text(json.dumps(snapshot, indent=2), encoding="utf-8")


def compact_comparison(entry: dict, key: str) -> dict | None:
    data = entry.get("comparisons", {}).get(key)
    if not data:
        return None
    return {
        "referencePx": data["referencePx"],
        "candidatePx": data["candidatePx"],
        "heightScale": data["heightScale"],
        "widthDeltaPx": data["widthDeltaPx"],
        "heightDeltaPx": data["heightDeltaPx"],
        "overlapRatio": data["overlapRatio"],
    }


def fmt_shape(entry: dict, key: str) -> str:
    data = entry.get("comparisons", {}).get(key)
    if not data:
        return "-"
    return f"{data['widthDeltaPx']:+.0f}px w, {data['heightDeltaPx']:+.0f}px h, overlap {data['overlapRatio']:.3f}"


def link(path: str) -> str:
    return quote(path, safe="/.-_")


def write_html(out: Path, report: dict) -> None:
    rows = []
    for entry in report["samples"]:
        if entry["status"] != "ok":
            rows.append(
                "<tr>"
                f"<td>{html.escape(entry['fontFamily'])}</td>"
                f"<td><code>{html.escape(entry['glyph'])}</code></td>"
                f"<td colspan=\"6\">{html.escape(entry.get('error', 'render failed'))}</td>"
                "</tr>"
            )
            continue
        images = entry["images"]
        applied = entry["indesign"]["sidecar"].get("appliedFont", {})
        applied_label = applied.get("name") or applied.get("postscriptName") or "-"
        rows.append(
            "<tr>"
            f"<td><strong>{html.escape(entry['fontFamily'])}</strong><br>"
            f"<small>{html.escape(entry['fontId'])}</small></td>"
            f"<td><code>{html.escape(entry['glyph'])}</code></td>"
            f"<td><small>{html.escape(applied_label)}</small></td>"
            f"<td>{html.escape(fmt_shape(entry, 'rawShapeParity'))}</td>"
            f"<td>{html.escape(fmt_shape(entry, 'heightNormalizedShapeParity'))}</td>"
            f"<td><img src=\"{link(images['indesign'])}\" alt=\"InDesign\"></td>"
            f"<td><img src=\"{link(images['typst'])}\" alt=\"Typst\"></td>"
            f"<td><img src=\"{link(images['rawOverlay'])}\" alt=\"Raw overlay\"></td>"
            f"<td><img src=\"{link(images['normalizedOverlay'])}\" alt=\"Normalized overlay\"></td>"
            "</tr>"
        )

    html_text = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Glyph Shape Parity</title>
  <style>
    :root {{
      color-scheme: light dark;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: Canvas;
      color: CanvasText;
    }}
    body {{ margin: 24px; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{
      border-bottom: 1px solid color-mix(in srgb, CanvasText 18%, transparent);
      padding: 8px;
      text-align: left;
      vertical-align: middle;
      font-size: 13px;
    }}
    th {{ position: sticky; top: 0; background: Canvas; z-index: 1; }}
    img {{
      max-width: 180px;
      max-height: 110px;
      background: white;
    }}
    code {{ font-size: 15px; }}
    small {{ color: color-mix(in srgb, CanvasText 64%, transparent); }}
  </style>
</head>
<body>
  <h1>Glyph Shape Parity</h1>
  <p>
    Kerning: none, ligatures: false, size: {report['pointSize']}pt,
    DPI: {report['dpi']}. Raw overlay is intentionally not scaled.
    Overlay colors: cyan = InDesign, magenta = Typst, black = overlap.
  </p>
  <table>
    <thead>
      <tr>
        <th>Font</th>
        <th>Glyph</th>
        <th>InDesign applied font</th>
        <th>Raw shape</th>
        <th>Normalized shape</th>
        <th>InDesign</th>
        <th>Typst</th>
        <th>Raw overlay</th>
        <th>Height-normalized overlay</th>
      </tr>
    </thead>
    <tbody>
      {''.join(rows)}
    </tbody>
  </table>
</body>
</html>
"""
    (out / "index.html").write_text(html_text, encoding="utf-8")


def write_contact_sheet(out: Path, report: dict) -> None:
    rows = [entry for entry in report["samples"] if entry["status"] == "ok"]
    if not rows:
        return

    label_w = 300
    col_w = 230
    row_h = 138
    header_h = 72
    columns = ["InDesign", "Typst", "Raw overlay", "Normalized"]
    width = label_w + col_w * len(columns)
    height = header_h + row_h * len(rows)
    canvas = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    try:
        title_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 17)
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 12)
        small_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 10)
    except OSError:
        title_font = font = small_font = ImageFont.load_default()

    draw.text((16, 14), "Glyph Shape Parity", fill=(0, 0, 0), font=title_font)
    draw.text(
        (16, 42),
        "kerning none; ligatures false; cyan = InDesign, magenta = Typst, black = overlap",
        fill=(70, 70, 70),
        font=small_font,
    )
    for i, column in enumerate(columns):
        draw.text((label_w + i * col_w + 12, 20), column, fill=(0, 0, 0), font=font)

    for row_index, entry in enumerate(rows):
        y = header_h + row_index * row_h
        draw.rectangle((0, y, width, y + row_h), outline=(220, 220, 220))
        draw.text(
            (16, y + 14),
            f"{entry['fontFamily']} / {entry['glyph']}",
            fill=(0, 0, 0),
            font=title_font,
        )
        draw.text((16, y + 38), fmt_shape(entry, "rawShapeParity"), fill=(35, 35, 35), font=font)
        draw.text(
            (16, y + 58),
            fmt_shape(entry, "heightNormalizedShapeParity"),
            fill=(70, 70, 70),
            font=small_font,
        )
        images = entry["images"]
        image_paths = [
            out / images["indesign"],
            out / images["typst"],
            out / images["rawOverlay"],
            out / images["normalizedOverlay"],
        ]
        for col_index, image_path in enumerate(image_paths):
            box = (
                label_w + col_index * col_w + 14,
                y + 16,
                label_w + (col_index + 1) * col_w - 14,
                y + row_h - 14,
            )
            paste_center(canvas, image_path, box)

    canvas.save(out / "contact-sheet.png")


def paste_center(canvas: Image.Image, image_path: Path, box: tuple[int, int, int, int]) -> None:
    img = Image.open(image_path).convert("RGB")
    max_w = box[2] - box[0]
    max_h = box[3] - box[1]
    scale = min(max_w / img.width, max_h / img.height, 1.0)
    size = (max(1, round(img.width * scale)), max(1, round(img.height * scale)))
    resized = img.resize(size, Image.Resampling.LANCZOS)
    x = box[0] + (max_w - resized.width) // 2
    y = box[1] + (max_h - resized.height) // 2
    canvas.paste(resized, (x, y))


def main() -> None:
    args = parse_args()
    root = repo_root()
    font_ids = [font_id.strip() for font_id in args.fonts.split(",") if font_id.strip()]
    fonts = load_font_specs(root, font_ids)
    glyphs = parse_glyphs(args.glyphs)
    out = root / args.output
    out.mkdir(parents=True, exist_ok=True)

    entries = []
    for font in fonts:
        for sample in glyphs:
            print(f"== {font.font_id} {sample.text} ==")
            entries.append(render_sample(root, font, sample, args, out))

    report = write_reports(root, args, entries)
    print(f"Summary: {args.output}/summary.json")
    print(f"HTML: {args.output}/index.html")
    print(f"Contact sheet: {args.output}/contact-sheet.png")
    print(f"OK: {report['okCount']}/{report['sampleCount']}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
