# Guarded V5 Review Results

This pass adds contact-zone awareness on top of the v4 side-shape model. It
still avoids word-specific or pair-specific overrides.

The new rules are deterministic and outline-derived:

- local outline collision opening
- uppercase punctuation tightening
- round-to-upper-overhang tightening

The goal is to handle serif/contact cases such as `YF`, `P.`, `T.`, and `oT`
without teaching the algorithm those exact strings.

## Run

```sh
scripts/run-guarded-review-batch.py \
  --output renders/guarded-v5-review/eb-garamond-100pt-no-ligatures
```

Settings:

- Font: `EB Garamond`
- Size: `100pt`
- Ligatures: `false`
- Samples: `32`
- Successful renders: `32`
- Review output: `renders/guarded-v5-review/eb-garamond-100pt-no-ligatures/index.html`
- Contact sheet: `renders/guarded-v5-review/eb-garamond-100pt-no-ligatures/contact-sheet.png`

## Summary

```text
Average absolute width error:
  guarded-v4: 0.0235em
  guarded-v5: 0.0184em

Worst absolute width error:
  guarded-v4: 0.0624em
  guarded-v5: 0.0624em
```

## Improvements

```text
T.          -0.0480em -> +0.0072em
P.          -0.0408em -> -0.0024em
WAYFINDER   +0.0456em -> +0.0072em
ToTaL       -0.0624em -> -0.0384em
WAVY        +0.0288em -> -0.0096em
```

## Regressions

```text
No measured regression in this 32-sample EB Garamond batch.
```

## Interpretation

The contact-zone pass fixes the specific serif and punctuation problems that
were visible in v4. `YF` is no longer merely blocked from tightening; when the
nearest-contour pass detects an actual local collision, v5 opens the pair.

`P.` and `T.` are now handled as uppercase punctuation instead of ordinary
profile pairs. This matters because the relevant visual gap is around the dot,
not across the full cap-height profile.

`ToTaL` improves because `oT` is treated as a round side moving under a strong
top-left overhang. The result is closer to InDesign Optical, but still not
perfect.

The broadest remaining visible class is uppercase words, especially cases such
as `LANDMARK` and `AVATAR`, where total word width can still differ from
InDesign Optical even when individual pair decisions are locally plausible.
