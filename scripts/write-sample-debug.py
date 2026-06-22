#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import json
import subprocess
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Write a pair-level debug table for one shaped sample."
    )
    parser.add_argument("--font-id", required=True)
    parser.add_argument("--font-path", default="")
    parser.add_argument("--text", required=True)
    parser.add_argument("--ligatures", choices=["true", "false"], default="false")
    parser.add_argument(
        "--output",
        required=True,
        help="Output directory for debug.json and index.html.",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def sample_deltas(root: Path, args: argparse.Namespace) -> dict:
    command = [
        "cargo",
        "run",
        "-q",
        "-p",
        "optikern-cli",
        "--",
        "sample-deltas",
        "--font-id",
        args.font_id,
        "--text",
        args.text,
        "--ligatures",
        args.ligatures,
    ]
    if args.font_path:
        command[9:9] = ["--font-path", args.font_path]
    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    if result.returncode != 0:
        raise SystemExit(result.stderr)
    return json.loads(result.stdout)


def output_for(pair: dict, algorithm: str) -> dict:
    return next(output for output in pair["outputs"] if output["algorithm"] == algorithm)


def flags(pair: dict, guarded: dict) -> list[str]:
    pair_flags = []
    if abs(pair["metricDeltaEm"]) < 0.006:
        pair_flags.append("metricless")
    if pair["deltaEm"] <= -0.080:
        pair_flags.append("severe-tightening")
    if guarded["gap_min_em"] < -0.020:
        pair_flags.append("connected-or-overlap")
    if pair["opticalDeltaEm"] > 0.006:
        pair_flags.append("profile-wants-opening")
    if pair["opticalDeltaEm"] < -0.006:
        pair_flags.append("profile-wants-tightening")
    if abs(pair["deltaEm"] - pair["opticalDeltaEm"]) > 0.030:
        pair_flags.append("guarded-deviates-from-profile")
    return pair_flags


def fmt(value: float | int | None) -> str:
    if value is None:
        return "n/a"
    return f"{float(value):+.5f}"


def compact(report: dict) -> dict:
    pairs = []
    for pair in report["pairs"]:
        guarded = output_for(pair, "guarded-profile-hybrid")
        pairs.append(
            {
                "display": pair["display"],
                "leftGlyphId": pair["leftGlyphId"],
                "rightGlyphId": pair["rightGlyphId"],
                "metricDeltaEm": pair["metricDeltaEm"],
                "opticalDeltaEm": pair["opticalDeltaEm"],
                "guardedDeltaEm": pair["deltaEm"],
                "targetGapEm": guarded["target_gap_em"],
                "gapMinEm": guarded["gap_min_em"],
                "gapRobustMeanEm": guarded["gap_robust_mean_em"],
                "gapWeightedMeanEm": guarded["gap_weighted_mean_em"],
                "gapMadEm": guarded["gap_mad_em"],
                "flags": flags(pair, guarded),
            }
        )
    return {
        "schemaVersion": 1,
        "fontId": report["fontId"],
        "family": report["family"],
        "fontPath": report["fontPath"],
        "text": report["text"],
        "ligatures": report["ligatures"],
        "pairs": pairs,
    }


def write_html(out: Path, report: dict) -> None:
    rows = []
    for pair in report["pairs"]:
        rows.append(
            "<tr>"
            f"<td><code>{html.escape(pair['display'])}</code><br>"
            f"<small>{pair['leftGlyphId']} -> {pair['rightGlyphId']}</small></td>"
            f"<td>{fmt(pair['metricDeltaEm'])}</td>"
            f"<td>{fmt(pair['opticalDeltaEm'])}</td>"
            f"<td>{fmt(pair['guardedDeltaEm'])}</td>"
            f"<td>{fmt(pair['targetGapEm'])}</td>"
            f"<td>{fmt(pair['gapMinEm'])}</td>"
            f"<td>{fmt(pair['gapRobustMeanEm'])}</td>"
            f"<td>{fmt(pair['gapWeightedMeanEm'])}</td>"
            f"<td>{fmt(pair['gapMadEm'])}</td>"
            f"<td>{html.escape(', '.join(pair['flags']))}</td>"
            "</tr>"
        )
    html_text = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Optikern Sample Debug</title>
  <style>
    :root {{ color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    body {{ margin: 24px; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{ border-bottom: 1px solid color-mix(in srgb, CanvasText 18%, transparent); padding: 8px; text-align: right; vertical-align: top; }}
    th:first-child, td:first-child, th:last-child, td:last-child {{ text-align: left; }}
    th {{ position: sticky; top: 0; background: Canvas; }}
    code {{ font-size: 1.15em; }}
    small {{ color: color-mix(in srgb, CanvasText 65%, transparent); }}
  </style>
</head>
<body>
  <h1>{html.escape(report['text'])}</h1>
  <p>{html.escape(report['family'])} / ligatures {"on" if report['ligatures'] else "off"}</p>
  <table>
    <thead>
      <tr>
        <th>Pair</th>
        <th>Metric</th>
        <th>Profile</th>
        <th>Guarded</th>
        <th>Target gap</th>
        <th>Min gap</th>
        <th>Robust gap</th>
        <th>Weighted gap</th>
        <th>MAD</th>
        <th>Flags</th>
      </tr>
    </thead>
    <tbody>{''.join(rows)}</tbody>
  </table>
</body>
</html>
"""
    (out / "index.html").write_text(html_text, encoding="utf-8")


def main() -> None:
    args = parse_args()
    root = repo_root()
    out = root / args.output
    out.mkdir(parents=True, exist_ok=True)
    report = compact(sample_deltas(root, args))
    (out / "debug.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    write_html(out, report)
    print(f"Debug JSON: {args.output}/debug.json")
    print(f"Debug HTML: {args.output}/index.html")


if __name__ == "__main__":
    main()
