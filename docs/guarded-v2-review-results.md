# Guarded V2 Review Results

This pass keeps the algorithm deterministic and avoids word-specific or
pair-specific overrides. The new logic only uses broad cluster classes:

- uppercase
- lowercase
- digit
- punctuation
- other

The goal was to reduce broad failure classes from `guarded-v1`, not to clone
InDesign Optical.

## Run

```sh
scripts/run-guarded-review-batch.py \
  --output renders/guarded-v2-review/eb-garamond-100pt-no-ligatures
```

Settings:

- Font: `EB Garamond`
- Size: `100pt`
- Ligatures: `false`
- Samples: `32`
- Successful renders: `32`
- Review output: `renders/guarded-v2-review/eb-garamond-100pt-no-ligatures/index.html`
- Contact sheet: `renders/guarded-v2-review/eb-garamond-100pt-no-ligatures/contact-sheet.png`

## Summary

```text
Average absolute width error:
  guarded-v1: 0.0443em
  guarded-v2: 0.0342em

Worst absolute width error:
  guarded-v1: 0.2064em
  guarded-v2: 0.1320em
```

## Strong Improvements

```text
V2.0      +0.1056em -> +0.0024em
ToTaL     -0.2064em -> -0.1320em
LY        +0.0696em -> +0.0000em
AV        -0.0456em -> -0.0024em
Yo        -0.0552em -> -0.0144em
To        -0.0792em -> -0.0384em
A10       -0.0696em -> -0.0288em
WA        -0.0432em -> -0.0048em
Ta        -0.0504em -> -0.0144em
T.        -0.0768em -> -0.0480em
```

## Regressions

```text
10.000    -0.0192em -> -0.0888em
1001      -0.0720em -> -0.1152em
AVATAR    +0.0120em -> +0.0528em
WAYFINDER +0.0048em -> +0.0456em
2026      +0.0144em -> -0.0288em
```

## Interpretation

The class-aware pass is directionally better for text-like cases:

- uppercase pairs are much better
- mixed pairs are much better
- alphanumeric examples are much better
- punctuation pairs improve
- lowercase is unchanged
- Goldfish is unchanged

The main problem is figures. Number-only samples got worse, especially `1001`
and `10.000`. The next pass should treat figure runs separately instead of
letting digit pairs receive the same fallback compaction and optical pull as
letters.

This is a useful step, but not the final algorithm.
