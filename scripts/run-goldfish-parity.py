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
    font_path: str | None = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run the single-word Goldfish parity gate for selected fonts. "
            "Metric parity is checked before optical results are treated as "
            "algorithm evidence."
        )
    )
    parser.add_argument(
        "--fonts",
        default="eb-garamond,libre-baskerville,inter",
        help="Comma-separated font ids from corpus/fonts.toml.",
    )
    parser.add_argument(
        "--font-specs",
        default="",
        help="Optional JSON list with fontId, family, and optional fontPath.",
    )
    parser.add_argument("--text", default="Goldfish")
    parser.add_argument("--point-size", default="100")
    parser.add_argument("--ligatures", choices=["true", "false"], default="false")
    parser.add_argument("--dpi", default="300")
    parser.add_argument(
        "--metric-threshold-em",
        type=float,
        default=0.02,
        help="Maximum absolute metric width delta before a font is gated out.",
    )
    parser.add_argument(
        "--output",
        default="renders/goldfish-parity/goldfish-100pt-no-ligatures",
    )
    parser.add_argument(
        "--baseline-output",
        default="",
        help="Optional compact JSON snapshot path, e.g. baselines/goldfish-parity-v1.json.",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def load_font_specs(root: Path, args: argparse.Namespace) -> list[FontSpec]:
    if args.font_specs:
        raw = json.loads((root / args.font_specs).read_text(encoding="utf-8"))
        return [
            FontSpec(
                font_id=str(item["fontId"]),
                family=str(item["family"]),
                font_path=str(item["fontPath"]) if item.get("fontPath") else None,
            )
            for item in raw
        ]

    requested = [font_id.strip() for font_id in args.fonts.split(",") if font_id.strip()]
    manifest = tomllib.loads((root / "corpus/fonts.toml").read_text(encoding="utf-8"))
    by_id = {
        str(item["id"]): FontSpec(str(item["id"]), str(item["family"]))
        for item in manifest["fonts"]
    }
    missing = [font_id for font_id in requested if font_id not in by_id]
    if missing:
        raise SystemExit(f"Unknown font id(s): {', '.join(missing)}")
    return [by_id[font_id] for font_id in requested]


def slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-") or "sample"


def run_pipeline(root: Path, args: argparse.Namespace, font: FontSpec, out: Path) -> dict:
    font_out = out / font.font_id
    if font_out.exists():
        shutil.rmtree(font_out)
    command = [
        str(root / "scripts/run-goldfish-pipeline.sh"),
        "--font-id",
        font.font_id,
    ]
    if font.font_path:
        command.extend(["--font-path", font.font_path])
    command.extend([
        "--font-family",
        font.family,
        "--text",
        args.text,
        "--point-size",
        args.point_size,
        "--ligatures",
        args.ligatures,
        "--dpi",
        args.dpi,
        "--output",
        str(font_out.relative_to(root)),
    ])
    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    log_dir = out / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / f"{font.font_id}.log"
    log_path.write_text(result.stdout + result.stderr, encoding="utf-8")

    entry: dict = {
        "fontId": font.font_id,
        "fontFamily": font.family,
        "fontPath": font.font_path,
        "text": args.text,
        "pointSize": float(args.point_size),
        "ligatures": args.ligatures == "true",
        "dpi": int(args.dpi),
        "directory": str(font_out.relative_to(root)),
        "status": "ok" if result.returncode == 0 else "error",
        "returnCode": result.returncode,
        "log": str(log_path.relative_to(root)),
    }

    comparison_path = font_out / "metrics/comparison.json"
    if result.returncode == 0 and comparison_path.exists():
        comparison = json.loads(comparison_path.read_text(encoding="utf-8"))
        entry["comparisons"] = comparison["comparisons"]
        entry["images"] = {
            "noneOverlay": relative_to_out(font_out / "overlays/none-parity.png", out),
            "indesignMetric": relative_to_out(font_out / "crops/indesign-metric-ink.png", out),
            "typstMetric": relative_to_out(font_out / "crops/typst-metric-ink.png", out),
            "metricOverlay": relative_to_out(font_out / "overlays/metric-parity.png", out),
            "indesignOptical": relative_to_out(font_out / "crops/indesign-optical-ink.png", out),
            "typstGuarded": relative_to_out(font_out / "crops/typst-guarded-ink.png", out),
            "opticalOverlay": relative_to_out(font_out / "overlays/optical-vs-guarded.png", out),
        }
        apply_metric_gate(entry, args.metric_threshold_em)
    else:
        entry["metricGate"] = {
            "thresholdEm": args.metric_threshold_em,
            "status": "render-error",
            "validForOpticalTuning": False,
        }
    return entry


def relative_to_out(path: Path, out: Path) -> str:
    return str(path.relative_to(out)).replace("\\", "/")


def apply_metric_gate(entry: dict, threshold_em: float) -> None:
    metric = entry["comparisons"]["metricParity"]
    metric_width = float(metric["widthDeltaEm"])
    abs_metric_width = abs(metric_width)
    valid = abs_metric_width <= threshold_em
    entry["metricGate"] = {
        "thresholdEm": threshold_em,
        "widthDeltaEm": metric_width,
        "absoluteWidthDeltaEm": abs_metric_width,
        "status": "valid-for-optical-tuning" if valid else "baseline-mismatch",
        "validForOpticalTuning": valid,
    }


def write_reports(root: Path, args: argparse.Namespace, entries: list[dict]) -> dict:
    out = root / args.output
    report = {
        "schemaVersion": 1,
        "purpose": "Goldfish metric parity gate before optical kerning comparison.",
        "text": args.text,
        "pointSize": float(args.point_size),
        "ligatures": args.ligatures == "true",
        "dpi": int(args.dpi),
        "metricThresholdEm": args.metric_threshold_em,
        "fontCount": len(entries),
        "validFontCount": sum(
            1
            for entry in entries
            if entry.get("metricGate", {}).get("validForOpticalTuning") is True
        ),
        "fonts": entries,
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
        "text": report["text"],
        "pointSize": report["pointSize"],
        "ligatures": report["ligatures"],
        "dpi": report["dpi"],
        "metricThresholdEm": report["metricThresholdEm"],
        "fonts": [
            {
                "fontId": entry["fontId"],
                "fontFamily": entry["fontFamily"],
                "fontPath": entry["fontPath"],
                "metricGate": entry["metricGate"],
                "noneParity": compact_comparison(entry, "noneParity"),
                "metricParity": compact_comparison(entry, "metricParity"),
                "opticalVsGuarded": compact_comparison(entry, "opticalVsGuarded"),
            }
            for entry in report["fonts"]
        ],
    }
    path.write_text(json.dumps(snapshot, indent=2), encoding="utf-8")


def compact_comparison(entry: dict, key: str) -> dict | None:
    data = entry.get("comparisons", {}).get(key)
    if not data:
        return None
    return {
        "widthDeltaEm": data.get("widthDeltaEm"),
        "inkPositionMeanAbsEm": data.get("inkPositionMeanAbsEm"),
        "segmentCenterMeanAbsEm": data.get("segmentCenterMeanAbsEm"),
    }


def width_delta(entry: dict, key: str) -> str:
    data = entry.get("comparisons", {}).get(key)
    if not data:
        return "-"
    return f"{data['widthDeltaPx']:+.0f}px / {data['widthDeltaEm']:+.4f}em"


def position_delta(entry: dict, key: str) -> str:
    data = entry.get("comparisons", {}).get(key)
    if not data:
        return "-"
    ink_em = data.get("inkPositionMeanAbsEm")
    seg_em = data.get("segmentCenterMeanAbsEm")
    if ink_em is None:
        return "-"
    segment = "n/a" if seg_em is None else f"{seg_em:.4f}em"
    return f"ink {ink_em:.4f}em; segments {segment}"


def link(path: str) -> str:
    return quote(path, safe="/.-_")


def gate_class(entry: dict) -> str:
    status = entry.get("metricGate", {}).get("status")
    if status == "valid-for-optical-tuning":
        return "valid"
    if status == "baseline-mismatch":
        return "blocked"
    return "error"


def write_html(out: Path, report: dict) -> None:
    rows = []
    for entry in report["fonts"]:
        klass = gate_class(entry)
        gate = entry.get("metricGate", {})
        if entry["status"] != "ok":
            rows.append(
                f"<tr class=\"{klass}\">"
                f"<td>{html.escape(entry['fontFamily'])}</td>"
                "<td colspan=\"10\">render failed; see log</td>"
                "</tr>"
            )
            continue
        images = entry["images"]
        rows.append(
            f"<tr class=\"{klass}\">"
            f"<td><strong>{html.escape(entry['fontFamily'])}</strong><br>"
            f"<small>{html.escape(entry['fontId'])}</small></td>"
            f"<td><span class=\"pill\">{html.escape(str(gate['status']))}</span><br>"
            f"<small>threshold {gate['thresholdEm']:.4f}em</small></td>"
            f"<td>{html.escape(width_delta(entry, 'noneParity'))}</td>"
            f"<td>{html.escape(width_delta(entry, 'metricParity'))}</td>"
            f"<td>{html.escape(width_delta(entry, 'opticalVsGuarded'))}</td>"
            f"<td>{html.escape(position_delta(entry, 'opticalVsGuarded'))}</td>"
            f"<td><img src=\"{link(images['noneOverlay'])}\" alt=\"None parity overlay\"></td>"
            f"<td><img src=\"{link(images['metricOverlay'])}\" alt=\"Metric parity overlay\"></td>"
            f"<td><img src=\"{link(images['indesignOptical'])}\" alt=\"InDesign Optical\"></td>"
            f"<td><img src=\"{link(images['typstGuarded'])}\" alt=\"Typst Guarded\"></td>"
            f"<td><img src=\"{link(images['opticalOverlay'])}\" alt=\"Optical overlay\"></td>"
            "</tr>"
        )

    html_text = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Goldfish Parity Gate</title>
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
      max-width: 320px;
      max-height: 88px;
      background: white;
    }}
    small {{
      color: color-mix(in srgb, CanvasText 64%, transparent);
    }}
    .pill {{
      display: inline-block;
      border-radius: 999px;
      padding: 2px 8px;
      font-size: 12px;
      border: 1px solid color-mix(in srgb, CanvasText 18%, transparent);
    }}
    tr.valid .pill {{
      background: color-mix(in srgb, #008a4b 18%, Canvas);
    }}
    tr.blocked .pill {{
      background: color-mix(in srgb, #c45a00 20%, Canvas);
    }}
    tr.error .pill {{
      background: color-mix(in srgb, #b00020 16%, Canvas);
    }}
    code {{
      font-size: 14px;
    }}
  </style>
</head>
<body>
  <h1>Goldfish Parity Gate</h1>
  <p>
    Text: <code>{html.escape(report['text'])}</code>, size: {report['pointSize']}pt,
    ligatures: {str(report['ligatures']).lower()}, DPI: {report['dpi']}.
  </p>
  <p>
    A font is only valid for optical tuning when InDesign Metrics and Typst Metrics
    differ by at most {report['metricThresholdEm']:.4f}em in ink width. Overlay
    colors: cyan = InDesign, magenta = Typst, black = overlap.
  </p>
  <table>
    <thead>
      <tr>
        <th>Font</th>
        <th>Gate</th>
        <th>None parity</th>
        <th>Metric parity</th>
        <th>Optical vs guarded</th>
        <th>Ink position</th>
        <th>None overlay</th>
        <th>Metric overlay</th>
        <th>InDesign Optical</th>
        <th>Typst Guarded</th>
        <th>Optical overlay</th>
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
    rows = [entry for entry in report["fonts"] if entry["status"] == "ok"]
    if not rows:
        return

    label_w = 330
    col_w = 340
    row_h = 160
    header_h = 72
    cols = ["None overlay", "Metric overlay", "InDesign Optical", "Typst Guarded", "Optical overlay"]
    width = label_w + col_w * len(cols)
    height = header_h + row_h * len(rows)
    canvas = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    try:
        title_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 18)
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 13)
        small_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 11)
    except OSError:
        title_font = font = small_font = ImageFont.load_default()

    draw.text((16, 14), "Goldfish Parity Gate", fill=(0, 0, 0), font=title_font)
    draw.text(
        (16, 42),
        f"metric threshold {report['metricThresholdEm']:.4f}em; cyan = InDesign, magenta = Typst, black = overlap",
        fill=(70, 70, 70),
        font=small_font,
    )
    for i, col in enumerate(cols):
        draw.text((label_w + i * col_w + 12, 18), col, fill=(0, 0, 0), font=font)

    for row_index, entry in enumerate(rows):
        y = header_h + row_index * row_h
        draw.rectangle((0, y, width, y + row_h), outline=(220, 220, 220))
        gate = entry["metricGate"]
        draw.text((16, y + 16), entry["fontFamily"], fill=(0, 0, 0), font=title_font)
        draw.text(
            (16, y + 42),
            f"{gate['status']} | none {width_delta(entry, 'noneParity')}",
            fill=(30, 30, 30),
            font=font,
        )
        draw.text(
            (16, y + 62),
            f"metric {width_delta(entry, 'metricParity')} | optical {width_delta(entry, 'opticalVsGuarded')}",
            fill=(30, 30, 30),
            font=font,
        )
        draw.text(
            (16, y + 82),
            position_delta(entry, "opticalVsGuarded"),
            fill=(80, 80, 80),
            font=small_font,
        )
        images = entry["images"]
        image_paths = [
            out / images["noneOverlay"],
            out / images["metricOverlay"],
            out / images["indesignOptical"],
            out / images["typstGuarded"],
            out / images["opticalOverlay"],
        ]
        for col_index, image_path in enumerate(image_paths):
            box = (
                label_w + col_index * col_w + 12,
                y + 22,
                label_w + (col_index + 1) * col_w - 12,
                y + row_h - 16,
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
    fonts = load_font_specs(root, args)
    out = root / args.output
    out.mkdir(parents=True, exist_ok=True)
    entries = [run_pipeline(root, args, font, out) for font in fonts]
    report = write_reports(root, args, entries)

    print(f"Summary: {args.output}/summary.json")
    print(f"HTML: {args.output}/index.html")
    print(f"Contact sheet: {args.output}/contact-sheet.png")
    for entry in report["fonts"]:
        gate = entry["metricGate"]["status"]
        metric = entry.get("comparisons", {}).get("metricParity", {})
        optical = entry.get("comparisons", {}).get("opticalVsGuarded", {})
        metric_em = metric.get("widthDeltaEm")
        optical_em = optical.get("widthDeltaEm")
        print(
            f"{entry['fontId']}: {gate}; "
            f"metric={metric_em if metric_em is not None else 'n/a'}em; "
            f"optical={optical_em if optical_em is not None else 'n/a'}em"
        )


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
