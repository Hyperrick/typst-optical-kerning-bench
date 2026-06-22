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

from PIL import Image, ImageDraw, ImageFont


@dataclass(frozen=True)
class Sample:
    index: int
    sample_id: str
    category: str
    text: str

    @property
    def slug(self) -> str:
        base = re.sub(r"[^a-z0-9]+", "-", self.sample_id.lower()).strip("-")
        return f"{self.index:02d}-{base or 'sample'}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render a guarded optical kerning review batch as PNG/HTML."
    )
    parser.add_argument(
        "--samples",
        default="corpus/samples/guarded-v1-review.json",
        help="JSON sample list.",
    )
    parser.add_argument("--font-id", default="eb-garamond")
    parser.add_argument("--font-family", default="EB Garamond")
    parser.add_argument("--point-size", default="100")
    parser.add_argument("--ligatures", choices=["true", "false"], default="false")
    parser.add_argument("--dpi", default="300")
    parser.add_argument(
        "--output",
        default="renders/guarded-v1-review/eb-garamond-100pt-no-ligatures",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=0,
        help="Optional smoke-test limit. 0 renders every sample.",
    )
    parser.add_argument(
        "--retries",
        type=int,
        default=2,
        help="Retries per sample for transient InDesign automation failures.",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def load_samples(path: Path, limit: int) -> list[Sample]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    samples = [
        Sample(
            index=i + 1,
            sample_id=str(item["id"]),
            category=str(item["category"]),
            text=str(item["text"]),
        )
        for i, item in enumerate(raw)
    ]
    return samples[:limit] if limit > 0 else samples


def render_sample(root: Path, args: argparse.Namespace, sample: Sample) -> dict:
    sample_dir = root / args.output / "samples" / sample.slug
    log_dir = root / args.output / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    command = [
        str(root / "scripts/run-goldfish-pipeline.sh"),
        "--font-id",
        args.font_id,
        "--font-family",
        args.font_family,
        "--text",
        sample.text,
        "--point-size",
        args.point_size,
        "--ligatures",
        args.ligatures,
        "--dpi",
        args.dpi,
        "--output",
        str(sample_dir.relative_to(root)),
    ]
    attempts = max(1, args.retries + 1)
    logs = []
    result = None
    for attempt in range(1, attempts + 1):
        if sample_dir.exists():
            shutil.rmtree(sample_dir)
        result = subprocess.run(command, cwd=root, text=True, capture_output=True)
        logs.append(
            f"== attempt {attempt}/{attempts} ==\n{result.stdout}{result.stderr}"
        )
        if result.returncode == 0:
            break
    assert result is not None
    (log_dir / f"{sample.slug}.log").write_text("\n".join(logs), encoding="utf-8")

    entry = {
        "id": sample.sample_id,
        "category": sample.category,
        "text": sample.text,
        "slug": sample.slug,
        "directory": str(sample_dir.relative_to(root)),
        "status": "ok" if result.returncode == 0 else "error",
        "returnCode": result.returncode,
    }
    comparison_path = sample_dir / "metrics/comparison.json"
    if result.returncode == 0 and comparison_path.exists():
        comparison = json.loads(comparison_path.read_text(encoding="utf-8"))
        entry["comparisons"] = comparison["comparisons"]
        entry["images"] = {
            "indesignOptical": str(
                (sample_dir / "crops/indesign-optical-ink.png").relative_to(root / args.output)
            ),
            "typstGuarded": str(
                (sample_dir / "crops/typst-guarded-ink.png").relative_to(root / args.output)
            ),
            "opticalOverlay": str(
                (sample_dir / "overlays/optical-vs-guarded.png").relative_to(root / args.output)
            ),
            "metricOverlay": str(
                (sample_dir / "overlays/metric-parity.png").relative_to(root / args.output)
            ),
        }
    return entry


def relative_link(path: str) -> str:
    return quote(path.replace("\\", "/"), safe="/.-_")


def write_summary(root: Path, args: argparse.Namespace, entries: list[dict]) -> None:
    out = root / args.output
    ok_entries = [entry for entry in entries if entry["status"] == "ok"]
    report = {
        "schemaVersion": 1,
        "fontId": args.font_id,
        "fontFamily": args.font_family,
        "pointSize": float(args.point_size),
        "ligatures": args.ligatures == "true",
        "dpi": int(args.dpi),
        "sampleCount": len(entries),
        "okCount": len(ok_entries),
        "errorCount": len(entries) - len(ok_entries),
        "samples": entries,
    }
    (out / "summary.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    write_html(out, report)
    write_contact_sheet(out, report)


def width_delta(entry: dict, key: str) -> str:
    comparisons = entry.get("comparisons", {})
    data = comparisons.get(key)
    if not data:
        return "-"
    px = data["widthDeltaPx"]
    em = data["widthDeltaEm"]
    return f"{px:+.0f}px / {em:+.4f}em"


def position_delta(entry: dict, key: str) -> str:
    comparisons = entry.get("comparisons", {})
    data = comparisons.get(key)
    if not data:
        return "-"
    ink_px = data.get("inkPositionMeanAbsPx")
    ink_em = data.get("inkPositionMeanAbsEm")
    seg_em = data.get("segmentCenterMeanAbsEm")
    if ink_px is None or ink_em is None:
        return "-"
    segment = "n/a" if seg_em is None else f"{seg_em:.4f}em"
    return f"ink {ink_px:.1f}px / {ink_em:.4f}em; segments {segment}"


def write_html(out: Path, report: dict) -> None:
    rows = []
    for entry in report["samples"]:
        if entry["status"] != "ok":
            rows.append(
                "<tr>"
                f"<td>{html.escape(entry['id'])}</td>"
                f"<td>{html.escape(entry['category'])}</td>"
                f"<td><code>{html.escape(entry['text'])}</code></td>"
                "<td colspan=\"6\">render failed; see log</td>"
                "</tr>"
            )
            continue
        images = entry["images"]
        rows.append(
            "<tr>"
            f"<td>{html.escape(entry['id'])}</td>"
            f"<td>{html.escape(entry['category'])}</td>"
            f"<td><code>{html.escape(entry['text'])}</code></td>"
            f"<td>{html.escape(width_delta(entry, 'metricParity'))}</td>"
            f"<td>{html.escape(width_delta(entry, 'opticalVsGuarded'))}</td>"
            f"<td>{html.escape(position_delta(entry, 'opticalVsGuarded'))}</td>"
            f"<td><img src=\"{relative_link(images['indesignOptical'])}\" alt=\"InDesign Optical\"></td>"
            f"<td><img src=\"{relative_link(images['typstGuarded'])}\" alt=\"Typst Guarded\"></td>"
            f"<td><img src=\"{relative_link(images['opticalOverlay'])}\" alt=\"Overlay\"></td>"
            "</tr>"
        )

    html_text = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Guarded Review</title>
  <style>
    :root {{
      color-scheme: light dark;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: Canvas;
      color: CanvasText;
    }}
    body {{
      margin: 24px;
    }}
    table {{
      border-collapse: collapse;
      width: 100%;
    }}
    th, td {{
      border-bottom: 1px solid color-mix(in srgb, CanvasText 18%, transparent);
      padding: 8px;
      text-align: left;
      vertical-align: middle;
      font-size: 13px;
    }}
    th {{
      position: sticky;
      top: 0;
      background: Canvas;
      z-index: 1;
    }}
    img {{
      max-width: 360px;
      max-height: 96px;
      background: white;
    }}
    code {{
      font-size: 14px;
    }}
  </style>
</head>
<body>
  <h1>Guarded Review</h1>
  <p>
    Font: {html.escape(report['fontFamily'])}, size: {report['pointSize']}pt,
    ligatures: {str(report['ligatures']).lower()}, samples: {report['okCount']}/{report['sampleCount']}.
  </p>
  <p>
    Overlay colors: cyan = InDesign Optical, magenta = Typst Guarded, black = overlap.
  </p>
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>Category</th>
        <th>Text</th>
        <th>Metric parity</th>
        <th>Optical vs Guarded</th>
        <th>Ink position</th>
        <th>InDesign Optical</th>
        <th>Typst Guarded</th>
        <th>Overlay</th>
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


def font() -> ImageFont.ImageFont:
    candidates = [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]
    for candidate in candidates:
        try:
            return ImageFont.truetype(candidate, 16)
        except OSError:
            pass
    return ImageFont.load_default()


def fit_image(image: Image.Image, width: int, height: int) -> Image.Image:
    scale = min(width / image.width, height / image.height, 1.0)
    size = (max(1, round(image.width * scale)), max(1, round(image.height * scale)))
    return image.resize(size, Image.Resampling.LANCZOS)


def paste_center(canvas: Image.Image, image_path: Path, box: tuple[int, int, int, int]) -> None:
    image = Image.open(image_path).convert("RGBA")
    fitted = fit_image(image, box[2] - box[0], box[3] - box[1])
    x = box[0] + ((box[2] - box[0]) - fitted.width) // 2
    y = box[1] + ((box[3] - box[1]) - fitted.height) // 2
    canvas.alpha_composite(fitted, (x, y))


def write_contact_sheet(out: Path, report: dict) -> None:
    entries = [entry for entry in report["samples"] if entry["status"] == "ok"]
    typeface = font()
    row_h = 160
    header_h = 58
    label_w = 280
    col_w = 420
    width = label_w + col_w * 3 + 64
    height = header_h + row_h * len(entries) + 32
    canvas = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    draw = ImageDraw.Draw(canvas)
    draw.text((24, 18), "Guarded Review", font=typeface, fill=(0, 0, 0, 255))
    draw.text((label_w + 24, 18), "InDesign Optical", font=typeface, fill=(0, 0, 0, 255))
    draw.text((label_w + col_w + 24, 18), "Typst Guarded", font=typeface, fill=(0, 0, 0, 255))
    draw.text((label_w + col_w * 2 + 24, 18), "Overlay", font=typeface, fill=(0, 0, 0, 255))

    for i, entry in enumerate(entries):
        y = header_h + i * row_h
        if i % 2 == 0:
            draw.rectangle((0, y, width, y + row_h), fill=(248, 248, 248, 255))
        draw.text((24, y + 22), entry["text"], font=typeface, fill=(0, 0, 0, 255))
        draw.text((24, y + 48), entry["category"], font=typeface, fill=(80, 80, 80, 255))
        draw.text(
            (24, y + 74),
            width_delta(entry, "opticalVsGuarded"),
            font=typeface,
            fill=(80, 80, 80, 255),
        )
        draw.text(
            (24, y + 100),
            position_delta(entry, "opticalVsGuarded"),
            font=typeface,
            fill=(80, 80, 80, 255),
        )
        images = entry["images"]
        paste_center(canvas, out / images["indesignOptical"], (label_w, y + 16, label_w + col_w - 20, y + row_h - 16))
        paste_center(canvas, out / images["typstGuarded"], (label_w + col_w, y + 16, label_w + col_w * 2 - 20, y + row_h - 16))
        paste_center(canvas, out / images["opticalOverlay"], (label_w + col_w * 2, y + 16, label_w + col_w * 3 - 20, y + row_h - 16))

    canvas.convert("RGB").save(out / "contact-sheet.png")


def main() -> int:
    args = parse_args()
    root = repo_root()
    samples = load_samples(root / args.samples, args.limit)
    output_root = root / args.output
    output_root.mkdir(parents=True, exist_ok=True)

    entries = []
    for sample in samples:
        print(
            f"[{sample.index:02d}/{len(samples):02d}] {sample.sample_id}: {sample.text}",
            flush=True,
        )
        entries.append(render_sample(root, args, sample))

    write_summary(root, args, entries)
    print(f"Output: {args.output}")
    print(f"HTML: {args.output}/index.html")
    print(f"Contact sheet: {args.output}/contact-sheet.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
