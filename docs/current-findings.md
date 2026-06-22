# Current Findings

Generated on 2026-06-22 from the pinned corpus after dynamic font calibration,
Rustybuzz glyph-run shaping, the guarded V5 contact-zone pass, V6 measurement
upgrades, the V8 sans run-context pass, script-run V13 tuning, the first
ligature-parity preparation pass, the V14 digit-run context pass, and the V15
compact-sans / long-caps pass. V21 adds shaped sans-lowercase run correction
for ligature-capable words. V22 adds wide-serif ligature-run tuning and short
connected-script lowercase tuning. V23 softens long fully connected script
ligature runs and adds end-of-suite InDesign cleanup for crash-recovery modals.
V24 relaxes over-compaction in short wide-serif `fi` ligature words. V25
improves the largest remaining no-ligature outlier, EB Garamond
`ToTaL`, while leaving the V24 ligature suite unchanged.

Current V25 summary:

- No-ligature suite: 30/30 measured, mean score `0.0177em -> 0.0170em`,
  worst score `0.0384em -> 0.0304em`.
- Changed no-ligature cases: only EB Garamond `ToTaL`
  (`0.0384em -> 0.0168em`, width `-0.0384em -> -0.0168em`,
  ink `0.0294em -> 0.0159em`).
- Ligature suite: 31/31 measured, mean score `0.0123em`, worst score
  `0.0240em`, zero changed cases from V24.

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
.venv-fonttools/bin/python scripts/build-parity-fonts.py \
  --variant ligatures \
  --spec-output renders/font-sandbox/goldfish-ligature-fonts.json
scripts/run-goldfish-parity.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --baseline-output baselines/goldfish-parity-sandbox-v1.json
scripts/run-metric-parity-suite.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --baseline-output baselines/metric-parity-suite-v1.json
scripts/run-optical-comparison-suite.py \
  --suite full \
  --metric-baseline baselines/metric-parity-suite-v1.json \
  --baseline-output baselines/optical-comparison-suite-v7.json
scripts/run-optical-comparison-suite.py \
  --suite fast \
  --metric-baseline baselines/metric-parity-suite-v1.json \
  --output renders/optical-comparison-suite/no-ligatures-100pt-v8-fast \
  --baseline-output baselines/optical-comparison-suite-v8-fast.json
scripts/run-metric-parity-suite.py \
  --sample-matrix corpus/samples/metric-cross-font-suite.json \
  --output renders/metric-parity-suite/no-ligatures-100pt-v8-cross-font \
  --baseline-output baselines/metric-parity-suite-v8-cross-font.json
scripts/run-optical-comparison-suite.py \
  --suite cross-font \
  --metric-baseline baselines/metric-parity-suite-v8-cross-font.json \
  --output renders/optical-comparison-suite/no-ligatures-100pt-v8-cross-font \
  --baseline-output baselines/optical-comparison-suite-v8-cross-font.json
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-number-focus-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-cross-font \
  --output renders/optical-comparison-suite/no-ligatures-100pt-number-focus-v14 \
  --baseline-output baselines/optical-comparison-suite-number-focus-v14.json
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-v15-focus-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-cross-font \
  --output renders/optical-comparison-suite/no-ligatures-100pt-v15-focus \
  --baseline-output baselines/optical-comparison-suite-v15-focus.json
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-cross-font \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v15 \
  --baseline-output baselines/optical-comparison-suite-five-font-v15.json
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-ligature-valid-suite.json \
  --metric-baseline baselines/metric-ligature-suite-v1.json \
  --reuse-indesign-from renders/optical-comparison-suite/ligatures-100pt-v20-valid-complete \
  --output renders/optical-comparison-suite/ligatures-100pt-v21-valid \
  --baseline-output baselines/optical-ligature-suite-v21-valid.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-v15 \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v21 \
  --baseline-output baselines/optical-comparison-suite-five-font-v21.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-ligature-valid-suite.json \
  --metric-baseline baselines/metric-ligature-suite-v1.json \
  --reuse-indesign-from renders/optical-comparison-suite/ligatures-100pt-v20-valid-complete \
  --output renders/optical-comparison-suite/ligatures-100pt-v22 \
  --baseline-output baselines/optical-ligature-suite-v22.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-v15 \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v22 \
  --baseline-output baselines/optical-comparison-suite-five-font-v22.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-ligature-valid-suite.json \
  --metric-baseline baselines/metric-ligature-suite-v1.json \
  --reuse-indesign-from renders/optical-comparison-suite/ligatures-100pt-v20-valid-complete \
  --output renders/optical-comparison-suite/ligatures-100pt-v23 \
  --baseline-output baselines/optical-ligature-suite-v23.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-v15 \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v23 \
  --baseline-output baselines/optical-comparison-suite-five-font-v23.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-ligature-valid-suite.json \
  --metric-baseline baselines/metric-ligature-suite-v1.json \
  --reuse-indesign-from renders/optical-comparison-suite/ligatures-100pt-v20-valid-complete \
  --output renders/optical-comparison-suite/ligatures-100pt-v24 \
  --baseline-output baselines/optical-ligature-suite-v24.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-v15 \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24 \
  --baseline-output baselines/optical-comparison-suite-five-font-v24.json \
  --retries 1 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-ligature-valid-suite.json \
  --metric-baseline baselines/metric-ligature-suite-v1.json \
  --reuse-indesign-from renders/optical-comparison-suite/ligatures-100pt-v24 \
  --output renders/optical-comparison-suite/ligatures-100pt-v25 \
  --baseline-output baselines/optical-ligature-suite-v25.json \
  --retries 0 \
  --preflight-timeout 45
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24 \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v25 \
  --baseline-output baselines/optical-comparison-suite-five-font-v25.json \
  --retries 0 \
  --preflight-timeout 45
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
- `renders/metric-parity-suite/no-ligatures-100pt/summary.json`:
  strict multi-sample metric-only parity gate using the same sandbox fonts.
- `renders/optical-comparison-suite/no-ligatures-100pt/summary.json`:
  InDesign Optical vs Typst Guarded Optical for samples that passed metric
  parity.
- `renders/optical-comparison-suite/no-ligatures-100pt-v8-fast/summary.json`:
  12-case fast optical suite for algorithm iteration.
- `renders/metric-parity-suite/no-ligatures-100pt-v8-cross-font/summary.json`:
  18-case metric-valid cross-font matrix.
- `renders/optical-comparison-suite/no-ligatures-100pt-v8-cross-font/summary.json`:
  18-case optical cross-font matrix for visual review.
- `renders/optical-comparison-suite/no-ligatures-100pt-number-focus-v14/summary.json`:
  5-case digit-run focus matrix across the current five-font spread.
- `renders/optical-comparison-suite/no-ligatures-100pt-v15-focus/summary.json`:
  20-case focus matrix for compact sans, mixed-case words, and long cap runs.
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v15/summary.json`:
  30-case no-ligature optical matrix after compact sans and long-cap tuning.
- `renders/optical-comparison-suite/ligatures-100pt-v21-valid/summary.json`:
  31-case ligature-capable optical matrix after shaped sans-lowercase run
  correction.
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v21/summary.json`:
  30-case no-ligature regression check. It matches the V15 score set.
- `renders/optical-comparison-suite/ligatures-100pt-v22/summary.json`:
  31-case ligature-capable optical matrix after wide-serif and short
  connected-script run tuning.
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v22/summary.json`:
  30-case no-ligature regression check after V22. It is unchanged from V21.
- `renders/optical-comparison-suite/ligatures-100pt-v23/summary.json`:
  31-case ligature-capable optical matrix after long connected-script
  ligature-run softening.
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v23/summary.json`:
  30-case no-ligature regression check after V23. It is unchanged from V22.
- `renders/optical-comparison-suite/ligatures-100pt-v24/summary.json`:
  31-case ligature-capable optical matrix after short wide-serif `fi` word
  compaction relief.
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24/summary.json`:
  30-case no-ligature regression check after V24. It is unchanged from V23.
- `renders/optical-comparison-suite/ligatures-100pt-v25/summary.json`:
  31-case ligature-capable regression check after V25. It is unchanged from V24.
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v25/summary.json`:
  30-case no-ligature regression check after V25. Only EB Garamond `ToTaL`
  changes from V24.
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
- The broader metric-only suite passes all 30 current samples across EB
  Garamond, Libre Baskerville, and Inter. The largest remaining metric deltas
  are Inter `WAYFINDER` and `LANDMARK` at `+0.0192em`, still inside the
  `0.02em` width gate. The worst ink-position delta is Inter `LANDMARK` at
  `0.0143em`, inside the `0.02em` ink gate.
- V7 improves the 30-case optical suite mean score from `0.0428em` to
  `0.0249em` and worst-case score from `0.1416em` to `0.0936em`, with no
  measured score regressions against V1. The remaining largest failures are
  Inter `OpenType`, Libre `AVATAR`, Libre `Goldfish`, EB `AVATAR`, and EB
  `ToTaL`.
- The fast optical suite selects 12 cases from JSON: seven known failure
  targets and five controls. The V8 run completed 12/12 in the background
  InDesign path. Libre `WAVY` hit one transient InDesign `-609`; rerunning only
  that case repaired the suite.
- The V8 cross-font metric suite passes all 18 same-sample matrix cases across
  EB Garamond, Libre Baskerville, and Inter. The only transient InDesign
  `-609` failure was EB `ToTaL`; rerunning just that case produced a valid
  metric gate.
- The V8 algorithm adds run-level context only for sans-like spacing profiles.
  It uses computed font spacing, pair classes, and metric-kerning density; it
  does not branch on font names or sample strings. This specifically tightens
  sans uppercase and mixed-case display words while leaving serif controls
  mostly unchanged.
- A focused aperture guard fixes the visible Libre Baskerville `G|o` collision
  in `Goldfish`. The pair delta is clamped from `-0.0958em` to `-0.0550em`
  because the local nearest-contour distance was already critical while the
  profile mean was inflated by the `G` aperture. The `Goldfish` score dropped
  from `0.0413em` to `0.0149em`.
- The V8 fast suite reduced the formerly dominant Inter failures: Inter
  `OpenType` is now `0.0120em` width / `0.0111em` ink, Inter `ToTaL` is
  `-0.0192em` width / `0.0111em` ink, and Inter `AVATAR` is `-0.0360em` width /
  `0.0201em` ink.
- The updated cross-font sheet still ranks Libre `AVATAR` (`0.0552em`), Libre
  `10.000` (`0.0456em`), EB `AVATAR` (`0.0384em`), EB `ToTaL` (`0.0384em`),
  and Inter `AVATAR` (`0.0360em`) highest numerically, but the visible hard
  collision in the current matrix was Libre `G|o`.
- The next algorithm pass should treat the remaining numeric and display-cap
  differences as careful visual tuning, not as broad failures. The main guard
  to preserve is the new local aperture/collision behavior for metricless
  upper-lower pairs.
- The V13 script-uppercase pass opens only near-touching, metricless uppercase
  gaps in long connected script runs. This fixed the Pacifico `AVATAR` split
  against InDesign Optical without changing the existing script-focus controls.
- The V14 digit-run context pass treats long metricless digit/punctuation runs
  separately from ordinary letter pairs. It is triggered from pair classes,
  measured metric deltas, target gap, and loose digit gaps, not font names or
  sample strings.
- V14 reduces the five-font 30-case mean optical score from `0.0276em` to
  `0.0240em` with no measured regression above `0.001em`. The targeted numeric
  fixes are Comic Neue `10.000` (`0.0984em` to `0.0144em`) and Libre
  Baskerville `10.000` (`0.0456em` to `0.0198em`).
- V15 adds compact-sans and long-cap run handling. Compact sans reduces
  over-tightening in simple lowercase/mixed words, but restores lower bridges in
  mixed-case words like `OpenType`. Long-cap handling loosens long serif cap
  runs and tightens long sans cap runs. The five-font 30-case mean score drops
  from `0.0240em` to `0.0177em`; worst case drops from `0.0648em` to
  `0.0384em`, with no measured regression above `0.001em`.
- V21 adds shaped sans-lowercase run correction for ligature-capable words. The
  ligature suite mean score drops from V20 `0.0249em` to `0.0168em`; worst case
  drops from `0.0888em` to `0.0648em`. The biggest fixes are Comic Neue
  `office` (`0.0864em` to `0.0081em`), Comic Neue `affinity` (`0.0888em` to
  `0.0152em`), Inter `office` (`0.0648em` to `0.0142em`), Inter `affinity`
  (`0.0504em` to `0.0171em`), and Inter `efficient` (`0.0296em` to
  `0.0147em`).
- The V21 no-ligature regression suite is unchanged from V15: mean score
  `0.0177em`, worst case `0.0384em`, and no changed cases. This is important
  because the ligature-path correction did not destabilize the established
  no-ligature behavior.
- V22 adds wide-serif ligature-run tuning and short connected-script lowercase
  tuning. The ligature suite mean score drops from V21 `0.0168em` to
  `0.0134em`; worst case drops from `0.0648em` to `0.0360em`. The targeted
  fixes are Libre Baskerville `efficient` (`0.0648em` to `0.0120em`), Libre
  Baskerville `fjord` (`0.0360em` to `0.0131em`), and Lobster `fjord`
  (`0.0360em` to `0.0074em`).
- The V22 no-ligature regression suite is unchanged from V21: mean score
  `0.0177em`, worst case `0.0384em`, and zero changed cases.
- V23 adds a narrow cap for long, fully connected script ligature runs when all
  letter pairs are connected, no metric tightening exists, and the outline
  profile does not request an opening. The ligature suite mean score drops from
  V22 `0.0134em` to `0.0127em`; worst case drops from `0.0360em` to
  `0.0288em`. The only changed measured case is Lobster `efficient`
  (`0.0360em` to `0.0144em`, width `-0.0360em` to `-0.0024em`).
- The V23 no-ligature regression suite is unchanged from V22: mean score
  `0.0177em`, worst case `0.0384em`, and zero changed cases.
- V24 relaxes short wide-serif lowercase words with a two-letter ligature
  cluster when a generic compaction target would tighten a bridge whose robust
  outline gap is already compact. The ligature suite mean score drops from V23
  `0.0127em` to `0.0123em`; worst case drops from `0.0288em` to `0.0240em`.
  The only changed measured case is Libre Baskerville `final` (`0.0288em` to
  `0.0168em`, width `+0.0288em` to `+0.0168em`).
- The V24 no-ligature regression suite is unchanged from V23: mean score
  `0.0177em`, worst case `0.0384em`, and zero changed cases.
- V25 keeps the V24 ligature suite unchanged and improves the no-ligature suite
  by tightening a dynamic serif mixed-case gap class. Only EB Garamond `ToTaL`
  changes: score `0.0384em -> 0.0168em`, width
  `-0.0384em -> -0.0168em`, ink `0.0294em -> 0.0159em`. The no-ligature mean
  score moves from `0.0177em` to `0.0170em`, and the worst score moves from
  `0.0384em` to `0.0304em`.
- `safe-fallback-only` is the conservative candidate. It is less ambitious but
  easier to defend as a low-risk fallback for sparse kerning.
- Ligature-sensitive benchmarking now has its own sandbox font variant. Unlike
  the no-ligature sandbox, it retains standard ligature data, legacy glyph
  names, and presentation-form cmap entries. Rustybuzz confirms `Goldfish` in
  EB Garamond Liga shapes as `G|o`, `o|l`, `l|d`, `d|fi`, `fi|s`, and `s|h`.
- InDesign crash recovery can leave a blocking "restore documents" startup
  modal. The suite now starts automation by killing InDesign, clearing recovery
  and scripting state, and then running the preflight. While InDesign starts,
  a best-effort watcher clicks known negative recovery buttons such as `No`,
  `Cancel`, and `Nicht wiederherstellen`, because the modal can appear after
  launch but before ExtendScript is scriptable. Individual case renders also
  have a timeout, so a modal or non-scriptable state logs as a failed attempt,
  resets InDesign, and retries instead of hanging indefinitely. Metric and
  optical suite runners also clean up InDesign and recovery state in a
  `finally` block so a later run is not blocked by stale crash recovery.

## Next Question

The next benchmark layer can either focus on the remaining ligature outliers
(`Comic Neue word spacing`, Inter/Pacifico small ink-position differences) or
continue turning the current evidence into the Typst-facing article/paper while keeping both V25
suites as the current reproducible baseline.
