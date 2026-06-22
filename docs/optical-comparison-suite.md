# Optical Comparison Suite

The optical comparison suite runs only samples that passed the metric parity
suite, then compares:

```text
InDesign Optical vs Typst Guarded Optical
```

This keeps the optical result separate from font-selection, variable-axis, and
metric-shaping problems.

## Command

```sh
scripts/run-optical-comparison-suite.py \
  --metric-baseline baselines/metric-parity-suite-v1.json \
  --output renders/optical-comparison-suite/no-ligatures-100pt \
  --baseline-output baselines/optical-comparison-suite-v1.json
```

The suite reuses the same isolated no-ligature sandbox fonts as the metric
parity suite. `sample-deltas` receives the exact sandbox `fontPath`, so the
guarded algorithm evaluates the same static font that InDesign and Typst render.

## Outputs

- `summary.json`: full per-case render comparison.
- `index.html`: ranked table by optical score.
- `contact-sheet.png`: compact visual sheet.
- `baselines/optical-comparison-suite-v1.json`: compact committed baseline
  with worst cases and pair deltas.

Overlay colors:

```text
cyan    = InDesign Optical
magenta = Typst Guarded Optical
black   = overlap
```

## Current Result

The current suite measured all 30 metric-valid cases.

Worst cases by combined optical score:

```text
Inter OpenType:       -0.1416em width, 0.0640em ink
Libre ToTaL:          -0.1152em width, 0.0830em ink
Libre WAVY:           -0.1104em width, 0.0613em ink
Libre AVATAR:         -0.0816em width, 0.0806em ink
Libre AV:             -0.0600em width, 0.0298em ink
Libre WA:             -0.0576em width, 0.0221em ink
Libre VA:             -0.0552em width, 0.0295em ink
Inter valley:         -0.0552em width, 0.0333em ink
Libre To:             -0.0480em width, 0.0216em ink
Inter ipsum:          -0.0432em width, 0.0273em ink
```

In the comparison metric, negative width means the Typst Guarded output is
wider than InDesign Optical.

## Interpretation

The current guarded algorithm generally under-tightens relative to InDesign
Optical in the largest failures.

Two patterns stand out:

- **Libre Baskerville uppercase diagonals and `To` pairs**: `AV`, `VA`, `WA`,
  `To`, `AVATAR`, `WAVY`, and `ToTaL` stay too wide compared with InDesign
  Optical. `WAVY` is especially useful because `V|Y` currently opens
  (`+0.0442em`) while InDesign Optical is visually tighter.
- **Inter mixed/lowercase words**: `OpenType`, `valley`, and `ipsum` show that
  the small generic lowercase correction is too weak for InDesign-like optical
  spacing. `OpenType` is the strongest current failure.

Good or near-good controls:

- Inter `LANDMARK` is close after static font parity.
- EB Garamond `Goldfish`, `AV`, `WA`, and `WAVY` are small enough to treat as
  controls.
- Numeric Inter cases are mostly moderate; `10.000` is close.

## Next Algorithm Focus

The next algorithm pass should not start with a broad rewrite. The current
evidence points to three focused changes:

1. Tighten guarded handling for uppercase diagonal pairs when InDesign Optical
   is consistently tighter than metric kerning.
2. Remove or strongly limit the false positive opening for `V|Y`-like
   diagonals.
3. Add a better lowercase/mixed-case spacing path for sans fonts, using
   `OpenType`, `valley`, and `ipsum` as regression targets.
