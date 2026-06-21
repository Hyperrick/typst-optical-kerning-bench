# Guarded V3 Review Results

This pass targets the `lowercase -> uppercase` failure class that remained
visible in `ToTaL` and `OpenType`.

It does not add word-specific or pair-specific overrides. The new signal is
derived from the right glyph outline: if an uppercase glyph has a strong
top-left overhang and a much more rightward lower-left edge, safe
lowercase-to-uppercase pairs may be tightened more strongly.

## Run

```sh
scripts/run-guarded-review-batch.py \
  --output renders/guarded-v3-review/eb-garamond-100pt-no-ligatures
```

Settings:

- Font: `EB Garamond`
- Size: `100pt`
- Ligatures: `false`
- Samples: `32`
- Successful renders: `32`
- Review output: `renders/guarded-v3-review/eb-garamond-100pt-no-ligatures/index.html`
- Contact sheet: `renders/guarded-v3-review/eb-garamond-100pt-no-ligatures/contact-sheet.png`

## Summary

```text
Average absolute width error:
  guarded-v2: 0.0342em
  guarded-v3: 0.0313em

Worst absolute width error:
  guarded-v2: 0.1320em
  guarded-v3: 0.1152em
```

## Improvements

```text
ToTaL     -0.1320em -> -0.0744em
OpenType  -0.0600em -> -0.0264em
```

All other samples in the 32-sample review stayed unchanged in width-error terms.

## Interpretation

The lower-to-upper overhang rule is narrowly scoped and useful. It improves the
intended mixed-word cases without moving Goldfish, uppercase pairs, lowercase
words, punctuation pairs, or figures.

The main remaining failure class is still number-only figure runs. That should
be handled separately instead of tuning the lower-to-upper rule further.
