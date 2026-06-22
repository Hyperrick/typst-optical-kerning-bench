# Optical Kerning Evaluation For Typst

Status: working draft for maintainers and typography reviewers.

## Abstract

This repository evaluates whether a deterministic, outline-based optical
kerning algorithm could fit Typst's compiler and publishing goals. It does not
argue that optical kerning should replace metric kerning. Instead, it asks which
optical strategy can get close to an industry desktop-publishing reference while
remaining explainable, cacheable, and fast enough for a document compiler.

The current candidate, `guarded-profile-hybrid`, combines existing font metric
kerning, outline-derived whitespace profiles, local collision guards, and
word-level run context. It is evaluated against scripted InDesign Optical
exports and Typst-rendered candidates after first proving that InDesign Metrics
and Typst Metrics render the same shaped text closely enough to compare.

## Motivation

Typst already supports metric kerning through font-provided OpenType data. That
is necessary, but it does not cover the full print-publishing workflow. In
branding, editorial, packaging, and display typography, designers often expect
optical kerning when fonts lack good pair data, when display sizes expose
spacing defects, or when mixed-case/ligature/script words need visual spacing
rather than only font-table spacing.

The project therefore treats optical kerning as a publishing feature with three
constraints:

- It must be deterministic.
- It must be understandable enough to review and maintain.
- It must not put raster analysis, machine learning, or expensive per-layout
  image operations into Typst's layout path.

## Why InDesign Is Used As A Baseline

InDesign Optical is used because it is the common professional reference point
for print-oriented optical kerning. It is not treated as mathematical ground
truth. The benchmark uses it as an external, familiar comparator: if a proposed
Typst-side approach consistently differs from InDesign, that difference must be
visible, measurable, and explainable.

This matters for trust. Designers and publishers already know what InDesign
Optical looks like. Typst maintainers do not need to clone Adobe behavior, but a
proposal becomes much easier to evaluate when examples show:

- InDesign Metrics,
- InDesign Optical,
- Typst Metrics,
- Typst Guarded Optical,
- and an overlay with numeric differences.

## Evaluation Pipeline

The suite intentionally compares rendered output, not only internal pair values.

1. Static benchmark fonts are built from pinned font files. Variable fonts are
   frozen where needed; unique family names avoid system-font substitution.
2. Text is shaped before kerning analysis. This is essential for ligatures:
   `fi` may be one glyph cluster in one font and two glyphs in another.
3. Metric parity is checked first. A font/sample only becomes optical evidence
   if InDesign Metrics and Typst Metrics agree within the configured gate.
4. InDesign exports are created by script, converted to outlines, fitted to the
   visible bounds, and rasterized as controlled PNGs.
5. Typst renders metric and guarded-optical candidates with the same font,
   point size, ligature setting, and text.
6. Crops and overlays are measured using black ink pixels.

Current primary metrics:

- `widthDeltaEm`: ink bounding-box width difference in em.
- `inkPositionMeanAbsEm`: average horizontal ink distribution difference.
- `segmentCenterMeanAbsEm`: center difference of separated glyph/cluster
  segments where segmentation is reliable.
- `scoreEm`: max of the relevant visual error metrics for ranking failures.

## Font And Sample Coverage

The current suites cover serif, sans, script, and comic-style fonts:

- EB Garamond
- Libre Baskerville
- Inter
- Pacifico
- Lobster
- Comic Neue

The no-ligature suite stresses display words and numbers:

- `Goldfish`
- `AVATAR`
- `WAVY`
- `ToTaL`
- `OpenType`
- `10.000`

The ligature-capable suite stresses shaped lowercase words:

- `Goldfish`
- `office`
- `affinity`
- `final`
- `efficient`
- `fjord`

The important rule is that ligature examples are evaluated after shaping. If a
font substitutes `fi`, the algorithm evaluates `d|fi` and `fi|s`; it does not
kern inside the ligature glyph. If ligatures are disabled, `f|i` becomes an
ordinary adjacent pair again.

## Algorithm Shape

The current candidate is not one universal formula. It is a small guarded
pipeline:

```text
shape text
for each adjacent shaped glyph cluster:
  compute metric kerning delta
  compute outline gap profile
  compute nearest contour gap
  compute pair class and local geometry
  choose metric-prior base delta
  apply safety bounds
  apply additive optical targets
apply run-level context after all pairs are known
normalize and emit em deltas
```

The algorithm is dynamic. It reads or computes all thresholds from the font:
x-height/cap-height profile bands, median profile gap, median absolute
deviation, pair class distributions, metric deltas, and shaped glyph clusters.
It does not contain per-font or per-word branches.

## Guard Model

The main lesson from early versions is that naive outline profiles over-tighten.
Contours alone can misread apertures, counters, serifs, script joins, and round
forms. The current guard model separates three responsibilities:

- Metric prior: preserve useful font kerning unless the outline evidence is
  clearly better.
- Safety bounds: prevent local collisions, false diagonal openings, and
  aperture-biased tightening.
- Run context: adjust full words when many individually plausible pair
  decisions accumulate into a visibly wrong word width.

This is closer to what would be maintainable in Typst than an opaque score
function. Each guard is small and testable, and each new rule must prove that it
fixes a class of shapes without destabilizing the regression suite.

## Current V24 Results

Current reproducible artifacts:

- `renders/optical-comparison-suite/ligatures-100pt-v24/summary.json`
- `renders/optical-comparison-suite/ligatures-100pt-v24/contact-sheet.png`
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24/summary.json`
- `renders/optical-comparison-suite/no-ligatures-100pt-five-font-v24/contact-sheet.png`
- `baselines/optical-ligature-suite-v24.json`
- `baselines/optical-comparison-suite-five-font-v24.json`

Summary:

| Suite | Cases | Mean score | Worst score | Regression note |
| --- | ---: | ---: | ---: | --- |
| Ligatures V23 | 31 | `0.0127em` | `0.0288em` | previous baseline |
| Ligatures V24 | 31 | `0.0123em` | `0.0240em` | only Libre `final` changed |
| No ligatures V23 | 30 | `0.0177em` | `0.0384em` | previous baseline |
| No ligatures V24 | 30 | `0.0177em` | `0.0384em` | zero changed cases |

V24 specifically fixes a short wide-serif `fi` ligature word:

```text
Libre Baskerville / final
score: 0.0288em -> 0.0168em
width: +0.0288em -> +0.0168em
ink:   0.0081em -> 0.0067em
```

The no-ligature suite is unchanged, which is important evidence that the
ligature-specific rule does not destabilize established no-ligature behavior.

## Reproduction Commands

```sh
cargo test

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
```

## What This Suggests For Typst

The strongest candidate direction is not a global `optical = true` raster-like
operation. It is a shaped-glyph, outline-profile, metric-prior algorithm with
small caches:

- cache glyph outlines and profile samples per font instance,
- cache pair geometry per glyph pair,
- compute deltas after shaping, so ligatures and substitutions are respected,
- preserve metric kerning when it is already strong,
- apply optical corrections only where dynamic evidence shows an outlier.

The public Typst API could later be discussed separately, for example:

```typst
#set text(kerning: "metric")
#set text(kerning: "optical")
#set text(kerning: "hybrid")
```

The benchmark is deliberately not an API proposal yet. Its value is to make the
behavioral tradeoffs visible before an implementation proposal is made.

## Remaining Work

- Broaden the font corpus while keeping the parity gates strict.
- Add more display-size words and number cases that designers actually care
  about.
- Keep optimizing only when a failure has a dynamic shape cause, not a
  font-name-specific exception.
- Turn the current Markdown draft into a polished public article with selected
  images from the V24 contact sheets.
- Decide which result tables and overlays should be committed, released, or
  generated in CI.

