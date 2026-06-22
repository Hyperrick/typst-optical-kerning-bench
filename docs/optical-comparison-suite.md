# Optical Comparison Suite

The optical comparison suite runs only samples that passed the metric parity
suite, then compares:

```text
InDesign Optical vs Typst Guarded Optical
```

This keeps the optical result separate from font-selection, variable-axis, and
metric-shaping problems.

## Command

Fast iteration suite:

```sh
scripts/run-optical-comparison-suite.py \
  --suite fast \
  --metric-baseline baselines/metric-parity-suite-v1.json \
  --output renders/optical-comparison-suite/no-ligatures-100pt-v7-fast \
  --baseline-output baselines/optical-comparison-suite-v7-fast.json
```

Full verification suite:

```sh
scripts/run-optical-comparison-suite.py \
  --suite full \
  --metric-baseline baselines/metric-parity-suite-v1.json \
  --output renders/optical-comparison-suite/no-ligatures-100pt-v7 \
  --baseline-output baselines/optical-comparison-suite-v7.json
```

The suite reuses the same isolated no-ligature sandbox fonts as the metric
parity suite. `sample-deltas` receives the exact sandbox `fontPath`, so the
guarded algorithm evaluates the same static font that InDesign and Typst render.
The `fast` and `full` case lists are explicit JSON files under
`corpus/samples/`.

## Outputs

- `summary.json`: full per-case render comparison.
- `index.html`: ranked table by optical score.
- `contact-sheet.png`: compact visual sheet.
- `baselines/optical-comparison-suite-v7.json`: compact committed baseline
  with worst cases and pair deltas.
- `baselines/optical-comparison-suite-v7-fast.json`: compact fast-suite
  baseline for algorithm iteration.

Overlay colors:

```text
cyan    = InDesign Optical
magenta = Typst Guarded Optical
black   = overlap
```

## Current Result: V7

The current full suite measured all 30 metric-valid cases. Compared with the
original V1 baseline, V7 reduced mean score from `0.0428em` to `0.0249em` and
worst-case score from `0.1416em` to `0.0936em`, with no measured score
regressions.

Worst cases by combined optical score:

```text
Inter OpenType:       -0.0936em width, 0.0401em ink
Libre AVATAR:         +0.0552em width, 0.0310em ink
Libre Goldfish:       +0.0360em width, 0.0413em ink
EB AVATAR:            +0.0384em width, 0.0184em ink
EB ToTaL:             -0.0384em width, 0.0294em ink
Inter WAYFINDER:      +0.0384em width, 0.0171em ink
Inter 1001:           +0.0384em width, 0.0252em ink
Inter V2.0:           -0.0360em width, 0.0141em ink
Inter 0123456789:     +0.0288em width, 0.0263em ink
EB To:                -0.0264em width, 0.0119em ink
```

In the comparison metric, negative width means the Typst Guarded output is
wider than InDesign Optical; positive width means Typst Guarded is narrower.

The fast suite measured all 12 selected iteration cases. Its current worst
cases are Inter `OpenType` (`0.0936em` score), Libre `AVATAR` (`0.0552em`),
Libre `AV` (`0.0240em`), Libre `ToTaL` (`0.0216em`), and Inter `ipsum`
(`0.0192em`).

## Interpretation

The current guarded algorithm is much closer to InDesign Optical than V1, but
still has targeted failures.

Three patterns stand out:

- **Inter mixed/lowercase accumulation**: `OpenType` remains the strongest
  failure. Several small lowercase corrections add up across the word, so V8
  should guard accumulation instead of applying blanket lowercase compaction.
- **Numeric and punctuation cases**: `1001`, `0123456789`, `V2.0`, and `A10`
  need a separate low-clamp class rather than sharing normal letter heuristics.
- **Serif display caps**: `AVATAR` and related uppercase sequences still need
  careful tuning, but only behind collision and aperture guards.

Good or near-good controls:

- EB Garamond `Goldfish`, `AV`, and `WA` are small enough to treat as controls.
- Inter `LANDMARK` and `10.000` are close after static font parity.
- Libre `WAVY` and `ToTaL` are now much improved compared with V1.

## Next Algorithm Focus

The next algorithm pass should stay narrow. The current evidence points to
three focused changes:

1. Add a lowercase accumulation guard for sans words, using `OpenType`,
   `valley`, and `ipsum`.
2. Add a numeric/punctuation class with smaller clamps, using `1001`, `V2.0`,
   `A10`, and `10.000`.
3. Carefully tune serif display-cap pairs, using Libre `AVATAR` while keeping
   EB `AV`, `WA`, and `Goldfish` neutral.
