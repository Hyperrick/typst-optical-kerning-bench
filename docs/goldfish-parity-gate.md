# Goldfish Parity Gate

`Goldfish` is the focused sanity test before using a font for optical kerning
tuning.

The benchmark first checks whether both layout engines agree without optical
kerning:

```text
InDesign Metrics ~= Typst Metrics
```

Only fonts that pass this gate are valid evidence for the optical comparison:

```text
InDesign Optical ~= Typst Guarded Optical
```

This avoids tuning the algorithm against unrelated differences such as font
instance selection, feature settings, shaping, export scaling, or crop behavior.

## Command

```sh
scripts/run-goldfish-parity.py \
  --fonts eb-garamond,libre-baskerville,inter \
  --text Goldfish \
  --point-size 100 \
  --ligatures false \
  --metric-threshold-em 0.02 \
  --output renders/goldfish-parity/goldfish-100pt-no-ligatures \
  --baseline-output baselines/goldfish-parity-v1.json
```

For the strict no-ligature comparison, build isolated static benchmark fonts
first:

```sh
python3 -m venv .venv-fonttools
.venv-fonttools/bin/pip install -r requirements-fonttools.txt
.venv-fonttools/bin/python scripts/build-parity-fonts.py
scripts/run-goldfish-parity.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --text Goldfish \
  --point-size 100 \
  --ligatures false \
  --metric-threshold-em 0.02 \
  --output renders/goldfish-parity/goldfish-100pt-no-ligatures-sandbox \
  --baseline-output baselines/goldfish-parity-sandbox-v1.json
```

The sandbox fonts are generated from the pinned corpus fonts and then used by
both engines: copied into InDesign's `Document fonts` folder and into Typst's
local `--font-path` directory. Typst is compiled with `--ignore-system-fonts`.

## Outputs

The script writes one per-font subdirectory with the existing single-word
pipeline outputs:

- `indesign-metric.png`
- `typst-metric.png`
- `metric-parity.png`
- `indesign-optical.png`
- `typst-guarded.png`
- `optical-vs-guarded.png`
- `metrics/comparison.json`

The suite-level outputs are:

- `summary.json`: full report with gate decisions.
- `index.html`: visual review table.
- `contact-sheet.png`: compact visual sheet.
- optional baseline JSON under `baselines/`.

## Gate Rule

The default threshold is:

```text
abs(metricParity.widthDeltaEm) <= 0.02em
```

If a font fails this rule, the result is labeled `baseline-mismatch`. That font
should not be used to tune the optical algorithm until Metric-vs-Metric parity
is fixed.

## Current No-Ligature Finding

The original failures had two different causes:

- Inter used a different variable-font instance or font selection path. Freezing
  `wght=400` and `opsz=14` under a unique family name fixes Metric-vs-Metric
  parity.
- Libre Baskerville had matching individual glyph outlines, but InDesign still
  formed the `fi` ligature in `Goldfish` even with ligatures disabled through
  scripting. The strict no-ligature sandbox removes standard ligature features,
  legacy glyph names, and Unicode presentation-form ligature cmap entries so the
  engines shape the same glyph sequence.

With the sandbox fonts, the current `Goldfish` metric gate passes for all three
tracked fonts:

```text
EB Garamond:        +0.0000em
Libre Baskerville:  +0.0120em
Inter:              -0.0024em
```

After this single-word gate passes, use the broader metric-only suite described
in [`metric-parity-suite.md`](metric-parity-suite.md) before treating a sample
set as optical tuning evidence.
