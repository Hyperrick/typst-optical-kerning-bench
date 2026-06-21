# Guarded V4 Review Results

This pass adds side-shape awareness. It still avoids word-specific or
pair-specific overrides.

The algorithm now measures the interacting sides of a pair:

- right side of the left glyph
- left side of the right glyph

From outline slices it derives whether the side is round-like, stem-like, or
neutral. This is intended to cover forms such as `o`, `0`, and `1` without
hardcoding those characters.

## Run

```sh
scripts/run-guarded-review-batch.py \
  --output renders/guarded-v4-review/eb-garamond-100pt-no-ligatures
```

Settings:

- Font: `EB Garamond`
- Size: `100pt`
- Ligatures: `false`
- Samples: `32`
- Successful renders: `32`
- Review output: `renders/guarded-v4-review/eb-garamond-100pt-no-ligatures/index.html`
- Contact sheet: `renders/guarded-v4-review/eb-garamond-100pt-no-ligatures/contact-sheet.png`

## Summary

```text
Average absolute width error:
  guarded-v3: 0.0313em
  guarded-v4: 0.0235em

Worst absolute width error:
  guarded-v3: 0.1152em
  guarded-v4: 0.0624em
```

## Improvements

```text
1001        -0.1152em -> -0.0264em
0123456789  -0.1032em -> +0.0168em
10.000      -0.0888em -> +0.0192em
A10         -0.0288em -> +0.0120em
To          -0.0384em -> -0.0264em
ToTaL       -0.0744em -> -0.0624em
```

## Regressions

```text
V2.0        +0.0024em -> +0.0216em
2026        -0.0288em -> +0.0432em
```

## Interpretation

The side-shape pass is a strong improvement. It fixes the main number-only
failure class from v3 and also improves `To`/`ToTaL` slightly.

The tradeoff is that some digit cases are now slightly over-tightened,
especially `2026`, while `V2.0` moves from nearly perfect to still acceptable.
Goldfish remains unchanged.

The broadest remaining category by average error is now uppercase words rather
than figures.
