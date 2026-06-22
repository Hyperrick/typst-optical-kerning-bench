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
- `contact-sheet.png`: compact visual sheet.
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

## Current Result: V15

The current no-ligature five-font matrix compares InDesign Optical against
Typst Guarded Optical for 30 rows. V15 builds on the V14 digit-run context with
compact-sans and long-cap run handling. Compared with V14, the mean score
improved from `0.0240em` to `0.0177em`; worst case improved from `0.0648em` to
`0.0384em`, with no measured regression above `0.001em`.

V15 five-font worst cases by combined optical score:

```text
EB ToTaL:             -0.0384em width, 0.0294em ink
Libre AVATAR:         +0.0168em width, 0.0304em ink
Pacifico AVATAR:      -0.0120em width, 0.0253em ink
Pacifico OpenType:    -0.0168em width, 0.0242em ink
Comic Neue Goldfish:  +0.0240em width, 0.0130em ink
EB OpenType:          -0.0240em width, 0.0103em ink
Pacifico ToTaL:       -0.0120em width, 0.0239em ink
```

In the comparison metric, negative width means the Typst Guarded output is
wider than InDesign Optical; positive width means Typst Guarded is narrower.

The V15 focus suite covers `Goldfish`, `ToTaL`, `OpenType`, and `AVATAR`
across all five fonts. It catches the compact-sans `OpenType` regression that a
smaller focus sheet missed, so it is now the preferred fast iteration suite.

## Interpretation

The current guarded algorithm is much closer to InDesign Optical than V1, but
still has targeted failures.

Four patterns stand out after V15:

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

## Next Algorithm Focus

The next algorithm pass should stay narrow. The current evidence points to
three focused changes:

1. Preserve and broaden the local aperture/collision guard for metricless
   upper-lower pairs without making good `Goldfish` rows wider.
2. Treat Libre `AVATAR`, EB `AVATAR`, and EB `ToTaL` as visual tuning targets,
   not as hard failures.
3. Treat EB `ToTaL`, Pacifico `AVATAR`, and Pacifico `OpenType` as the next
   visual tuning targets, while keeping V14 digit-run and V15 compact-sans
   improvements locked by tests.
