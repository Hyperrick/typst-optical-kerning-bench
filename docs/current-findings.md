# Current Findings

Generated on 2026-06-22 from the pinned corpus after dynamic font calibration,
Rustybuzz glyph-run shaping, the guarded V5 contact-zone pass, and V6
measurement upgrades.

## Commands

```sh
cargo run -p optikern-cli -- bench
cargo run -p optikern-cli -- report
cargo run -p optikern-cli -- render-indesign --run
scripts/run-guarded-review-batch.py --output renders/guarded-v5-review/eb-garamond-100pt-no-ligatures
scripts/run-guarded-review-batch.py --output renders/guarded-v6-review/eb-garamond-100pt-no-ligatures
scripts/run-glyph-shape-parity.py --baseline-output baselines/glyph-shape-parity-v1.json
scripts/run-goldfish-parity.py --baseline-output baselines/goldfish-parity-v1.json
.venv-fonttools/bin/python scripts/build-parity-fonts.py
scripts/run-goldfish-parity.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --baseline-output baselines/goldfish-parity-sandbox-v1.json
cargo test
```

## Artifacts

- `metrics/bench.json`: full glyph-pair metrics and algorithm outputs.
- `reports/summary.pdf`: numeric report.
- `renders/indesign/indesign-comparison.pdf`: InDesign Metrics vs Optical on
  the configured per-font pair/word selection.
- `renders/indesign/indesign-metrics.pdf` and `indesign-optical.pdf`: separate
  baseline PDFs with matching JSON sidecars.
- `renders/guarded-v5-review/eb-garamond-100pt-no-ligatures/contact-sheet.png`:
  cropped InDesign Optical, Typst Guarded, and overlay rows for the V5 review.
- `renders/guarded-v6-review/*/summary.json`: V6 reviews with width,
  ink-position, segment-center, and metric-parity measurements.
- `renders/goldfish-parity/goldfish-100pt-no-ligatures/summary.json`:
  focused single-word metric parity gate before optical tuning.
- `renders/goldfish-parity/goldfish-100pt-no-ligatures-sandbox/summary.json`:
  strict no-ligature metric parity gate using isolated static benchmark fonts.
- `renders/glyph-shape-parity/goldfish-glyphs-100pt-no-ligatures/summary.json`:
  individual glyph shape parity before word or kerning comparison.

Generated artifacts live in ignored output folders. Re-run the commands above to
reproduce them.

## Snapshot

- Successful glyph-pair results: 3327.
- Failures: 3.
- Failure pattern: `n"` in EB Garamond, Libre Baskerville, and Source Sans 3
  has too little vertical outline overlap for the current profile sampler.
- Ligature proof cases are present in `metrics/bench.json`, including
  `d|fi`, `fi|s`, `o|ffi`, `ffi|c`, `fl|u`, and `fi|l` where supported by the
  font.
- Monospaced candidate behavior: `metric-prior-hybrid` and
  `safe-fallback-only` preserve Roboto Mono and Source Code Pro because
  monospacing is detected from the font or measured glyph advances.

## Visual Review

The InDesign comparison PDF was regenerated and spot-checked as rendered PNGs.

- Page 1 contains EB Garamond with `Goldfish`, `office`, `offline`, and
  `affinity`.
- Page 2 contains Libre Baskerville with `fluent` and `file`.
- The previous page-2 layout collapse is not present in the regenerated
  comparison PDF.
- Pure `profile-whitespace` and `area-balance` still over-tighten some word
  rows. This is useful evidence that naive outline profiles are not enough.
- `metric-prior-hybrid` remains an important baseline: it follows good font
  kerning where present, uses optical estimates only when there is a real
  disagreement or missing metric data, and preserves monospaced fonts.
- `guarded-profile-hybrid` is the leading candidate. V5 adds contact-zone
  handling for local outline collisions, uppercase punctuation, and
  round-to-overhang pairs without hard-coding words or glyph names.
- In the 32-sample EB Garamond V5 review, average absolute width error against
  InDesign Optical improved from `0.0235em` to `0.0184em`.
- `T.`, `P.`, `WAYFINDER`, `ToTaL`, and `WAVY` improved in the V5 review, with
  no measured regression in that batch.
- V6 adds glyph-position-style diagnostics through rendered PNG ink
  distributions. EB Garamond remains essentially neutral by width
  (`0.0184em` to `0.0185em`) while exposing internal position error.
- Damped class-local calibration improved `AVATAR` but regressed `VA` and
  `WAVY` slightly; the stronger class-local replacement was rejected.
- Multi-font review shows a baseline problem: Inter has large Typst Metric vs
  InDesign Metrics mismatch (`0.1672em` average), so it is not yet valid for
  optical algorithm tuning.
- The focused Goldfish parity gate treats Metric-vs-Metric parity as a hard
  prerequisite. Fonts above `0.02em` absolute metric width delta are labeled
  `baseline-mismatch` and must be debugged before their optical deltas are used
  as algorithm evidence.
- Glyph shape parity is now the first gate. If individual glyph overlays differ
  without kerning or ligatures, the benchmark must resolve font instance,
  variable-axis, or rendering parity before interpreting word-level kerning
  errors.
- The first `Goldfish` glyph-shape run succeeded for 24/24 glyphs. EB Garamond
  and Libre Baskerville have strong raw overlay parity (`0.978` and `0.974`
  average overlap). Inter does not (`0.801` average overlap, worst glyph `f` at
  `0.653`), so Inter is blocked at the glyph-shape stage.
- Because Libre Baskerville passes glyph-shape parity but fails `Goldfish`
  Metric-vs-Metric width parity, the next Libre debug layer should compare
  unkerned word advance/spacing rather than glyph outlines.
- The strict no-ligature sandbox resolves the immediate `Goldfish`
  Metric-vs-Metric baseline problem for EB Garamond, Libre Baskerville, and
  Inter. The sandbox freezes variable axes, uses unique family names, disables
  standard ligature features, strips legacy glyph names, and removes
  presentation-form ligature cmap entries before rendering in both engines.
- This confirmed two different failure causes: Inter needed a fixed static
  instance (`wght=400`, `opsz=14`), while Libre Baskerville needed the legacy
  `fi` ligature path removed before a no-ligature comparison was meaningful.
- `safe-fallback-only` is the conservative candidate. It is less ambitious but
  easier to defend as a low-risk fallback for sparse kerning.

## Next Question

The next benchmark layer should extend the strict sandbox beyond `Goldfish`.
Only words that pass Metric-vs-Metric parity should be used as evidence for
InDesign Optical vs Typst Guarded Optical tuning.
