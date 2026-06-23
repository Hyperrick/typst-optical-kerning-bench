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

Targeted number-focus suite:

```sh
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-number-focus-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-cross-font \
  --output renders/optical-comparison-suite/no-ligatures-100pt-number-focus-v14 \
  --baseline-output baselines/optical-comparison-suite-number-focus-v14.json
```

V15 focus suite:

```sh
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-v15-focus-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-cross-font \
  --output renders/optical-comparison-suite/no-ligatures-100pt-v15-focus \
  --baseline-output baselines/optical-comparison-suite-v15-focus.json
```

Five-font verification suite:

```sh
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-cross-font \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v15 \
  --baseline-output baselines/optical-comparison-suite-five-font-v15.json
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
- `contact-sheet.png`: compact visual sheet. For review runs, the sheet should
  group the same sample across fonts so visual differences are caused by font
  behavior and kerning, not by changing text every row.
- `baselines/optical-comparison-suite-v7.json`: compact committed baseline
  with worst cases and pair deltas.
- `baselines/optical-comparison-suite-v8-fast.json`: compact fast-suite
  baseline for algorithm iteration.
- `baselines/metric-parity-suite-v8-cross-font.json`: metric-valid cross-font
  baseline for the visual matrix.
- `baselines/optical-comparison-suite-v8-cross-font.json`: compact cross-font
  optical baseline.
- `baselines/optical-comparison-suite-number-focus-v14.json`: compact
  digit-run focus baseline.
- `baselines/optical-comparison-suite-five-font-v14.json`: compact five-font
  optical baseline after the V14 digit-run pass.
- `baselines/optical-comparison-suite-v15-focus.json`: compact V15 focus-suite
  baseline.
- `baselines/optical-comparison-suite-five-font-v15.json`: compact five-font
  optical baseline after compact-sans and long-cap tuning.

Overlay colors:

```text
cyan    = InDesign Optical
magenta = Typst Guarded Optical
black   = overlap
```

## Current Result After 25 Iterations

The current no-ligature five-font matrix compares InDesign Optical against
Typst Guarded Optical for 30 rows. The current candidate has gone through 25
evaluation iterations, building on compact sans / long-cap handling and later
ligature work. Compared with the early V14/V15 baseline, the mean score
improved from `0.0240em` to `0.0170em`; worst case improved from `0.0648em` to
`0.0304em`. Compared with the previous committed no-ligature baseline, exactly
one case changed: EB Garamond `ToTaL`.

Current five-font worst cases by combined optical score:

```text
Libre AVATAR:         +0.0168em width, 0.0304em ink
Pacifico AVATAR:      -0.0120em width, 0.0253em ink
Pacifico OpenType:    -0.0168em width, 0.0242em ink
Comic Neue Goldfish:  +0.0240em width, 0.0130em ink
EB OpenType:          -0.0240em width, 0.0103em ink
Pacifico ToTaL:       -0.0120em width, 0.0239em ink
EB 10.000:            +0.0192em width, 0.0216em ink
```

In the comparison metric, negative width means the Typst Guarded output is
wider than InDesign Optical; positive width means Typst Guarded is narrower.

The cross-font suite covers `Goldfish`, `AVATAR`, `WAVY`, `ToTaL`, `OpenType`,
and `10.000` across all five no-ligature fonts. It catches compact-sans,
serif, script, and numeric regressions that smaller focus sheets missed.

For quick mixed-case regression checks, `corpus/samples/optical-total-target-suite.json`
runs only `ToTaL` across the five no-ligature fonts. The latest iteration uses
this slice to prove that the EB Garamond improvement does not move Libre
Baskerville, Inter, Pacifico, or Comic Neue.

## Interpretation

The current guarded algorithm is much closer to InDesign Optical than V1. It
still has known residual visual differences, which are useful review boundaries
rather than a sign that the public artifact is still mid-tuning.

Four patterns stand out in the current five-font comparison:

- **Sans context improved**: Inter `OpenType`, `ToTaL`, and `WAVY` are now
  close to InDesign Optical. The run-level V8 pass uses computed sans-like font
  spacing and metric-kerning density, not font names.
- **Libre Goldfish collision fixed**: the visible `G|o` collision came from a
  metricless upper-lower aperture case. The guard now clamps that pair from
  `-0.0958em` to `-0.0550em` instead of trusting the inflated profile mean.
- **Inter AVATAR remains the sans stress case**: it is much closer than before,
  but still the largest Inter row in the cross-font sheet.
- **Remaining top cases are visual, not metric-parity failures**: Libre
  `AVATAR`, Pacifico `AVATAR`, Pacifico `OpenType`, and EB `OpenType` are the
  next no-ligature review targets.
- **Numeric and punctuation runs are now separate**: the V14 digit-run context
  tightens long metricless digit runs when the local gaps and font spacing show
  that ordinary pair guards are too weak. This is dynamic, not a font-name or
  sample-name exception.
- **Compact sans needs run context**: Comic Neue `Goldfish`, `ToTaL`, and
  `AVATAR` improved substantially only after treating simple lowercase,
  mixed-case, and all-caps runs differently.
- **Focus suites must include controls**: the first V15 focus set missed
  `OpenType`; adding it caught an over-loosening regression before the final
  full-suite baseline.

Good or near-good controls:

- EB Garamond `Goldfish`, `AV`, and `WA` are small enough to treat as controls.
- Inter `LANDMARK` and `10.000` are close after static font parity.
- Libre `WAVY` and `ToTaL` are now much improved compared with V1.

## Residual Review Notes

The notes below document where the current candidate still differs most visibly
from InDesign Optical. They are not a required next tuning queue for publishing
the benchmark. If the algorithm is taken further, any follow-up should stay
narrow and preserve the current regression controls.

1. Preserve the local aperture/collision guard for metricless upper-lower pairs
   without making good `Goldfish` rows wider.
2. Read Libre `AVATAR`, Pacifico `AVATAR`, and Pacifico `OpenType` as the main
   remaining visual review cases, not as metric-parity failures.
3. Read EB `OpenType`, EB `10.000`, and Pacifico `WAVY` as secondary visual
   review cases, while keeping the digit-run and compact-sans improvements
   locked by tests.
