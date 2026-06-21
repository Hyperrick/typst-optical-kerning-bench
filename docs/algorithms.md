# Algorithm Notes

All V1 algorithms operate on font outlines, not raster images. Raster analysis is
used only after rendering PDFs to evaluate output.

## Shared Preprocessing

1. Load the font with `ttf-parser`.
2. Map each character to a glyph id.
3. Flatten quadratic and cubic contours into deterministic line segments.
4. Build vertical profile samples across the Latin-relevant band
   `-0.20em..0.88em`.
5. Compare the left glyph's right profile against the right glyph's left
   profile at the default advance position.

The algorithms emit one pair delta in `em`. Typst render sheets apply this as:

```typst
#set text(kerning: false)
A#h(-0.042em)V
```

This avoids double-applying OpenType kerning.

## Implemented Algorithms

- `nearest-contour-distance`: preserves a minimum contour gap and otherwise
  moves toward the target gap using the closest sampled distance.
- `profile-whitespace`: uses the weighted mean profile gap. The x-height-like
  central band receives the strongest weight.
- `area-balance`: uses a robust mean after median/MAD filtering to reduce
  outlier influence.
- `metric-prior-hybrid`: uses HarfBuzz/Rustybuzz metric kerning as a prior and
  only moves toward optical spacing when the disagreement is large.
- `safe-fallback-only`: preserves metric kerning if it exists; otherwise falls
  back to the profile-whitespace delta.

## Typst Compatibility Constraints

These implementations deliberately favor:

- deterministic output
- no ML model in the layout path
- no rasterization in the layout path
- small state that can be cached per font and glyph pair
- deltas that can be represented as `em` advances after shaping

