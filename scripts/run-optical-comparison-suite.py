#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from urllib.parse import quote

from PIL import Image, ImageDraw, ImageFont


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compare InDesign Optical against Typst Guarded Optical for metric-valid samples."
    )
    parser.add_argument(
        "--suite",
        choices=["fast", "cross-font", "full"],
        default="full",
        help="Case set to render. Defaults to full.",
    )
    parser.add_argument(
        "--suite-file",
        default="",
        help="Optional explicit suite JSON file. Overrides --suite.",
    )
    parser.add_argument(
        "--metric-baseline",
        default="baselines/metric-parity-suite-v1.json",
        help="Metric parity baseline; only valid cases are rendered.",
    )
    parser.add_argument(
        "--output",
        default="",
    )
    parser.add_argument(
        "--baseline-output",
        default="",
    )
    parser.add_argument("--dpi", default="")
    parser.add_argument("--point-size", default="")
    parser.add_argument("--ligatures", choices=["true", "false", ""], default="")
    args = parser.parse_args()
    name = suite_name(args)
    if not args.output:
        args.output = f"renders/optical-comparison-suite/no-ligatures-100pt-{name}"
    if not args.baseline_output:
        args.baseline_output = f"baselines/optical-comparison-suite-{name}.json"
    return args


def suite_name(args: argparse.Namespace) -> str:
    if not args.suite_file:
        return args.suite
    stem = Path(args.suite_file).stem
    return stem.removeprefix("optical-").removesuffix("-suite")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-") or "sample"


def relative_to_out(path: Path, out: Path) -> str:
    return str(path.relative_to(out)).replace("\\", "/")


def load_cases(root: Path, args: argparse.Namespace) -> tuple[dict, list[dict]]:
    baseline = json.loads((root / args.metric_baseline).read_text(encoding="utf-8"))
    valid_cases = [
        case
        for case in baseline["cases"]
        if case.get("metricGate", {}).get("validForOpticalTuning") is True
    ]
    suite_path = suite_file(root, args)
    if suite_path is None:
        return baseline, valid_cases
    suite = json.loads(suite_path.read_text(encoding="utf-8"))
    return baseline, select_suite_cases(valid_cases, suite, suite_path)


def suite_file(root: Path, args: argparse.Namespace) -> Path | None:
    if args.suite_file:
        return root / args.suite_file
    return root / "corpus/samples" / f"optical-{args.suite}-suite.json"


def select_suite_cases(valid_cases: list[dict], suite: dict, suite_path: Path) -> list[dict]:
    by_key = {(case["fontId"], case["sample"]): case for case in valid_cases}
    selected = []
    seen = set()
    for item in suite.get("cases", []):
        key = (item["fontId"], item["sample"])
        if key in seen:
            raise SystemExit(f"Duplicate suite case in {suite_path}: {key[0]} / {key[1]}")
        seen.add(key)
        case = by_key.get(key)
        if case is None:
            raise SystemExit(
                f"Suite case is missing from valid metric baseline cases: {key[0]} / {key[1]}"
            )
        selected.append(case)
    if not selected:
        raise SystemExit(f"Suite file has no cases: {suite_path}")
    return selected


def case_value(args_value: str, baseline_value) -> str:
    return args_value if args_value else str(baseline_value).lower()


def run_case(root: Path, args: argparse.Namespace, baseline: dict, case: dict, out: Path) -> dict:
    font_id = case["fontId"]
    sample = case["sample"]
    case_dir = out / font_id / slug(sample)
    if case_dir.exists():
        shutil.rmtree(case_dir)

    point_size = case_value(args.point_size, case.get("pointSize", baseline["pointSize"]))
    ligatures = case_value(args.ligatures, case.get("ligatures", baseline["ligatures"]))
    dpi = case_value(args.dpi, baseline["dpi"])
    command = [
        str(root / "scripts/run-goldfish-pipeline.sh"),
        "--font-id",
        font_id,
        "--font-family",
        case["fontFamily"],
        "--text",
        sample,
        "--point-size",
        point_size,
        "--ligatures",
        ligatures,
        "--dpi",
        dpi,
        "--output",
        str(case_dir.relative_to(root)),
    ]
    if case.get("fontPath"):
        command[3:3] = ["--font-path", case["fontPath"]]

    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    log_dir = out / "logs" / font_id
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / f"{slug(sample)}.log"
    log_path.write_text(result.stdout + result.stderr, encoding="utf-8")

    entry = {
        "fontId": font_id,
        "fontFamily": case["fontFamily"],
        "fontPath": case.get("fontPath"),
        "sample": sample,
        "pointSize": float(point_size),
        "ligatures": ligatures == "true",
        "dpi": int(dpi),
        "directory": str(case_dir.relative_to(root)),
        "status": "ok" if result.returncode == 0 else "error",
        "returnCode": result.returncode,
        "log": str(log_path.relative_to(root)),
        "metricGate": case["metricGate"],
    }

    comparison_path = case_dir / "metrics/comparison.json"
    deltas_path = case_dir / "metrics/guarded-deltas.json"
    if result.returncode == 0 and comparison_path.exists():
        comparison = json.loads(comparison_path.read_text(encoding="utf-8"))
        entry["comparisons"] = comparison["comparisons"]
        entry["guardedDeltas"] = json.loads(deltas_path.read_text(encoding="utf-8"))
        entry["images"] = {
            "metricOverlay": relative_to_out(case_dir / "overlays/metric-parity.png", out),
            "indesignOptical": relative_to_out(case_dir / "crops/indesign-optical-ink.png", out),
            "typstGuarded": relative_to_out(case_dir / "crops/typst-guarded-ink.png", out),
            "opticalOverlay": relative_to_out(case_dir / "overlays/optical-vs-guarded.png", out),
        }
        entry["opticalScore"] = optical_score(entry)
    else:
        entry["opticalScore"] = {"status": "render-error"}
    return entry


def optical_score(entry: dict) -> dict:
    data = entry["comparisons"]["opticalVsGuarded"]
    width = abs(float(data["widthDeltaEm"]))
    ink = float(data["inkPositionMeanAbsEm"])
    segment = data.get("segmentCenterMeanAbsEm")
    segment_value = 0.0 if segment is None else float(segment)
    score = max(width, ink, segment_value)
    return {
        "status": "measured",
        "scoreEm": score,
        "absoluteWidthDeltaEm": width,
        "widthDeltaEm": data["widthDeltaEm"],
        "inkPositionMeanAbsEm": ink,
        "segmentCenterMeanAbsEm": segment,
    }


def compact(entry: dict) -> dict:
    return {
        "fontId": entry["fontId"],
        "fontFamily": entry["fontFamily"],
        "fontPath": entry["fontPath"],
        "sample": entry["sample"],
        "metricGate": entry["metricGate"],
        "opticalScore": entry["opticalScore"],
        "deltas": compact_deltas(entry.get("guardedDeltas", {})),
    }


def compact_deltas(report: dict) -> list[dict]:
    return [
        {
            "display": pair["display"],
            "leftGlyphId": pair["leftGlyphId"],
            "rightGlyphId": pair["rightGlyphId"],
            "deltaEm": pair["deltaEm"],
            "metricDeltaEm": pair["metricDeltaEm"],
            "opticalDeltaEm": pair["opticalDeltaEm"],
        }
        for pair in report.get("pairs", [])
    ]


def write_reports(root: Path, args: argparse.Namespace, baseline: dict, entries: list[dict]) -> dict:
    out = root / args.output
    measured = [entry for entry in entries if entry.get("opticalScore", {}).get("status") == "measured"]
    ranked = sorted(
        measured,
        key=lambda entry: entry["opticalScore"]["scoreEm"],
        reverse=True,
    )
    report = {
        "schemaVersion": 1,
        "purpose": "InDesign Optical vs Typst Guarded Optical comparison for metric-valid samples.",
        "suite": args.suite_file or args.suite,
        "metricBaseline": args.metric_baseline,
        "pointSize": float(case_value(args.point_size, baseline["pointSize"])),
        "ligatures": case_value(args.ligatures, baseline["ligatures"]) == "true",
        "dpi": int(case_value(args.dpi, baseline["dpi"])),
        "caseCount": len(entries),
        "measuredCaseCount": len(measured),
        "cases": entries,
        "worstCases": [compact(entry) for entry in ranked[:10]],
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "summary.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    baseline_path = root / args.baseline_output
    baseline_path.parent.mkdir(parents=True, exist_ok=True)
    baseline_path.write_text(
        json.dumps({**report, "cases": [compact(entry) for entry in entries]}, indent=2),
        encoding="utf-8",
    )
    write_html(out, report)
    write_contact_sheet(out, report)
    return report


def fmt(value) -> str:
    if value is None:
        return "n/a"
    return f"{float(value):+.4f}em"


def link(path: str) -> str:
    return quote(path, safe="/.-_")


def write_html(out: Path, report: dict) -> None:
    rows_to_render = report_rows(report)
    rows = []
    previous_group = None
    for entry in rows_to_render:
        score = entry.get("opticalScore", {})
        images = entry.get("images", {})
        overlay = images.get("opticalOverlay")
        overlay_html = "" if not overlay else f"<img src=\"{link(overlay)}\" alt=\"Optical overlay\">"
        group = report_group_key(report, entry)
        group_class = "group-start" if previous_group is not None and previous_group != group else ""
        rows.append(
            f"<tr class=\"{group_class}\">"
            f"<td>{html.escape(entry['fontFamily'])}</td>"
            f"<td><code>{html.escape(entry['sample'])}</code></td>"
            f"<td>{fmt(score.get('widthDeltaEm'))}</td>"
            f"<td>{fmt(score.get('inkPositionMeanAbsEm'))}</td>"
            f"<td>{fmt(score.get('segmentCenterMeanAbsEm'))}</td>"
            f"<td>{overlay_html}</td>"
            "</tr>"
        )
        previous_group = group
    html_text = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Optical Comparison Suite</title>
  <style>
    :root {{ color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    body {{ margin: 24px; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{ border-bottom: 1px solid color-mix(in srgb, CanvasText 18%, transparent); padding: 8px; text-align: left; vertical-align: middle; }}
    th {{ position: sticky; top: 0; background: Canvas; }}
    tr.group-start td {{ border-top: 3px solid color-mix(in srgb, CanvasText 35%, transparent); }}
    img {{ max-width: 420px; max-height: 96px; background: white; }}
  </style>
</head>
<body>
  <h1>Optical Comparison Suite</h1>
  <p>{report['measuredCaseCount']} / {report['caseCount']} metric-valid cases measured. Cyan = InDesign Optical, magenta = Typst Guarded, black = overlap.</p>
  <table>
    <thead><tr><th>Font</th><th>Sample</th><th>Width</th><th>Ink position</th><th>Segment center</th><th>Overlay</th></tr></thead>
    <tbody>{''.join(rows)}</tbody>
  </table>
</body>
</html>
"""
    (out / "index.html").write_text(html_text, encoding="utf-8")


def write_contact_sheet(out: Path, report: dict) -> None:
    rows = [entry for entry in report_rows(report) if entry["status"] == "ok"]
    if not rows:
        return
    label_w = 430
    col_w = 360
    row_h = 130
    header_h = 70
    cols = ["InDesign Optical", "Typst Guarded", "Optical overlay"]
    width = label_w + col_w * len(cols)
    height = header_h + row_h * len(rows)
    canvas = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(canvas)
    try:
        title_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 16)
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 12)
        small = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 10)
    except OSError:
        title_font = font = small = ImageFont.load_default()
    draw.text((16, 14), "Optical Comparison Suite", fill=(0, 0, 0), font=title_font)
    draw.text((16, 38), report_order_label(report), fill=(60, 60, 60), font=small)
    for i, col in enumerate(cols):
        draw.text((label_w + i * col_w + 12, 18), col, fill=(0, 0, 0), font=font)
    column_lines = [label_w + i * col_w for i in range(len(cols) + 1)]
    for index, entry in enumerate(rows):
        y = header_h + index * row_h
        score = entry["opticalScore"]
        group_start = is_group_start(report, rows, index)
        outline = (170, 170, 170) if group_start else (220, 220, 220)
        line_width = 3 if group_start else 1
        draw.rectangle((0, y, width, y + row_h), outline=outline, width=line_width)
        draw.text((16, y + 14), entry["fontFamily"], fill=(0, 0, 0), font=title_font)
        draw.text((16, y + 36), entry["sample"], fill=(0, 0, 0), font=font)
        draw.text(
            (16, y + 58),
            f"width {fmt(score['widthDeltaEm'])}; ink {fmt(score['inkPositionMeanAbsEm'])}",
            fill=(50, 50, 50),
            font=font,
        )
        draw.text(
            (16, y + 78),
            f"segment {fmt(score['segmentCenterMeanAbsEm'])}; score {fmt(score['scoreEm'])}",
            fill=(70, 70, 70),
            font=small,
        )
        images = entry["images"]
        for col_index, key in enumerate(["indesignOptical", "typstGuarded", "opticalOverlay"]):
            paste_center(
                canvas,
                out / images[key],
                (
                    label_w + col_index * col_w + 12,
                    y + 14,
                    label_w + (col_index + 1) * col_w - 12,
                    y + row_h - 14,
                ),
            )
    for x in column_lines:
        draw.line((x, 0, x, height), fill=(185, 185, 185), width=2)
    draw.line((0, header_h, width, header_h), fill=(185, 185, 185), width=2)
    canvas.save(out / "contact-sheet.png")


def report_rows(report: dict) -> list[dict]:
    if report.get("suite") == "cross-font":
        return list(report["cases"])
    return sorted(
        report["cases"],
        key=lambda entry: entry.get("opticalScore", {}).get("scoreEm", -1),
        reverse=True,
    )


def report_order_label(report: dict) -> str:
    if report.get("suite") == "cross-font":
        return "grouped by sample; fonts keep suite order"
    return "sorted by worst guarded-vs-InDesign-optical score"


def is_group_start(report: dict, rows: list[dict], index: int) -> bool:
    return index > 0 and report_group_key(report, rows[index - 1]) != report_group_key(report, rows[index])


def report_group_key(report: dict, entry: dict) -> str:
    if report.get("suite") == "cross-font":
        return entry["sample"]
    return entry["fontId"]


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
    baseline, cases = load_cases(root, args)
    entries = []
    for case in cases:
        entry = run_case(root, args, baseline, case, out)
        entries.append(entry)
        score = entry.get("opticalScore", {})
        print(
            f"{entry['fontId']} {entry['sample']}: "
            f"width={score.get('widthDeltaEm', 'n/a')}em; "
            f"ink={score.get('inkPositionMeanAbsEm', 'n/a')}em"
        )
    report = write_reports(root, args, baseline, entries)
    print(f"Summary: {args.output}/summary.json")
    print(f"Contact sheet: {args.output}/contact-sheet.png")
    print(f"Measured: {report['measuredCaseCount']} / {report['caseCount']}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
