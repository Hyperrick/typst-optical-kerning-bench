#!/usr/bin/env python3
"""Verify that the Typst hook applies the workbench candidate deltas."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


LENGTH_RE = re.compile(r"^([+-]?(?:\d+(?:\.\d*)?|\.\d+))pt$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--typst", type=Path, required=True)
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--source",
        type=Path,
        default=Path("prototypes/typst/measure-sample.typ"),
    )
    parser.add_argument(
        "--font-path", type=Path, default=Path("renders/font-sandbox")
    )
    parser.add_argument("--tolerance-em", type=float, default=0.00011)
    return parser.parse_args()


def measured_correction(
    typst: Path,
    source: Path,
    font_path: Path,
    family: str,
    sample: str,
    ligatures: bool,
) -> float:
    command = [
        str(typst),
        "eval",
        "query(<optical-measurement>).first().value",
        "--in",
        str(source),
        "--font-path",
        str(font_path),
        "--input",
        f"font={family}",
        "--input",
        f"sample={sample}",
        "--input",
        f"ligatures={'true' if ligatures else 'false'}",
    ]
    result = subprocess.run(command, check=True, capture_output=True, text=True)
    value = json.loads(result.stdout)
    match = LENGTH_RE.fullmatch(value["correction"])
    if not match:
        raise RuntimeError(f"unexpected Typst length: {value['correction']!r}")
    return float(match.group(1)) / 100.0


def expected_correction(case: dict[str, object]) -> float:
    pairs = case["guardedDeltas"]["pairs"]
    return sum(pair["deltaEm"] - pair["metricDeltaEm"] for pair in pairs)


def main() -> None:
    args = parse_args()
    summary = json.loads(args.summary.read_text(encoding="utf-8"))
    results = []

    for case in summary["cases"]:
        expected = expected_correction(case)
        measured = measured_correction(
            args.typst.resolve(),
            args.source.resolve(),
            args.font_path.resolve(),
            case["fontFamily"],
            case["sample"],
            summary["ligatures"],
        )
        error = abs(measured - expected)
        results.append(
            {
                "fontId": case["fontId"],
                "sample": case["sample"],
                "expectedCorrectionEm": expected,
                "measuredCorrectionEm": measured,
                "absoluteErrorEm": error,
                "passed": error <= args.tolerance_em,
            }
        )

    report = {
        "schemaVersion": 1,
        "summary": str(args.summary),
        "caseCount": len(results),
        "toleranceEm": args.tolerance_em,
        "maxAbsoluteErrorEm": max(result["absoluteErrorEm"] for result in results),
        "passed": all(result["passed"] for result in results),
        "cases": results,
    }
    serialized = json.dumps(report, indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(serialized + "\n", encoding="utf-8")
    print(serialized)
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
