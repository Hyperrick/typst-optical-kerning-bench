#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/render-indesign-outlined-text.sh \
    --font-family "EB Garamond" \
    --text "Goldfish" \
    --kerning optical \
    --ligatures true \
    --point-size 12 \
    --output-pdf renders/indesign-outlines/goldfish.pdf

Options:
  --font-family NAME     Required InDesign font family.
  --font-style NAME      Optional style, for example Regular or Light.
  --text TEXT            Required text to render.
  --kerning MODE         optical, metrics, or none. Default: optical.
  --ligatures BOOL       true or false. Default: true.
  --point-size PT        Default: 12.
  --padding-pt PT        Default: 0.
  --output-pdf PATH      Required PDF output path.
  --output-indd PATH     Optional INDD output path.
  --output-json PATH     Optional JSON sidecar path.
USAGE
}

font_family=""
font_style=""
text=""
kerning="optical"
ligatures="true"
point_size="12"
padding_pt="0"
output_pdf=""
output_indd=""
output_json=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --font-family) font_family="$2"; shift 2 ;;
    --font-style) font_style="$2"; shift 2 ;;
    --text) text="$2"; shift 2 ;;
    --kerning) kerning="$2"; shift 2 ;;
    --ligatures) ligatures="$2"; shift 2 ;;
    --point-size) point_size="$2"; shift 2 ;;
    --padding-pt) padding_pt="$2"; shift 2 ;;
    --output-pdf) output_pdf="$2"; shift 2 ;;
    --output-indd) output_indd="$2"; shift 2 ;;
    --output-json) output_json="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "$font_family" || -z "$text" || -z "$output_pdf" ]]; then
  usage >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$(dirname "$output_pdf")"
if [[ -n "$output_indd" ]]; then mkdir -p "$(dirname "$output_indd")"; fi
if [[ -n "$output_json" ]]; then mkdir -p "$(dirname "$output_json")"; fi

config_path="$(mktemp "${TMPDIR:-/tmp}/optikern-indesign-outline.XXXXXX.json")"
wrapper_path=""
trap 'rm -f "$config_path" "$wrapper_path"' EXIT

FONT_FAMILY="$font_family" \
FONT_STYLE="$font_style" \
TEXT="$text" \
KERNING="$kerning" \
LIGATURES="$ligatures" \
POINT_SIZE="$point_size" \
PADDING_PT="$padding_pt" \
OUTPUT_PDF="$output_pdf" \
OUTPUT_INDD="$output_indd" \
OUTPUT_JSON="$output_json" \
python3 - "$config_path" <<'PY'
import json
import os
import sys

config = {
    "fontFamily": os.environ["FONT_FAMILY"],
    "fontStyle": os.environ["FONT_STYLE"],
    "text": os.environ["TEXT"],
    "kerning": os.environ["KERNING"],
    "ligatures": os.environ["LIGATURES"].lower() == "true",
    "pointSize": float(os.environ["POINT_SIZE"]),
    "paddingPt": float(os.environ["PADDING_PT"]),
    "outputPdf": os.environ["OUTPUT_PDF"],
}
if os.environ["OUTPUT_INDD"]:
    config["outputIndd"] = os.environ["OUTPUT_INDD"]
if os.environ["OUTPUT_JSON"]:
    config["outputJson"] = os.environ["OUTPUT_JSON"]

with open(sys.argv[1], "w", encoding="utf-8") as fh:
    json.dump(config, fh, indent=2)
PY

wrapper_path="$(python3 - "$config_path" "$repo_root/scripts/render-indesign-outlined-text.jsx" <<'PY'
from pathlib import Path
import json
import sys
import tempfile

config_path = Path(sys.argv[1]).resolve()
renderer_path = Path(sys.argv[2]).resolve()
wrapper = Path(tempfile.mkstemp(prefix="optikern-indesign-outline-", suffix=".jsx")[1])
wrapper.write_text(
    "var OPTIKERN_CONFIG_PATH = "
    + json.dumps(str(config_path))
    + ";\n$.evalFile(File("
    + json.dumps(str(renderer_path))
    + "));\n",
    encoding="utf-8",
)
print(wrapper)
PY
)"

osascript \
  "$repo_root/scripts/run-indesign-export.scpt" \
  "$wrapper_path"
