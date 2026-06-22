# Guarded V6 Review Results

V6 adds measurement before adding more aggressive tuning.

Two changes landed:

- PNG comparisons now report ink-position and segment-center errors, not only
  total width error.
- Per-font class-local gap distributions are computed, but applied only as a
  small blend for uppercase, digit, and punctuation classes.

The first full class-local replacement was rejected because it made EB Garamond
numbers and uppercase pairs worse. The committed version keeps the class signal
deliberately damped.

## EB Garamond

```text
Samples: 32/32

Average absolute width error:
  guarded-v5: 0.0184em
  guarded-v6: 0.0185em

Worst absolute width error:
  guarded-v5: 0.0624em
  guarded-v6: 0.0624em

New V6 position metrics:
  average ink-position error:     0.0115em
  worst ink-position error:       0.0303em
  average segment-center error:   0.0122em
  worst segment-center error:     0.0326em
```

V6 is essentially neutral against V5 on total width for EB Garamond. The useful
part is the new diagnostic layer: cases such as `ToTaL` now show both width
error and internal ink-position error.

Notable width changes:

```text
AVATAR  +0.0528em -> +0.0384em
P.      -0.0024em -> +0.0000em
VA      -0.0096em -> -0.0216em
WAVY    -0.0096em -> -0.0144em
```

## Multi-Font Smoke

```text
EB Garamond
  metric parity avg/max: 0.0015em / 0.0072em
  optical avg/max:       0.0185em / 0.0624em
  ink avg/max:           0.0115em / 0.0303em

Libre Baskerville
  metric parity avg/max: 0.0103em / 0.1032em
  optical avg/max:       0.0433em / 0.1152em
  ink avg/max:           0.0259em / 0.0830em

Inter
  metric parity avg/max: 0.1672em / 0.4416em
  optical avg/max:       0.1048em / 0.3624em
  ink avg/max:           0.0554em / 0.2065em
```

Inter is not a valid optical-algorithm ranking case yet. Typst Metric and
InDesign Metrics already differ strongly, so the InDesign Optical comparison is
dominated by font/rendering parity. This should be fixed before tuning the
algorithm against Inter.

## Interpretation

V6 should be treated as an evaluation upgrade, not a visual breakthrough.

The key outcome is that we can now distinguish:

- total word width error,
- internal ink distribution error,
- rough segment-center error,
- and baseline validity via Typst Metric vs InDesign Metrics parity.

The next productive step is to fix font parity for non-EB fonts, especially
Inter, before using them to tune the algorithm.
