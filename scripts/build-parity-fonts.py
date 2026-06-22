#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class ParityFont:
    font_id: str
    source: str
    family: str
    output_name: str
    axes: tuple[tuple[str, float], ...]


PARITY_FONTS = (
    ParityFont(
        font_id="eb-garamond",
        source="corpus/fonts/eb-garamond.ttf",
        family="Optikern EB Garamond NoLiga",
        output_name="optikern-eb-garamond-regular-static-noliga.ttf",
        axes=(("wght", 400.0),),
    ),
    ParityFont(
        font_id="libre-baskerville",
        source="corpus/fonts/libre-baskerville.ttf",
        family="Optikern Libre Baskerville NoLiga",
        output_name="optikern-libre-baskerville-regular-static-noliga.ttf",
        axes=(("wght", 400.0),),
    ),
    ParityFont(
        font_id="inter",
        source="corpus/fonts/inter.ttf",
        family="Optikern Inter NoLiga",
        output_name="optikern-inter-regular-opsz14-static-noliga.ttf",
        axes=(("wght", 400.0), ("opsz", 14.0)),
    ),
    ParityFont(
        font_id="pacifico",
        source="corpus/fonts/pacifico.ttf",
        family="Optikern Pacifico NoLiga",
        output_name="optikern-pacifico-regular-noliga.ttf",
        axes=(),
    ),
    ParityFont(
        font_id="lobster",
        source="corpus/fonts/lobster.ttf",
        family="Optikern Lobster NoLiga",
        output_name="optikern-lobster-regular-noliga.ttf",
        axes=(),
    ),
    ParityFont(
        font_id="comic-neue",
        source="corpus/fonts/comic-neue.ttf",
        family="Optikern Comic Neue NoLiga",
        output_name="optikern-comic-neue-regular-noliga.ttf",
        axes=(),
    ),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build isolated parity fonts for InDesign-vs-Typst metric baseline checks."
    )
    parser.add_argument(
        "--variant",
        choices=["no-ligatures", "ligatures"],
        default="no-ligatures",
        help="Build fonts with ligature features removed or retained. Default: no-ligatures.",
    )
    parser.add_argument(
        "--output-dir",
        default="renders/font-sandbox",
        help="Directory for generated fonts. Default: renders/font-sandbox.",
    )
    parser.add_argument(
        "--spec-output",
        default="",
        help="JSON font spec consumed by run-goldfish-parity.py.",
    )
    parser.add_argument(
        "--python",
        default=sys.executable,
        help="Python executable with fontTools installed. Default: current Python.",
    )
    return parser.parse_args()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def relative_to_root(path: Path, root: Path) -> str:
    return str(path.relative_to(root)).replace("\\", "/")


def require_fonttools(args: argparse.Namespace) -> None:
    result = subprocess.run(
        [args.python, "-c", "import fontTools"],
        text=True,
        capture_output=True,
    )
    if result.returncode == 0:
        return
    raise SystemExit(
        "fontTools is required for parity font generation. "
        "Create a Python environment and install requirements-fonttools.txt, "
        "then pass that interpreter with --python if needed."
    )


def axis_args(font: ParityFont) -> list[str]:
    args: list[str] = []
    for tag, value in font.axes:
        args.extend(["--axis", f"{tag}={value:g}"])
    return args


def variant_family(font: ParityFont, variant: str) -> str:
    if variant == "ligatures":
        return font.family.removesuffix(" NoLiga") + " Liga"
    return font.family


def variant_output_name(font: ParityFont, variant: str) -> str:
    if variant == "ligatures":
        return font.output_name.replace("-noliga", "-liga")
    return font.output_name


def feature_args(variant: str) -> list[str]:
    if variant == "ligatures":
        return []
    return [
        "--drop-feature",
        "liga",
        "--drop-feature",
        "clig",
        "--drop-feature",
        "dlig",
    ]


def isolation_args(variant: str) -> list[str]:
    if variant == "ligatures":
        return []
    return [
        "--strip-glyph-names",
        "--drop-ligature-cmap",
    ]


def build_font(root: Path, args: argparse.Namespace, font: ParityFont) -> dict:
    source = root / font.source
    if not source.exists():
        raise SystemExit(f"Missing source font: {font.source}")

    output_dir = root / args.output_dir
    family = variant_family(font, args.variant)
    output = output_dir / variant_output_name(font, args.variant)
    output_dir.mkdir(parents=True, exist_ok=True)

    command = [
        args.python,
        str(root / "scripts/rename-font-family.py"),
        str(source),
        str(output),
        "--family",
        family,
        "--style",
        "Regular",
        *axis_args(font),
        *feature_args(args.variant),
        *isolation_args(args.variant),
    ]
    subprocess.run(command, cwd=root, check=True)
    return {
        "fontId": font.font_id,
        "family": family,
        "fontPath": relative_to_root(output, root),
    }


def spec_output_path(root: Path, args: argparse.Namespace) -> Path:
    if args.spec_output:
        return root / args.spec_output
    name = (
        "goldfish-no-ligature-fonts.json"
        if args.variant == "no-ligatures"
        else "goldfish-ligature-fonts.json"
    )
    return root / args.output_dir / name


def main() -> None:
    args = parse_args()
    require_fonttools(args)
    root = repo_root()
    specs = [build_font(root, args, font) for font in PARITY_FONTS]
    spec_output = spec_output_path(root, args)
    spec_output.parent.mkdir(parents=True, exist_ok=True)
    spec_output.write_text(json.dumps(specs, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {relative_to_root(spec_output, root)}")
    for spec in specs:
        print(f"{spec['fontId']}: {spec['family']} -> {spec['fontPath']}")


if __name__ == "__main__":
    main()
