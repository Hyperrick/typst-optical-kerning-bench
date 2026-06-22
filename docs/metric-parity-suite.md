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

For a ligature-sensitive suite, build the sibling sandbox separately:

```sh
.venv-fonttools/bin/python scripts/build-parity-fonts.py \
  --variant ligatures \
  --spec-output renders/font-sandbox/goldfish-ligature-fonts.json
```

The no-ligature variant removes standard ligature features, legacy glyph names,
and presentation-form cmap entries. The ligature variant keeps those values so
InDesign and Typst can both shape real ligature glyph clusters before any
optical spacing is evaluated.

Then run the metric-only suite:

```sh
scripts/run-metric-parity-suite.py \
  --font-specs renders/font-sandbox/goldfish-no-ligature-fonts.json \
  --point-size 100 \
  --ligatures false \
  --metric-threshold-em 0.02 \
  --ink-threshold-em 0.02 \
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

Before the first case, the runner executes a one-shot InDesign automation
preflight. If InDesign is blocked by a modal dialog, crash recovery, or another
non-scriptable state, the suite exits before writing a baseline. This prevents
large all-`render-error` baselines from being committed accidentally.

After the suite finishes or aborts, the runner closes InDesign and removes
InDesign recovery and scripting state. This keeps Adobe's crash-recovery
"restore documents" dialog from blocking the next automated run. While
InDesign is starting, a best-effort watcher also clicks known negative recovery
buttons, because some crashes show the modal after the process is already
running and before ExtendScript accepts commands.

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
ink gate: 0.02em mean ink-position delta
```

Worst cases:

```text
Inter LANDMARK:  +0.0192em width, 0.0143em ink
Inter WAYFINDER: +0.0192em width, 0.0093em ink
Libre AVATAR:   +0.0120em width, 0.0076em ink
Libre Goldfish: +0.0120em width, 0.0059em ink
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
