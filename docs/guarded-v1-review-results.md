# Guarded V1 Review Results

This review keeps the Goldfish-tuned `guarded-profile-hybrid` unchanged and
renders broader samples to find failure classes.

## Run

```sh
scripts/run-guarded-review-batch.py \
  --output renders/guarded-v1-review/eb-garamond-100pt-no-ligatures
```

Settings:

- Font: `EB Garamond`
- Size: `100pt`
- Ligatures: `false`
- Samples: `32`
- Successful renders: `32`
- Review output: `renders/guarded-v1-review/eb-garamond-100pt-no-ligatures/index.html`
- Contact sheet: `renders/guarded-v1-review/eb-garamond-100pt-no-ligatures/contact-sheet.png`

## Strongest Width Deviations

Negative means Typst Guarded is wider than InDesign Optical.
Positive means Typst Guarded is narrower than InDesign Optical.

```text
word-total       ToTaL        -86px / -0.2064em
number-v2        V2.0         +44px / +0.1056em
number-digits    0123456789   -41px / -0.0984em
pair-to          To           -33px / -0.0792em
pair-t-period    T.           -32px / -0.0768em
number-1001      1001         -30px / -0.0720em
pair-ly          LY           +29px / +0.0696em
number-a10       A10          -29px / -0.0696em
word-landmark    LANDMARK     +26px / +0.0624em
word-wavy        WAVY         -23px / -0.0552em
word-opentype    OpenType     -23px / -0.0552em
pair-yo          Yo           -23px / -0.0552em
```

## Category Signal

```text
baseline-word      n=1  avg_abs=0.0048em  max_abs=0.0048em
lowercase-pair     n=2  avg_abs=0.0048em  max_abs=0.0048em
lowercase-word     n=7  avg_abs=0.0195em  max_abs=0.0456em
uppercase-word     n=4  avg_abs=0.0336em  max_abs=0.0624em
uppercase-pair     n=4  avg_abs=0.0420em  max_abs=0.0696em
numbers            n=4  avg_abs=0.0510em  max_abs=0.0984em
punctuation-pair   n=2  avg_abs=0.0588em  max_abs=0.0768em
mixed-pair         n=3  avg_abs=0.0616em  max_abs=0.0792em
alphanumeric       n=2  avg_abs=0.0876em  max_abs=0.1056em
mixed-word         n=3  avg_abs=0.0944em  max_abs=0.2064em
```

## Initial Interpretation

The Goldfish result is good, but the current baseline is not general enough.
The failure classes are useful:

- `T`-related mixed pairs are too weak compared with InDesign Optical.
- `LY` and some all-caps words are too tight, so diagonal/cap combinations need
  a better guard.
- Figure and alphanumeric examples need separate treatment; one rule does not
  behave consistently across `0123456789`, `1001`, `A10`, and `V2.0`.
- Lowercase-only words are comparatively stable, although `lorem` is already a
  warning case.

The next tuning step should not optimize one sample. It should add class-aware
rules for uppercase/lowercase, figures, punctuation, and mixed-script-like
boundaries, then rerun this exact review batch and keep Goldfish as a regression
case.
