# Current Findings

Generated on 2026-06-21 from the pinned corpus after dynamic font calibration.

## Commands

```sh
cargo run -p optikern-cli -- bench
cargo run -p optikern-cli -- report
cargo run -p optikern-cli -- render-typst
cargo run -p optikern-cli -- contact-sheet
pdftoppm -png -r 180 reports/contact-sheet.pdf renders/contact-sheet/contact-sheet
cargo test
```

## Artifacts

- `metrics/bench.json`: full pair metrics and algorithm outputs.
- `reports/summary.pdf`: numeric report.
- `renders/typst/typst-comparison.pdf`: detailed Typst-rendered comparison.
- `reports/contact-sheet.pdf`: compact A3 visual contact sheet.
- `renders/contact-sheet/contact-sheet-1.png` and
  `renders/contact-sheet/contact-sheet-2.png`: PNG review renders.

Generated artifacts live in ignored output folders. Re-run the commands above to
reproduce them.

## Snapshot

- Successful pair results: 2667.
- Failures: 3.
- Failure pattern: `n"` in EB Garamond, Libre Baskerville, and Source Sans 3
  has too little vertical outline overlap for the current profile sampler.
- Monospaced candidate behavior: `metric-prior-hybrid` and
  `safe-fallback-only` preserve Roboto Mono and Source Code Pro because
  monospacing is detected from the font or measured glyph advances.

## Visual Review

The contact sheet was reviewed page by page.

- The sheet layout is readable and does not overflow.
- Pair rows such as `AV` and `To` show visible differences without collapsing.
- Word rows now apply every adjacent non-space pair honestly.
- Pure `profile-whitespace` and `area-balance` still over-tighten some word
  rows. This is useful evidence that naive outline profiles are not enough.
- `metric-prior-hybrid` is the strongest Typst candidate so far: it follows good
  font kerning where present, uses optical estimates only when there is a real
  disagreement or missing metric data, and preserves monospaced fonts.
- `safe-fallback-only` is the conservative candidate. It is less ambitious but
  easier to defend as a low-risk fallback for sparse kerning.

## Next Question

The next benchmark layer should compare these outputs against fresh InDesign
Optical PDFs on the same rows. The current InDesign artifacts exist, but they
should be regenerated after this calibration change before drawing conclusions
against Adobe.
