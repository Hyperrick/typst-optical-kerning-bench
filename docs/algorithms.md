# Algorithm Notes

All V1 algorithms operate on font outlines, not raster images. Raster analysis is
used only after rendering PDFs to evaluate output.

## Shared Preprocessing

1. Load the font with `ttf-parser`.
2. Read font metrics such as x-height/cap-height when available.
3. Map each corpus character to a glyph id.
4. Flatten quadratic and cubic contours into deterministic line segments.
5. Build vertical profile samples across a Latin-relevant band derived from the
   font metrics.
6. Compare the left glyph's right profile against the right glyph's left profile
   at the default advance position.

The benchmark evaluates every adjacent non-space glyph pair in a word. Looking
at every pair does not mean changing every pair. The current candidate methods
derive a per-font robust gap distribution from a Latin reference alphabet and
apply optical deltas only when a pair is an outlier against that distribution.

The algorithms emit one pair delta in `em`. Typst render sheets apply this as:

```typst
#set text(kerning: false)
A#h(-0.042em)V
```

This avoids double-applying OpenType kerning.

## Dynamic Font Calibration

The benchmark no longer uses hand-set per-font values. For each font it computes:

- the Latin profile sampling band from x-height/cap-height when available,
- a median profile-gap value over a reference alphabet,
- a median absolute deviation (MAD) over the same sampled gaps,
- a monospaced-font signal from `post.isFixedPitch` or uniform measured advance
  widths.

The median/MAD pair defines a font-local "normal" spacing range. Normal pairs
return a zero optical delta. Wide or tight outliers move partway back toward the
normal range. This follows the practical lesson from optical-kerning tools:
constant spacing is too naive, but robust outlier detection is explainable and
cacheable.

## Implemented Algorithms

- `nearest-contour-distance`: preserves a minimum contour gap and otherwise
  applies a light outlier correction using the nearest sampled distance.
- `profile-whitespace`: uses the weighted mean profile gap. The x-height-like
  central band receives the strongest weight.
- `area-balance`: uses a robust mean after median/MAD filtering to reduce
  outlier influence.
- `metric-prior-hybrid`: uses HarfBuzz/Rustybuzz metric kerning as a prior. It
  preserves existing metric kerning when it is close to the optical estimate and
  blends only when the disagreement is large. Monospaced fonts are preserved.
- `safe-fallback-only`: preserves metric kerning if it exists; otherwise uses
  the robust optical outlier correction. Monospaced fonts are preserved.

## Typst Compatibility Constraints

These implementations deliberately favor:

- deterministic output,
- no ML model in the layout path,
- no rasterization in the layout path,
- small state that can be cached per font and glyph pair,
- dynamic values read from or computed from the font, not hand-set per font,
- deltas that can be represented as `em` advances after shaping.
