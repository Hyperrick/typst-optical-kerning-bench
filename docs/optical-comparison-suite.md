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
  --output renders/optical-comparison-suite/no-ligatures-100pt-v8-fast \
  --baseline-output baselines/optical-comparison-suite-v8-fast.json
```

Cross-font visual suite:

```sh
scripts/run-metric-parity-suite.py \
  --sample-matrix corpus/samples/metric-cross-font-suite.json \
  --output renders/metric-parity-suite/no-ligatures-100pt-v8-cross-font \
  --baseline-output baselines/metric-parity-suite-v8-cross-font.json
scripts/run-optical-comparison-suite.py \
  --suite cross-font \
  --metric-baseline baselines/metric-parity-suite-v8-cross-font.json \
  --output renders/optical-comparison-suite/no-ligatures-100pt-v8-cross-font \
  --baseline-output baselines/optical-comparison-suite-v8-cross-font.json
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
The `fast`, `cross-font`, and `full` case lists are explicit JSON files under
`corpus/samples/`.

## Outputs

- `summary.json`: full per-case render comparison.
- `index.html`: ranked table by optical score.
- `contact-sheet.png`: compact visual sheet.
- `baselines/optical-comparison-suite-v7.json`: compact committed baseline
  with worst cases and pair deltas.
- `baselines/optical-comparison-suite-v8-fast.json`: compact fast-suite
  baseline for algorithm iteration.
- `baselines/metric-parity-suite-v8-cross-font.json`: metric-valid cross-font
  baseline for the visual matrix.
- `baselines/optical-comparison-suite-v8-cross-font.json`: compact cross-font
  optical baseline.

Overlay colors:

```text
cyan    = InDesign Optical
magenta = Typst Guarded Optical
black   = overlap
```

## Current Result: V8

The latest full 30-case suite is still the V7 reference. Compared with the
original V1 baseline, V7 reduced mean score from `0.0428em` to `0.0249em` and
worst-case score from `0.1416em` to `0.0936em`, with no measured score
regressions. V8 has currently been verified on the 12-case fast suite and the
18-case cross-font visual matrix.

V7 full-suite worst cases by combined optical score:

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

The V8 fast suite measured all 12 selected iteration cases. Libre `WAVY`
hit one transient InDesign `-609`; rerunning only that case repaired the suite.
Its current worst cases are Libre `AVATAR` (`0.0552em` score), Libre `ToTaL`
(`0.0216em`), Inter `ToTaL` (`0.0192em`), Inter `ipsum` (`0.0192em`), and
Libre `WAVY` (`0.0172em`).

The V8 cross-font metric suite measured all 18 matrix cases as valid. The
optical cross-font suite then measured the same 18 cases. A focused local
aperture guard repaired the visible Libre `Goldfish` `G|o` collision, reducing
that row from `0.0413em` to `0.0149em`. The largest remaining numeric scores
are Libre `AVATAR` (`0.0552em`), Libre `10.000` (`0.0456em`), EB `AVATAR`
(`0.0384em`), EB `ToTaL` (`0.0384em`), and Inter `AVATAR` (`0.0360em`).

## Interpretation

The current guarded algorithm is much closer to InDesign Optical than V1, but
still has targeted failures.

Three patterns stand out after V8:

- **Sans context improved**: Inter `OpenType`, `ToTaL`, and `WAVY` are now
  close to InDesign Optical. The run-level V8 pass uses computed sans-like font
  spacing and metric-kerning density, not font names.
- **Libre Goldfish collision fixed**: the visible `G|o` collision came from a
  metricless upper-lower aperture case. The guard now clamps that pair from
  `-0.0958em` to `-0.0550em` instead of trusting the inflated profile mean.
- **Inter AVATAR remains the sans stress case**: it is much closer than before,
  but still the largest Inter row in the cross-font sheet.
- **Libre/EB display caps are now the main failures**: Libre `AVATAR`, EB
  `AVATAR`, and EB `ToTaL` need serif-specific contour safeguards.
- **Numeric and punctuation cases need their own class**: Libre `10.000` is the
  strongest current numeric target; it should not be tuned with ordinary letter
  clamps.

Good or near-good controls:

- EB Garamond `Goldfish`, `AV`, and `WA` are small enough to treat as controls.
- Inter `LANDMARK` and `10.000` are close after static font parity.
- Libre `WAVY` and `ToTaL` are now much improved compared with V1.

## Next Algorithm Focus

The next algorithm pass should stay narrow. The current evidence points to
three focused changes:

1. Preserve and broaden the local aperture/collision guard for metricless
   upper-lower pairs without making good `Goldfish` rows wider.
2. Treat Libre `AVATAR`, EB `AVATAR`, and EB `ToTaL` as visual tuning targets,
   not as hard failures.
3. Add a numeric/punctuation class only if Libre `10.000` remains visibly wrong
   after closer inspection.
