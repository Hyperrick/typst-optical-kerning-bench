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

Five-font verification suite:

```sh
scripts/run-optical-comparison-suite.py \
  --suite-file corpus/samples/optical-cross-font-suite.json \
  --metric-baseline baselines/metric-parity-suite-five-font-cross-font.json \
  --reuse-indesign-from renders/optical-comparison-suite/no-ligatures-100pt-five-font-cross-font \
  --output renders/optical-comparison-suite/no-ligatures-100pt-five-font-v14 \
  --baseline-output baselines/optical-comparison-suite-five-font-v14.json
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

Overlay colors:

```text
cyan    = InDesign Optical
magenta = Typst Guarded Optical
black   = overlap
```

## Current Result: V14

The current no-ligature five-font matrix compares InDesign Optical against
Typst Guarded Optical for 30 rows. V14 adds a digit-run context pass after the
V13 script-run work. Compared with the previous five-font current baseline, the
mean score improved from `0.0276em` to `0.0240em`, with no measured regression
above `0.001em`.

V14 five-font worst cases by combined optical score:

```text
Comic Neue Goldfish:  +0.0648em width, 0.0271em ink
Libre AVATAR:         +0.0552em width, 0.0310em ink
Comic Neue ToTaL:     +0.0504em width, 0.0332em ink
Comic Neue AVATAR:    -0.0480em width, 0.0261em ink
EB AVATAR:            +0.0384em width, 0.0184em ink
EB ToTaL:             -0.0384em width, 0.0294em ink
Inter AVATAR:         -0.0360em width, 0.0201em ink
```

In the comparison metric, negative width means the Typst Guarded output is
wider than InDesign Optical; positive width means Typst Guarded is narrower.

The V14 number-focus suite measured five `10.000` rows. Comic Neue improved
from `0.0984em` to `0.0144em`; Libre Baskerville improved from `0.0456em` to
`0.0198em`. EB Garamond, Inter, and Pacifico stayed effectively unchanged.

## Interpretation

The current guarded algorithm is much closer to InDesign Optical than V1, but
still has targeted failures.

Four patterns stand out after V14:

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
- **Comic Neue is the main remaining spread test**: after numeric tuning, its
  `Goldfish`, `ToTaL`, and `AVATAR` rows remain the largest non-serif visual
  deviations.

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
3. Treat Comic Neue `Goldfish`, Comic Neue `ToTaL`, and Libre `AVATAR` as the
   next visual tuning targets, while keeping the V14 digit-run improvements
   locked by tests.
