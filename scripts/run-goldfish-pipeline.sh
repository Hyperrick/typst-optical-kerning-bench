#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/run-goldfish-pipeline.sh [options]

Defaults reproduce the current Goldfish benchmark:
  EB Garamond, 100pt, Goldfish, ligatures=false, 300 DPI.

Options:
  --font-id ID          Font file id in corpus/fonts. Default: eb-garamond.
  --font-path PATH      Font file to render. Default: corpus/fonts/<font-id>.ttf.
  --font-family NAME    InDesign/Typst font family. Default: EB Garamond.
  --text TEXT           Text to render. Default: Goldfish.
  --point-size PT       Text size. Default: 100.
  --ligatures BOOL      true or false. Default: false.
  --dpi DPI             Raster DPI. Default: 300.
  --output DIR          Output directory. Default derived from text/settings.
  --metric-only         Render only None and Metrics parity outputs.
  -h, --help            Show this help.
USAGE
}

font_id="eb-garamond"
font_path=""
font_family="EB Garamond"
text="Goldfish"
point_size="100"
ligatures="false"
dpi="300"
output_dir=""
metric_only="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --font-id) font_id="$2"; shift 2 ;;
    --font-path) font_path="$2"; shift 2 ;;
    --font-family) font_family="$2"; shift 2 ;;
    --text) text="$2"; shift 2 ;;
    --point-size) point_size="$2"; shift 2 ;;
    --ligatures) ligatures="$2"; shift 2 ;;
    --dpi) dpi="$2"; shift 2 ;;
    --output) output_dir="$2"; shift 2 ;;
    --metric-only) metric_only="true"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$ligatures" in
  true|false) ;;
  *) echo "--ligatures must be true or false" >&2; exit 2 ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -z "$output_dir" ]]; then
  text_slug="$(python3 - "$text" <<'PY'
import re, sys
print(re.sub(r"[^a-z0-9]+", "-", sys.argv[1].lower()).strip("-"))
PY
)"
  lig_slug="$([[ "$ligatures" == "true" ]] && echo "ligatures" || echo "no-ligatures")"
  output_dir="renders/${text_slug}/${lig_slug}-${point_size}pt"
fi

if [[ -z "$font_path" ]]; then
  font_path="corpus/fonts/${font_id}.ttf"
fi
if [[ ! -f "$font_path" ]]; then
  echo "Missing font file: $font_path" >&2
  exit 1
fi

mkdir -p \
  "$output_dir/indesign/Document fonts" \
  "$output_dir/typst/fonts" \
  "$output_dir/typst" \
  "$output_dir/crops" \
  "$output_dir/overlays" \
  "$output_dir/metrics"
font_filename="$(basename "$font_path")"
cp "$font_path" "$output_dir/indesign/Document fonts/$font_filename"
cp "$font_path" "$output_dir/typst/fonts/$font_filename"

if [[ "$metric_only" == "false" ]]; then
  echo "== Deltas =="
  cargo run -q -p optikern-cli -- sample-deltas \
    --font-id "$font_id" \
    --font-path "$font_path" \
    --text "$text" \
    --ligatures="$ligatures" \
    > "$output_dir/metrics/guarded-deltas.json"
fi

echo "== InDesign =="
scripts/render-indesign-outlined-text.sh \
  --font-family "$font_family" \
  --text "$text" \
  --kerning none \
  --ligatures "$ligatures" \
  --point-size "$point_size" \
  --output-pdf "$repo_root/$output_dir/indesign/none.tmp.pdf" \
  --output-indd "$repo_root/$output_dir/indesign/none.indd" \
  --output-json "$repo_root/$output_dir/metrics/indesign-none.json"

scripts/render-indesign-outlined-text.sh \
  --font-family "$font_family" \
  --text "$text" \
  --kerning metrics \
  --ligatures "$ligatures" \
  --point-size "$point_size" \
  --output-pdf "$repo_root/$output_dir/indesign/metric.tmp.pdf" \
  --output-indd "$repo_root/$output_dir/indesign/metric.indd" \
  --output-json "$repo_root/$output_dir/metrics/indesign-metric.json"

if [[ "$metric_only" == "false" ]]; then
  scripts/render-indesign-outlined-text.sh \
    --font-family "$font_family" \
    --text "$text" \
    --kerning optical \
    --ligatures "$ligatures" \
    --point-size "$point_size" \
    --output-pdf "$repo_root/$output_dir/indesign/optical.tmp.pdf" \
    --output-indd "$repo_root/$output_dir/indesign/optical.indd" \
    --output-json "$repo_root/$output_dir/metrics/indesign-optical.json"
fi

pdftoppm -png -r "$dpi" "$output_dir/indesign/none.tmp.pdf" "$output_dir/indesign/none"
pdftoppm -png -r "$dpi" "$output_dir/indesign/metric.tmp.pdf" "$output_dir/indesign/metric"
if [[ "$metric_only" == "false" ]]; then
  pdftoppm -png -r "$dpi" "$output_dir/indesign/optical.tmp.pdf" "$output_dir/indesign/optical"
fi
rm -f "$output_dir/indesign/"*.tmp.pdf
rm -f "$output_dir/indesign/"*.idlk

echo "== Typst =="
python3 - "$output_dir" "$font_family" "$text" "$point_size" "$ligatures" "$metric_only" <<'PY'
from pathlib import Path
import json
import sys

out = Path(sys.argv[1])
font_family = sys.argv[2]
text = sys.argv[3]
point_size = float(sys.argv[4])
ligatures = sys.argv[5] == "true"
metric_only = sys.argv[6] == "true"
deltas = None
if not metric_only:
    deltas = json.loads((out / "metrics/guarded-deltas.json").read_text(encoding="utf-8"))
margin = point_size * 0.8

def typ_content(value: str) -> str:
    return (
        value
        .replace("\\", "\\\\")
        .replace("#", "\\#")
        .replace("[", "\\[")
        .replace("]", "\\]")
    )

def typ_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')

features = "(liga: 1, clig: 1, calt: 1)" if ligatures else "(liga: 0, clig: 0, calt: 0)"
metric = f"""#set page(width: auto, height: auto, margin: {margin:.4f}pt)
#set text(
  font: "{typ_string(font_family)}",
  size: {point_size:.4f}pt,
  kerning: true,
  ligatures: {"true" if ligatures else "false"},
  features: {features},
)
{typ_content(text)}
"""
(out / "typst/metric.typ").write_text(metric, encoding="utf-8")

none = f"""#set page(width: auto, height: auto, margin: {margin:.4f}pt)
#set text(
  font: "{typ_string(font_family)}",
  size: {point_size:.4f}pt,
  kerning: false,
  ligatures: {"true" if ligatures else "false"},
  features: {features},
)
{typ_content(text)}
"""
(out / "typst/none.typ").write_text(none, encoding="utf-8")

if not metric_only:
    pairs = deltas["pairs"]
    if not pairs:
        guarded_body = typ_content(text)
    else:
        parts = [typ_content(pairs[0]["leftCluster"])]
        for pair in pairs:
            parts.append(f'#h({pair["deltaEm"]:.5f}em)')
            parts.append(typ_content(pair["rightCluster"]))
        guarded_body = "".join(parts)

    guarded = f"""#set page(width: auto, height: auto, margin: {margin:.4f}pt)
#set text(
  font: "{typ_string(font_family)}",
  size: {point_size:.4f}pt,
  kerning: false,
  ligatures: {"true" if ligatures else "false"},
  features: {features},
)
{guarded_body}
"""
    (out / "typst/guarded.typ").write_text(guarded, encoding="utf-8")
PY

typst compile --font-path "$output_dir/typst/fonts" --ignore-system-fonts --format png --ppi "$dpi" \
  "$output_dir/typst/none.typ" "$output_dir/typst/none.png"
typst compile --font-path "$output_dir/typst/fonts" --ignore-system-fonts --format png --ppi "$dpi" \
  "$output_dir/typst/metric.typ" "$output_dir/typst/metric.png"
if [[ "$metric_only" == "false" ]]; then
  typst compile --font-path "$output_dir/typst/fonts" --ignore-system-fonts --format png --ppi "$dpi" \
    "$output_dir/typst/guarded.typ" "$output_dir/typst/guarded.png"
fi

echo "== Crops and overlays =="
python3 - "$output_dir" "$dpi" "$font_family" "$text" "$point_size" "$ligatures" "$metric_only" <<'PY'
from pathlib import Path
from PIL import Image
import json
import sys

root = Path(sys.argv[1])
dpi = int(sys.argv[2])
font = sys.argv[3]
text = sys.argv[4]
point_size = float(sys.argv[5])
ligatures = sys.argv[6] == "true"
metric_only = sys.argv[7] == "true"

sources = {
    "indesign_none": root / "indesign/none-1.png",
    "indesign_metric": root / "indesign/metric-1.png",
    "typst_none": root / "typst/none.png",
    "typst_metric": root / "typst/metric.png",
}
crop_paths = {
    "indesign_none": root / "crops/indesign-none-ink.png",
    "indesign_metric": root / "crops/indesign-metric-ink.png",
    "typst_none": root / "crops/typst-none-ink.png",
    "typst_metric": root / "crops/typst-metric-ink.png",
}
if not metric_only:
    sources["indesign_optical"] = root / "indesign/optical-1.png"
    sources["typst_guarded"] = root / "typst/guarded.png"
    crop_paths["indesign_optical"] = root / "crops/indesign-optical-ink.png"
    crop_paths["typst_guarded"] = root / "crops/typst-guarded-ink.png"

def dark(pixel):
    r, g, b, a = pixel
    return a > 0 and r + g + b < 660

def crop_ink(src, out):
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
    crop.save(out)
    return crop, {
        "x0": min_x, "y0": min_y, "x1": max_x, "y1": max_y,
        "width": crop.width, "height": crop.height,
    }

crops = {}
boxes = {}
for key, src in sources.items():
    crops[key], boxes[key] = crop_ink(src, crop_paths[key])

def overlay(ref_key, cand_key, output):
    ref = crops[ref_key]
    cand = crops[cand_key]
    scale = ref.height / cand.height if cand.height else 1.0
    cand_scaled = cand.resize((round(cand.width * scale), ref.height), Image.Resampling.LANCZOS)
    canvas = Image.new(
        "RGBA",
        (max(ref.width, cand_scaled.width), max(ref.height, cand_scaled.height)),
        (255, 255, 255, 255),
    )
    px = canvas.load()
    for y in range(ref.height):
        for x in range(ref.width):
            if dark(ref.getpixel((x, y))):
                px[x, y] = (0, 170, 220, 255)
    for y in range(cand_scaled.height):
        for x in range(cand_scaled.width):
            if dark(cand_scaled.getpixel((x, y))):
                px[x, y] = (
                    (10, 10, 10, 255)
                    if px[x, y][:3] != (255, 255, 255)
                    else (220, 0, 170, 255)
                )
    canvas.save(output)
    position = ink_position_metrics(ref, cand_scaled)
    return {
        "path": str(output),
        "referencePx": [ref.width, ref.height],
        "candidatePx": [cand.width, cand.height],
        "candidateScaledPx": [cand_scaled.width, cand_scaled.height],
        "heightScale": scale,
        "widthDeltaPx": ref.width - cand_scaled.width,
        "widthDeltaEm": (ref.width - cand_scaled.width) / (point_size * dpi / 72),
        "inkPositionMeanAbsPx": position["inkPositionMeanAbsPx"],
        "inkPositionMeanAbsEm": position["inkPositionMeanAbsPx"] / (point_size * dpi / 72),
        "inkPositionMaxCdfDelta": position["inkPositionMaxCdfDelta"],
        "segmentCountReference": position["segmentCountReference"],
        "segmentCountCandidate": position["segmentCountCandidate"],
        "segmentCenterMeanAbsPx": position["segmentCenterMeanAbsPx"],
        "segmentCenterMeanAbsEm": (
            None
            if position["segmentCenterMeanAbsPx"] is None
            else position["segmentCenterMeanAbsPx"] / (point_size * dpi / 72)
        ),
        "segmentCenterMaxAbsPx": position["segmentCenterMaxAbsPx"],
        "segmentCenterMaxAbsEm": (
            None
            if position["segmentCenterMaxAbsPx"] is None
            else position["segmentCenterMaxAbsPx"] / (point_size * dpi / 72)
        ),
        "segments": position["segments"],
    }

def column_ink_counts(img):
    counts = []
    for x in range(img.width):
        ink = 0
        for y in range(img.height):
            if dark(img.getpixel((x, y))):
                ink += 1
        counts.append(ink)
    return counts

def ink_segments(profile):
    segments = []
    start = None
    ink_sum = 0
    weighted_sum = 0.0
    for x, ink in enumerate(profile):
        if ink > 0 and start is None:
            start = x
            ink_sum = 0
            weighted_sum = 0.0
        if start is not None and ink > 0:
            ink_sum += ink
            weighted_sum += x * ink
        if start is not None and (ink == 0 or x == len(profile) - 1):
            end = x - 1 if ink == 0 else x
            if ink_sum > 0:
                segments.append({
                    "x0": start,
                    "x1": end,
                    "widthPx": end - start + 1,
                    "inkPx": ink_sum,
                    "centerPx": weighted_sum / ink_sum,
                })
            start = None
    return segments

def comparable_segment_errors(ref_segments, cand_segments):
    if not ref_segments or len(ref_segments) != len(cand_segments):
        return None, None
    errors = [
        cand["centerPx"] - ref["centerPx"]
        for ref, cand in zip(ref_segments, cand_segments)
    ]
    mean_abs = sum(abs(error) for error in errors) / len(errors)
    max_abs = max(abs(error) for error in errors)
    return mean_abs, max_abs

def ink_position_metrics(ref, cand):
    width = max(ref.width, cand.width)
    ref_profile = column_ink_counts(ref) + [0] * (width - ref.width)
    cand_profile = column_ink_counts(cand) + [0] * (width - cand.width)
    ref_total = sum(ref_profile)
    cand_total = sum(cand_profile)
    if ref_total == 0 or cand_total == 0:
        return {
            "inkPositionMeanAbsPx": 0.0,
            "inkPositionMaxCdfDelta": 0.0,
            "segmentCountReference": 0,
            "segmentCountCandidate": 0,
            "segmentCenterMeanAbsPx": None,
            "segmentCenterMaxAbsPx": None,
            "segments": {"reference": [], "candidate": []},
        }

    ref_cdf = 0.0
    cand_cdf = 0.0
    transport = 0.0
    max_cdf = 0.0
    for ref_ink, cand_ink in zip(ref_profile, cand_profile):
        ref_cdf += ref_ink / ref_total
        cand_cdf += cand_ink / cand_total
        diff = abs(ref_cdf - cand_cdf)
        transport += diff
        max_cdf = max(max_cdf, diff)

    ref_segments = ink_segments(ref_profile)
    cand_segments = ink_segments(cand_profile)
    segment_mean, segment_max = comparable_segment_errors(ref_segments, cand_segments)
    return {
        "inkPositionMeanAbsPx": transport,
        "inkPositionMaxCdfDelta": max_cdf,
        "segmentCountReference": len(ref_segments),
        "segmentCountCandidate": len(cand_segments),
        "segmentCenterMeanAbsPx": segment_mean,
        "segmentCenterMaxAbsPx": segment_max,
        "segments": {"reference": ref_segments, "candidate": cand_segments},
    }

none = overlay("indesign_none", "typst_none", root / "overlays/none-parity.png")
metric = overlay("indesign_metric", "typst_metric", root / "overlays/metric-parity.png")
comparisons = {
    "noneParity": none,
    "metricParity": metric,
}
if not metric_only:
    comparisons["opticalVsGuarded"] = overlay(
        "indesign_optical",
        "typst_guarded",
        root / "overlays/optical-vs-guarded.png",
    )
report = {
    "schemaVersion": 1,
    "text": text,
    "font": font,
    "pointSize": point_size,
    "ligatures": ligatures,
    "dpi": dpi,
    "crops": boxes,
    "comparisons": comparisons,
}
(root / "metrics/comparison.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps(report["comparisons"], indent=2))
PY

echo "Output: $output_dir"
