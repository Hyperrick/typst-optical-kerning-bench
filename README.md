# Typst Optical Kerning Bench

Rust-first lab for developing a deterministic optical kerning algorithm that
could plausibly fit Typst.

The repo focuses on one question: can an outline-based, cacheable algorithm get
close to InDesign Optical while preserving good font metric kerning and Typst's
performance expectations?

The current development loop is:

1. shape text with Rustybuzz,
2. evaluate adjacent shaped glyph pairs from font outlines,
3. emit deterministic `em` deltas,
4. render Typst Metric, Typst Guarded Optical, and InDesign Optical,
5. compare cropped PNGs and overlays.

## Quick Start

```sh
cargo run -p optikern-cli -- fetch-fonts
cargo run -p optikern-cli -- bench
cargo run -p optikern-cli -- sample-deltas --font-id eb-garamond --text ToTaL --ligatures=false
scripts/run-guarded-review-batch.py --output renders/guarded-v5-review/eb-garamond-100pt-no-ligatures
```

Outputs are written to `metrics/`, `renders/`, and `reports/`.

Useful focused commands:

```sh
cargo run -p optikern-cli -- contact-sheet
cargo run -p optikern-cli -- triad-compare --run-indesign
scripts/run-glyph-shape-parity.py --baseline-output baselines/glyph-shape-parity-v1.json
scripts/run-goldfish-pipeline.sh --text Goldfish --point-size 100 --ligatures false
scripts/run-goldfish-parity.py --baseline-output baselines/goldfish-parity-v1.json
.venv-fonttools/bin/python scripts/build-parity-fonts.py
scripts/run-goldfish-parity.py --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json
scripts/run-metric-parity-suite.py
scripts/run-optical-comparison-suite.py
```

The suite performs a small InDesign automation preflight before writing metric
or optical baselines. If InDesign is stuck behind a modal dialog or is not
scriptable, the run stops before any render-error baseline is written.

## InDesign Baselines

Generate the InDesign ExtendScript and sidecar data:

```sh
cargo run -p optikern-cli -- render-indesign
```

Then run it from InDesign's Scripts panel or execute:

```sh
osascript scripts/run-indesign-export.scpt "$(pwd)/renders/indesign/export-baselines.jsx"
```

The generated document uses fixed A4 pages, black text on white background,
tracking `0`, horizontal/vertical scale `100`, no hyphenation, and exports both
`$ID/Metrics` and `$ID/Optical` PDFs. It also exports
`indesign-comparison.pdf`, a side-by-side visual sheet with Metrics and Optical
columns for pairs and real words.

See [`docs/indesign-baseline.md`](docs/indesign-baseline.md) for the exact
document construction rules.

## Algorithms

Implemented candidate algorithms:

- `nearest-contour-distance`
- `profile-whitespace`
- `area-balance`
- `metric-prior-hybrid`
- `guarded-profile-hybrid`
- `safe-fallback-only`

See [`docs/algorithms.md`](docs/algorithms.md) for the current heuristics and
the constraints that make them plausible for a future Typst implementation.
See [`docs/research-alignment.md`](docs/research-alignment.md) for the external
sources that shaped the current benchmark rules.

The current leading candidate is `guarded-profile-hybrid`. It combines
metric-prior kerning, contact-zone awareness, and V8 run-context tuning for
sans-like display words. See
[`docs/algorithms.md`](docs/algorithms.md) and
[`docs/optical-comparison-suite.md`](docs/optical-comparison-suite.md).

## Contact Sheet

Generate a compact A3 PDF showing the baselines and all algorithms side by side:

```sh
cargo run -p optikern-cli -- contact-sheet
```

This writes `reports/contact-sheet.typ` and, when Typst is installed,
`reports/contact-sheet.pdf`. The sheet is a compact Typst-rendered smoke review
for simple pair and word spacing. Ligature-capable words must be evaluated after
shaping: a substituted ligature glyph is spaced against its neighbors while its
internal letters are not kerned separately.

## Goldfish Parity Gate

Before the word-level gate, check individual glyph shapes:

```sh
scripts/run-glyph-shape-parity.py \
  --fonts eb-garamond,libre-baskerville,inter \
  --glyphs G,o,l,d,f,i,s,h \
  --point-size 100 \
  --output renders/glyph-shape-parity/goldfish-glyphs-100pt-no-ligatures \
  --baseline-output baselines/glyph-shape-parity-v1.json
```

Before tuning against a new font, run the single-word `Goldfish` gate:

```sh
scripts/run-goldfish-parity.py \
  --fonts eb-garamond,libre-baskerville,inter \
  --text Goldfish \
  --point-size 100 \
  --ligatures false \
  --metric-threshold-em 0.02 \
  --output renders/goldfish-parity/goldfish-100pt-no-ligatures \
  --baseline-output baselines/goldfish-parity-v1.json
```

The gate compares InDesign Metrics against Typst Metrics first. A font only
counts as valid optical tuning evidence when metric width parity is within
`0.02em`; otherwise the result is treated as a font/rendering baseline mismatch.
For the strict no-ligature benchmark, first generate isolated static fonts with
unique family names:

```sh
python3 -m venv .venv-fonttools
.venv-fonttools/bin/pip install -r requirements-fonttools.txt
.venv-fonttools/bin/python scripts/build-parity-fonts.py
scripts/run-goldfish-parity.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --text Goldfish \
  --point-size 100 \
  --ligatures false \
  --metric-threshold-em 0.02 \
  --output renders/goldfish-parity/goldfish-100pt-no-ligatures-sandbox \
  --baseline-output baselines/goldfish-parity-sandbox-v1.json
```

The generated parity fonts are local render artifacts. They freeze variable
axes, use unique family names, remove standard ligature GSUB features, strip
legacy glyph names, and remove Unicode presentation-form ligature cmap entries
so both engines shape the same no-ligature text.
See [`docs/glyph-shape-parity.md`](docs/glyph-shape-parity.md) and
[`docs/goldfish-parity-gate.md`](docs/goldfish-parity-gate.md).

For ligature-sensitive checks, build the sibling sandbox with standard ligature
data retained:

```sh
.venv-fonttools/bin/python scripts/build-parity-fonts.py \
  --variant ligatures \
  --spec-output renders/font-sandbox/goldfish-ligature-fonts.json
```

The ligature variant keeps `liga`/ligature glyph metadata so words such as
`Goldfish` can be evaluated after shaping as `d|fi` and `fi|s` instead of
pretending `f|i` is a real pair.

After `Goldfish` passes, run the broader metric-only suite:

```sh
scripts/run-metric-parity-suite.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --point-size 100 \
  --ligatures false \
  --metric-threshold-em 0.02 \
  --ink-threshold-em 0.02 \
  --output renders/metric-parity-suite/no-ligatures-100pt \
  --baseline-output baselines/metric-parity-suite-v1.json
```

See [`docs/metric-parity-suite.md`](docs/metric-parity-suite.md).

Once metric parity passes, run the optical comparison suite:

```sh
scripts/run-optical-comparison-suite.py --suite fast
scripts/run-optical-comparison-suite.py --suite cross-font
scripts/run-optical-comparison-suite.py --suite full
```

See [`docs/optical-comparison-suite.md`](docs/optical-comparison-suite.md).

## Design Constraints

- Deterministic algorithms only.
- No ML or runtime raster analysis in the layout path.
- Deltas are emitted as `em` values and can be applied after shaping.
- InDesign Optical is a comparison baseline, not an absolute truth.
