#!/usr/bin/env python3
"""Compare metric and optical compiler paths without external dependencies."""

from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--prototype-typst", type=Path, required=True)
    parser.add_argument("--baseline-typst", default="typst")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument(
        "--document",
        type=Path,
        default=Path("prototypes/typst/compiler-benchmark.typ"),
    )
    parser.add_argument("--font-path", type=Path, default=Path("corpus/fonts"))
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def compile_once(
    executable: str,
    document: Path,
    font_path: Path,
    mode: str,
    output: Path,
) -> tuple[float, int]:
    command = [
        executable,
        "compile",
        "--font-path",
        str(font_path),
        "--input",
        f"mode={mode}",
        str(document),
        str(output),
    ]
    start = time.perf_counter()
    subprocess.run(command, check=True, capture_output=True, text=True)
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    return elapsed_ms, output.stat().st_size


def benchmark_case(
    name: str,
    executable: str,
    mode: str,
    runs: int,
    document: Path,
    font_path: Path,
    directory: Path,
) -> dict[str, object]:
    output = directory / f"{name}.pdf"
    compile_once(executable, document, font_path, mode, output)
    times = []
    output_bytes = 0
    for _ in range(runs):
        elapsed_ms, output_bytes = compile_once(
            executable, document, font_path, mode, output
        )
        times.append(elapsed_ms)
    return {
        "name": name,
        "executable": executable,
        "mode": mode,
        "runs": runs,
        "timesMs": times,
        "medianMs": statistics.median(times),
        "meanMs": statistics.fmean(times),
        "minMs": min(times),
        "maxMs": max(times),
        "outputBytes": output_bytes,
    }


def main() -> None:
    args = parse_args()
    if args.runs < 1:
        raise SystemExit("--runs must be at least 1")

    document = args.document.resolve()
    font_path = args.font_path.resolve()
    prototype = str(args.prototype_typst.resolve())
    cases = [
        ("baseline-metric", args.baseline_typst, "metric"),
        ("prototype-metric", prototype, "metric"),
        ("prototype-headings", prototype, "headings"),
        ("prototype-all", prototype, "all"),
    ]

    with tempfile.TemporaryDirectory(prefix="optikern-typst-bench-") as temp:
        directory = Path(temp)
        results = [
            benchmark_case(
                name,
                executable,
                mode,
                args.runs,
                document,
                font_path,
                directory,
            )
            for name, executable, mode in cases
        ]

    report = {
        "schemaVersion": 1,
        "document": str(document),
        "fontPath": str(font_path),
        "cases": results,
    }
    serialized = json.dumps(report, indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(serialized + "\n", encoding="utf-8")
    print(serialized)


if __name__ == "__main__":
    main()
