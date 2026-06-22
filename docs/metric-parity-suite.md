# Metric Parity Suite

The metric parity suite verifies that InDesign Metrics and Typst Metrics render
the same baseline before any optical-kerning result is interpreted.

This is stricter than checking a single word. It runs a matrix of pair, word,
number, and mixed-case samples through the same isolated benchmark fonts used by
the `Goldfish` gate.

## Command

Build the isolated no-ligature fonts first:

```sh
python3 -m venv .venv-fonttools
.venv-fonttools/bin/pip install -r requirements-fonttools.txt
.venv-fonttools/bin/python scripts/build-parity-fonts.py
```

Then run the metric-only suite:

```sh
scripts/run-metric-parity-suite.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --point-size 100 \
  --ligatures false \
  --metric-threshold-em 0.02 \
  --output renders/metric-parity-suite/no-ligatures-100pt \
  --baseline-output baselines/metric-parity-suite-v1.json
```

The underlying render pipeline is called with `--metric-only`, so it renders
only:

- InDesign None
- Typst None
- InDesign Metrics
- Typst Metrics

This keeps the gate focused on baseline parity and avoids mixing optical
algorithm differences into the evidence.

## Current Sample Matrix

```text
EB Garamond:
Goldfish, AV, VA, WA, To, AVATAR, WAVY, ToTaL

Libre Baskerville:
Goldfish, AV, VA, WA, To, AVATAR, WAVY, ToTaL

Inter:
Goldfish, WAVY, WAYFINDER, LANDMARK, valley, yellow, lorem, ipsum,
OpenType, 0123456789, 1001, 10.000, A10, V2.0
```

## Current Result

The current no-ligature sandbox suite passes all 30 cases:

```text
30 / 30 valid
threshold: 0.02em absolute metric width delta
```

Worst cases:

```text
Inter LANDMARK:  +0.0192em
Inter WAYFINDER: +0.0192em
Libre AVATAR:   +0.0120em
Libre Goldfish: +0.0120em
```

The Inter long-word cases are close to the gate and should remain visible in
future reviews, but they currently pass.

## Outputs

- `summary.json`: full per-case report.
- `index.html`: table view with metric overlays.
- `contact-sheet.png`: compact visual sheet.
- `baselines/metric-parity-suite-v1.json`: compact committed baseline.

Overlay colors:

```text
cyan    = InDesign
magenta = Typst
black   = overlap
```
