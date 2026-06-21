# Current Findings

Generated on 2026-06-22 from the pinned corpus after dynamic font calibration,
Rustybuzz glyph-run shaping, and the guarded V5 contact-zone pass.

## Commands

```sh
cargo run -p optikern-cli -- bench
cargo run -p optikern-cli -- report
cargo run -p optikern-cli -- render-indesign --run
scripts/run-guarded-review-batch.py --output renders/guarded-v5-review/eb-garamond-100pt-no-ligatures
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
- `safe-fallback-only` is the conservative candidate. It is less ambitious but
  easier to defend as a low-risk fallback for sparse kerning.

## Next Question

The next benchmark layer should separate width error from per-glyph position
error. Total word width can look close while one internal pair is wrong, and the
reverse is also possible.
