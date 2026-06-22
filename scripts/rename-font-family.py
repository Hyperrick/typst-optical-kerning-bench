#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont


FAMILY_NAME_IDS = {1, 16}
SUBFAMILY_NAME_IDS = {2, 17}
FULL_NAME_IDS = {3, 4}
POSTSCRIPT_NAME_IDS = {6, 25}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Rewrite a font family name for isolated parity tests.")
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument("--family", required=True)
    parser.add_argument("--style", default="Regular")
    parser.add_argument(
        "--axis",
        action="append",
        default=[],
        help="Freeze a variable axis, for example --axis wght=400.",
    )
    parser.add_argument(
        "--drop-feature",
        action="append",
        default=[],
        help="Remove a GSUB feature tag, for example --drop-feature liga.",
    )
    parser.add_argument(
        "--strip-glyph-names",
        action="store_true",
        help="Write post table format 3.0 so legacy glyph names are unavailable.",
    )
    parser.add_argument(
        "--drop-ligature-cmap",
        action="store_true",
        help="Remove Unicode presentation-form ligature cmap entries U+FB00..U+FB04.",
    )
    return parser.parse_args()


def postscript_name(family: str, style: str) -> str:
    clean_family = "".join(ch for ch in family if ch.isalnum())
    clean_style = "".join(ch for ch in style if ch.isalnum()) or "Regular"
    return f"{clean_family}-{clean_style}"


def set_name(record, value: str) -> None:
    record.string = value.encode(record.getEncoding(), errors="replace")


def parse_axes(values: list[str]) -> dict[str, float]:
    axes = {}
    for value in values:
        if "=" not in value:
            raise SystemExit(f"Invalid --axis value: {value}")
        tag, raw = value.split("=", 1)
        axes[tag.strip()] = float(raw)
    return axes


def drop_gsub_features(font: TTFont, tags: set[str]) -> None:
    if "GSUB" not in font or not tags:
        return
    gsub = font["GSUB"].table
    feature_list = getattr(gsub, "FeatureList", None)
    script_list = getattr(gsub, "ScriptList", None)
    if feature_list is None or script_list is None:
        return

    old_records = list(feature_list.FeatureRecord)
    drop_indices = {
        index for index, record in enumerate(old_records) if record.FeatureTag in tags
    }
    if not drop_indices:
        return
    index_map = {}
    kept_records = []
    for old_index, record in enumerate(old_records):
        if old_index in drop_indices:
            continue
        index_map[old_index] = len(kept_records)
        kept_records.append(record)

    for script_record in script_list.ScriptRecord:
        lang_systems = []
        default_lang = getattr(script_record.Script, "DefaultLangSys", None)
        if default_lang is not None:
            lang_systems.append(default_lang)
        for lang_record in getattr(script_record.Script, "LangSysRecord", []):
            lang_systems.append(lang_record.LangSys)
        for lang_system in lang_systems:
            if getattr(lang_system, "ReqFeatureIndex", 0xFFFF) in drop_indices:
                lang_system.ReqFeatureIndex = 0xFFFF
            lang_system.FeatureIndex = [
                index_map[index]
                for index in lang_system.FeatureIndex
                if index in index_map
            ]
            lang_system.FeatureCount = len(lang_system.FeatureIndex)

    feature_list.FeatureRecord = kept_records
    feature_list.FeatureCount = len(kept_records)


def strip_glyph_names(font: TTFont) -> None:
    if "post" not in font:
        return
    post = font["post"]
    post.formatType = 3.0
    post.extraNames = []
    post.mapping = {}


def drop_ligature_cmap(font: TTFont) -> None:
    if "cmap" not in font:
        return
    for table in font["cmap"].tables:
        for codepoint in range(0xFB00, 0xFB05):
            table.cmap.pop(codepoint, None)


def main() -> None:
    args = parse_args()
    font = TTFont(args.input)
    axes = parse_axes(args.axis)
    if axes:
        font = instantiateVariableFont(font, axes, inplace=False)
    drop_gsub_features(font, set(args.drop_feature))
    if args.strip_glyph_names:
        strip_glyph_names(font)
    if args.drop_ligature_cmap:
        drop_ligature_cmap(font)
    family = args.family
    style = args.style
    full = f"{family} {style}".strip()
    ps_name = postscript_name(family, style)

    for record in font["name"].names:
        if record.nameID in FAMILY_NAME_IDS:
            set_name(record, family)
        elif record.nameID in SUBFAMILY_NAME_IDS:
            set_name(record, style)
        elif record.nameID in FULL_NAME_IDS:
            set_name(record, full)
        elif record.nameID in POSTSCRIPT_NAME_IDS:
            set_name(record, ps_name)

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    font.save(output)


if __name__ == "__main__":
    main()
