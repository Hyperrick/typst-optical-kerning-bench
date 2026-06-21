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
scripts/run-goldfish-pipeline.sh --text Goldfish --point-size 100 --ligatures false
```

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

The current leading candidate is `guarded-profile-hybrid`. The V5 pass adds
contact-zone awareness for local outline collisions, uppercase punctuation, and
round-to-overhang pairs. See
[`docs/guarded-v5-review-results.md`](docs/guarded-v5-review-results.md).

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

## Design Constraints

- Deterministic algorithms only.
- No ML or runtime raster analysis in the layout path.
- Deltas are emitted as `em` values and can be applied after shaping.
- InDesign Optical is a comparison baseline, not an absolute truth.
