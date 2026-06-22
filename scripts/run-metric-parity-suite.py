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


DEFAULT_SAMPLE_MATRIX = {
    "eb-garamond": ["Goldfish", "AV", "VA", "WA", "To", "AVATAR", "WAVY", "ToTaL"],
    "libre-baskerville": ["Goldfish", "AV", "VA", "WA", "To", "AVATAR", "WAVY", "ToTaL"],
    "inter": [
        "Goldfish",
        "WAVY",
        "WAYFINDER",
        "LANDMARK",
        "valley",
        "yellow",
        "lorem",
        "ipsum",
        "OpenType",
        "0123456789",
        "1001",
        "10.000",
        "A10",
        "V2.0",
    ],
}


@dataclass(frozen=True)
class FontSpec:
    font_id: str
    family: str
    font_path: str | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a multi-sample InDesign Metrics vs Typst Metrics gate."
    )
    parser.add_argument(
        "--font-specs",
        default="renders/font-sandbox/goldfish-no-ligature-fonts.json",
        help="JSON list with fontId, family, and fontPath.",
    )
    parser.add_argument(
        "--sample-matrix",
        default="",
        help="Optional JSON object mapping font ids to sample strings.",
    )
    parser.add_argument(
        "--fonts",
        default="",
        help="Optional comma-separated font ids to run from the sample matrix.",
    )
    parser.add_argument("--point-size", default="100")
    parser.add_argument("--ligatures", choices=["true", "false"], default="false")
    parser.add_argument("--dpi", default="300")
    parser.add_argument("--metric-threshold-em", type=float, default=0.02)
    parser.add_argument(
        "--ink-threshold-em",
        type=float,
        default=0.02,
        help="Maximum mean ink-position delta for Metric-vs-Metric parity.",
    )
    parser.add_argument(
        "--output",
        default="renders/metric-parity-suite/no-ligatures-100pt",
    )
    parser.add_argument(
        "--baseline-output",
        default="baselines/metric-parity-suite-v1.json",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-") or "sample"


def load_font_specs(root: Path, path: str) -> dict[str, FontSpec]:
    raw = json.loads((root / path).read_text(encoding="utf-8"))
    specs = {}
    for item in raw:
        font_id = str(item["fontId"])
        specs[font_id] = FontSpec(
            font_id=font_id,
            family=str(item["family"]),
            font_path=str(item["fontPath"]) if item.get("fontPath") else None,
        )
    return specs


def load_sample_matrix(root: Path, args: argparse.Namespace) -> dict[str, list[str]]:
    if args.sample_matrix:
        raw = json.loads((root / args.sample_matrix).read_text(encoding="utf-8"))
        matrix = {str(font_id): [str(sample) for sample in samples] for font_id, samples in raw.items()}
    else:
        matrix = {font_id: list(samples) for font_id, samples in DEFAULT_SAMPLE_MATRIX.items()}
    if args.fonts:
        requested = {font_id.strip() for font_id in args.fonts.split(",") if font_id.strip()}
        matrix = {font_id: samples for font_id, samples in matrix.items() if font_id in requested}
    return matrix


def relative_to_out(path: Path, out: Path) -> str:
    return str(path.relative_to(out)).replace("\\", "/")


def run_case(
    root: Path,
    args: argparse.Namespace,
    font: FontSpec,
    sample: str,
    out: Path,
) -> dict:
    case_dir = out / font.font_id / slug(sample)
    if case_dir.exists():
        shutil.rmtree(case_dir)
    command = [
        str(root / "scripts/run-goldfish-pipeline.sh"),
        "--font-id",
        font.font_id,
        "--font-family",
        font.family,
        "--text",
        sample,
        "--point-size",
        args.point_size,
        "--ligatures",
        args.ligatures,
        "--dpi",
        args.dpi,
        "--output",
        str(case_dir.relative_to(root)),
        "--metric-only",
    ]
    if font.font_path:
        command[3:3] = ["--font-path", font.font_path]

    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    log_dir = out / "logs" / font.font_id
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / f"{slug(sample)}.log"
    log_path.write_text(result.stdout + result.stderr, encoding="utf-8")

    entry: dict = {
        "fontId": font.font_id,
        "fontFamily": font.family,
        "fontPath": font.font_path,
        "sample": sample,
        "pointSize": float(args.point_size),
        "ligatures": args.ligatures == "true",
        "dpi": int(args.dpi),
        "directory": str(case_dir.relative_to(root)),
        "status": "ok" if result.returncode == 0 else "error",
        "returnCode": result.returncode,
        "log": str(log_path.relative_to(root)),
    }

    comparison_path = case_dir / "metrics/comparison.json"
    if result.returncode == 0 and comparison_path.exists():
        comparison = json.loads(comparison_path.read_text(encoding="utf-8"))
        entry["comparisons"] = comparison["comparisons"]
        entry["images"] = {
            "noneOverlay": relative_to_out(case_dir / "overlays/none-parity.png", out),
            "metricOverlay": relative_to_out(case_dir / "overlays/metric-parity.png", out),
            "indesignMetric": relative_to_out(case_dir / "crops/indesign-metric-ink.png", out),
            "typstMetric": relative_to_out(case_dir / "crops/typst-metric-ink.png", out),
        }
        apply_metric_gate(entry, args.metric_threshold_em, args.ink_threshold_em)
    else:
        entry["metricGate"] = {
            "thresholdEm": args.metric_threshold_em,
            "status": "render-error",
            "validForOpticalTuning": False,
        }
    return entry


def apply_metric_gate(entry: dict, width_threshold_em: float, ink_threshold_em: float) -> None:
    metric = entry["comparisons"]["metricParity"]
    width = float(metric["widthDeltaEm"])
    ink = float(metric["inkPositionMeanAbsEm"])
    valid = abs(width) <= width_threshold_em and ink <= ink_threshold_em
    entry["metricGate"] = {
        "thresholdEm": width_threshold_em,
        "inkThresholdEm": ink_threshold_em,
        "widthDeltaEm": width,
        "absoluteWidthDeltaEm": abs(width),
        "inkPositionMeanAbsEm": ink,
        "status": "valid-for-optical-tuning" if valid else "baseline-mismatch",
        "validForOpticalTuning": valid,
    }


def compact(entry: dict) -> dict:
    metric = entry.get("comparisons", {}).get("metricParity")
    none = entry.get("comparisons", {}).get("noneParity")
    return {
        "fontId": entry["fontId"],
        "fontFamily": entry["fontFamily"],
        "fontPath": entry["fontPath"],
        "sample": entry["sample"],
        "metricGate": entry["metricGate"],
        "noneParity": compact_comparison(none),
        "metricParity": compact_comparison(metric),
    }


def compact_comparison(data: dict | None) -> dict | None:
    if not data:
        return None
    return {
        "widthDeltaEm": data.get("widthDeltaEm"),
        "inkPositionMeanAbsEm": data.get("inkPositionMeanAbsEm"),
        "segmentCenterMeanAbsEm": data.get("segmentCenterMeanAbsEm"),
    }


def write_reports(root: Path, args: argparse.Namespace, entries: list[dict]) -> dict:
    out = root / args.output
    valid = [entry for entry in entries if entry.get("metricGate", {}).get("validForOpticalTuning")]
    report = {
        "schemaVersion": 1,
        "purpose": "Multi-sample metric parity gate before optical tuning.",
        "pointSize": float(args.point_size),
        "ligatures": args.ligatures == "true",
        "dpi": int(args.dpi),
        "metricThresholdEm": args.metric_threshold_em,
        "inkThresholdEm": args.ink_threshold_em,
        "caseCount": len(entries),
        "validCaseCount": len(valid),
        "cases": entries,
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "summary.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    (root / args.baseline_output).write_text(
        json.dumps({**report, "cases": [compact(entry) for entry in entries]}, indent=2),
        encoding="utf-8",
    )
    write_html(out, report)
    write_contact_sheet(out, report)
    return report


def width_delta(entry: dict, key: str) -> str:
    data = entry.get("comparisons", {}).get(key)
    if not data:
        return "-"
    return f"{data['widthDeltaPx']:+.0f}px / {data['widthDeltaEm']:+.4f}em"


def link(path: str) -> str:
    return quote(path, safe="/.-_")


def write_html(out: Path, report: dict) -> None:
    rows = []
    for entry in report["cases"]:
        gate = entry["metricGate"]
        klass = "valid" if gate.get("validForOpticalTuning") else "blocked"
        if entry["status"] != "ok":
            klass = "error"
        image = entry.get("images", {}).get("metricOverlay")
        image_html = "" if not image else f"<img src=\"{link(image)}\" alt=\"Metric overlay\">"
        rows.append(
            f"<tr class=\"{klass}\">"
            f"<td>{html.escape(entry['fontFamily'])}</td>"
            f"<td><code>{html.escape(entry['sample'])}</code></td>"
            f"<td>{html.escape(str(gate['status']))}</td>"
            f"<td>{html.escape(width_delta(entry, 'noneParity'))}</td>"
            f"<td>{html.escape(width_delta(entry, 'metricParity'))}</td>"
            f"<td>{image_html}</td>"
            "</tr>"
        )
    html_text = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Metric Parity Suite</title>
  <style>
    :root {{ color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    body {{ margin: 24px; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{ border-bottom: 1px solid color-mix(in srgb, CanvasText 18%, transparent); padding: 8px; text-align: left; vertical-align: middle; }}
    th {{ position: sticky; top: 0; background: Canvas; }}
    img {{ max-width: 360px; max-height: 88px; background: white; }}
    tr.blocked td {{ background: color-mix(in srgb, #c45a00 14%, Canvas); }}
    tr.error td {{ background: color-mix(in srgb, #b00020 14%, Canvas); }}
  </style>
</head>
<body>
  <h1>Metric Parity Suite</h1>
  <p>{report['validCaseCount']} / {report['caseCount']} cases pass the {report['metricThresholdEm']:.4f}em width gate and {report['inkThresholdEm']:.4f}em ink-position gate.</p>
  <table>
    <thead><tr><th>Font</th><th>Sample</th><th>Gate</th><th>None</th><th>Metric</th><th>Overlay</th></tr></thead>
    <tbody>{''.join(rows)}</tbody>
  </table>
</body>
</html>
"""
    (out / "index.html").write_text(html_text, encoding="utf-8")


def write_contact_sheet(out: Path, report: dict) -> None:
    rows = [entry for entry in report["cases"] if entry["status"] == "ok"]
    if not rows:
        return
    label_w = 420
    image_w = 500
    row_h = 120
    header_h = 64
    width = label_w + image_w
    height = header_h + row_h * len(rows)
    canvas = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    try:
        title_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 16)
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 12)
    except OSError:
        title_font = font = ImageFont.load_default()
    draw.text((16, 14), "Metric Parity Suite", fill=(0, 0, 0), font=title_font)
    draw.text((label_w + 12, 14), "Metric overlay: cyan = InDesign, magenta = Typst", fill=(0, 0, 0), font=font)
    for index, entry in enumerate(rows):
        y = header_h + index * row_h
        draw.rectangle((0, y, width, y + row_h), outline=(220, 220, 220))
        draw.text((16, y + 16), entry["fontFamily"], fill=(0, 0, 0), font=title_font)
        draw.text((16, y + 38), entry["sample"], fill=(0, 0, 0), font=font)
        draw.text((16, y + 58), width_delta(entry, "metricParity"), fill=(50, 50, 50), font=font)
        paste_center(canvas, out / entry["images"]["metricOverlay"], (label_w + 12, y + 12, width - 12, y + row_h - 12))
    canvas.save(out / "contact-sheet.png")


def paste_center(canvas: Image.Image, image_path: Path, box: tuple[int, int, int, int]) -> None:
    img = Image.open(image_path).convert("RGB")
    max_w = box[2] - box[0]
    max_h = box[3] - box[1]
    scale = min(max_w / img.width, max_h / img.height, 1.0)
    resized = img.resize((round(img.width * scale), round(img.height * scale)), Image.Resampling.LANCZOS)
    x = box[0] + (max_w - resized.width) // 2
    y = box[1] + (max_h - resized.height) // 2
    canvas.paste(resized, (x, y))


def main() -> None:
    args = parse_args()
    root = repo_root()
    out = root / args.output
    out.mkdir(parents=True, exist_ok=True)
    specs = load_font_specs(root, args.font_specs)
    matrix = load_sample_matrix(root, args)
    entries = []
    for font_id, samples in matrix.items():
        if font_id not in specs:
            raise SystemExit(f"Missing font spec for {font_id}")
        for sample in samples:
            entries.append(run_case(root, args, specs[font_id], sample, out))
            gate = entries[-1]["metricGate"]["status"]
            width = entries[-1].get("comparisons", {}).get("metricParity", {}).get("widthDeltaEm")
            print(f"{font_id} {sample}: {gate}; metric={width if width is not None else 'n/a'}em")
    report = write_reports(root, args, entries)
    failed = report["caseCount"] - report["validCaseCount"]
    print(f"Summary: {args.output}/summary.json")
    print(f"Contact sheet: {args.output}/contact-sheet.png")
    print(f"Metric parity: {report['validCaseCount']} / {report['caseCount']} valid; failed={failed}")
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
