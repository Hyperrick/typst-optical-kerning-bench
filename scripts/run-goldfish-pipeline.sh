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
  --font-family NAME    InDesign/Typst font family. Default: EB Garamond.
  --text TEXT           Text to render. Default: Goldfish.
  --point-size PT       Text size. Default: 100.
  --ligatures BOOL      true or false. Default: false.
  --dpi DPI             Raster DPI. Default: 300.
  --output DIR          Output directory. Default derived from text/settings.
  -h, --help            Show this help.
USAGE
}

font_id="eb-garamond"
font_family="EB Garamond"
text="Goldfish"
point_size="100"
ligatures="false"
dpi="300"
output_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --font-id) font_id="$2"; shift 2 ;;
    --font-family) font_family="$2"; shift 2 ;;
    --text) text="$2"; shift 2 ;;
    --point-size) point_size="$2"; shift 2 ;;
    --ligatures) ligatures="$2"; shift 2 ;;
    --dpi) dpi="$2"; shift 2 ;;
    --output) output_dir="$2"; shift 2 ;;
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

font_path="corpus/fonts/${font_id}.ttf"
if [[ ! -f "$font_path" ]]; then
  echo "Missing font file: $font_path" >&2
  exit 1
fi

mkdir -p \
  "$output_dir/indesign/Document fonts" \
  "$output_dir/typst" \
  "$output_dir/crops" \
  "$output_dir/overlays" \
  "$output_dir/metrics"
cp "$font_path" "$output_dir/indesign/Document fonts/${font_id}.ttf"

echo "== Deltas =="
cargo run -q -p optikern-cli -- sample-deltas \
  --font-id "$font_id" \
  --text "$text" \
  --ligatures="$ligatures" \
  > "$output_dir/metrics/guarded-deltas.json"

echo "== InDesign =="
scripts/render-indesign-outlined-text.sh \
  --font-family "$font_family" \
  --text "$text" \
  --kerning metrics \
  --ligatures "$ligatures" \
  --point-size "$point_size" \
  --output-pdf "$repo_root/$output_dir/indesign/metric.tmp.pdf" \
  --output-indd "$repo_root/$output_dir/indesign/metric.indd" \
  --output-json "$repo_root/$output_dir/metrics/indesign-metric.json"

scripts/render-indesign-outlined-text.sh \
  --font-family "$font_family" \
  --text "$text" \
  --kerning optical \
  --ligatures "$ligatures" \
  --point-size "$point_size" \
  --output-pdf "$repo_root/$output_dir/indesign/optical.tmp.pdf" \
  --output-indd "$repo_root/$output_dir/indesign/optical.indd" \
  --output-json "$repo_root/$output_dir/metrics/indesign-optical.json"

pdftoppm -png -r "$dpi" "$output_dir/indesign/metric.tmp.pdf" "$output_dir/indesign/metric"
pdftoppm -png -r "$dpi" "$output_dir/indesign/optical.tmp.pdf" "$output_dir/indesign/optical"
rm -f "$output_dir/indesign/"*.tmp.pdf
rm -f "$output_dir/indesign/"*.idlk

echo "== Typst =="
python3 - "$output_dir" "$font_family" "$text" "$point_size" "$ligatures" <<'PY'
from pathlib import Path
import json
import sys

out = Path(sys.argv[1])
font_family = sys.argv[2]
text = sys.argv[3]
point_size = float(sys.argv[4])
ligatures = sys.argv[5] == "true"
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

features = "(liga: 1, clig: 1)" if ligatures else "(liga: 0, clig: 0)"
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

typst compile --font-path corpus/fonts --ignore-system-fonts --format png --ppi "$dpi" \
  "$output_dir/typst/metric.typ" "$output_dir/typst/metric.png"
typst compile --font-path corpus/fonts --ignore-system-fonts --format png --ppi "$dpi" \
  "$output_dir/typst/guarded.typ" "$output_dir/typst/guarded.png"

echo "== Crops and overlays =="
python3 - "$output_dir" "$dpi" "$font_family" "$text" "$point_size" "$ligatures" <<'PY'
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

sources = {
    "indesign_metric": root / "indesign/metric-1.png",
    "indesign_optical": root / "indesign/optical-1.png",
    "typst_metric": root / "typst/metric.png",
    "typst_guarded": root / "typst/guarded.png",
}
crop_paths = {
    "indesign_metric": root / "crops/indesign-metric-ink.png",
    "indesign_optical": root / "crops/indesign-optical-ink.png",
    "typst_metric": root / "crops/typst-metric-ink.png",
    "typst_guarded": root / "crops/typst-guarded-ink.png",
}

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
    return {
        "path": str(output),
        "referencePx": [ref.width, ref.height],
        "candidatePx": [cand.width, cand.height],
        "candidateScaledPx": [cand_scaled.width, cand_scaled.height],
        "heightScale": scale,
        "widthDeltaPx": ref.width - cand_scaled.width,
        "widthDeltaEm": (ref.width - cand_scaled.width) / (point_size * dpi / 72),
    }

metric = overlay("indesign_metric", "typst_metric", root / "overlays/metric-parity.png")
optical = overlay("indesign_optical", "typst_guarded", root / "overlays/optical-vs-guarded.png")
report = {
    "schemaVersion": 1,
    "text": text,
    "font": font,
    "pointSize": point_size,
    "ligatures": ligatures,
    "dpi": dpi,
    "crops": boxes,
    "comparisons": {
        "metricParity": metric,
        "opticalVsGuarded": optical,
    },
}
(root / "metrics/comparison.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps(report["comparisons"], indent=2))
PY

echo "Output: $output_dir"
