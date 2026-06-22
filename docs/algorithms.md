# Algorithm Notes

All V1 algorithms operate on font outlines, not raster images. Raster analysis is
used only after rendering PDFs to evaluate output.

## Shared Preprocessing

1. Load the font with `ttf-parser`.
2. Read font metrics such as x-height/cap-height when available.
3. Shape the sample with Rustybuzz and the same OpenType feature settings as
   the renderer mode, then read the resulting glyph ids and clusters.
4. Flatten quadratic and cubic contours into deterministic line segments.
5. Build vertical profile samples across a Latin-relevant band derived from the
   font metrics.
6. Compare the left glyph's right profile against the right glyph's left profile
   at the default advance position.

The benchmark evaluates every adjacent non-space glyph pair in a word after
shaping. Pair samples disable standard ligatures so explicit pair cases remain
literal. Word samples keep standard ligatures enabled, matching normal Typst and
InDesign text behavior. Looking at every pair does not mean changing every pair.
The current candidate methods derive a per-font robust gap distribution from a
Latin reference alphabet and apply optical deltas only when a pair is an outlier
against that distribution.

Ligatures must be handled after shaping. If a font and the active Typst feature
settings turn `f` + `i` into a single `fi` ligature glyph, the algorithm must not
kern inside that ligature. It should instead evaluate the ligature glyph against
its real shaped neighbors, for example `d-fi_ligature` and `fi_ligature-s` in a
word like `Goldfish`. If ligatures are disabled, `f` and `i` remain separate
glyphs and `f-i` becomes an ordinary adjacent pair again.

The algorithms emit one glyph-pair delta in `em`. Typst render sheets apply this
as explicit spacing for simple pair review:

```typst
#set text(kerning: false)
A#h(-0.042em)V
```

This avoids double-applying OpenType kerning for pair sheets. For
ligature-sensitive word review, deltas must be computed after shaping and
applied to shaped glyph positions. Inserting `#h(...)` into source text is only
a prototype trick for non-ligature review because it can change the source
sequence that forms a ligature.

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
- `guarded-profile-hybrid`: starts from the metric-prior hybrid, then blocks
  negative optical corrections when the nearest contour gap is already small
  but the profile average is inflated by an aperture or counter-like shape.
  The current pass also adds contact-zone rules for local outline collisions,
  uppercase punctuation, round-to-overhang pairs, and V8 run-context tuning for
  sans-like display words. This is intended to catch cases such as Libre
  Baskerville `G|o`, EB Garamond `Y|F`, `P|.`, `T|.`, and `o|T` without
  hard-coding a font or glyph name.
- `safe-fallback-only`: preserves metric kerning if it exists; otherwise uses
  the robust optical outlier correction. Monospaced fonts are preserved.

## V6 Calibration Notes

The core now computes class-local gap distributions per font for broad classes
such as uppercase-uppercase, digit-digit, and uppercase-punctuation. These
values are not allowed to fully replace the global font distribution. A full
replacement was tested and rejected because it over-widened EB Garamond numbers
and uppercase pairs.

The committed V6 behavior blends eligible class-local values into the global
distribution at a low weight. This keeps the signal available for tuning while
preserving the more stable V5 behavior.

## V8 Run-Context Notes

V8 evaluates a shaped word as a run, not only as isolated pairs. The guarded
output is still pair-based, but a small final adjustment can be applied when the
run looks sans-like and contains multiple strong metric-kerned uppercase or
mixed-case pairs. The trigger is computed from the font spacing profile, pair
classes, and existing metric deltas; it does not use font names or sample names.

This keeps the implementation deterministic and cacheable while fixing the
largest Inter failures from the cross-font matrix. The pass is intentionally
narrow: serif fonts and metricless uppercase controls stay near their previous
behavior.

The current guard also clamps metricless upper-lower aperture cases when the
right glyph is round-like and the nearest contour distance is already critical.
This fixes the Libre Baskerville `G|o` collision without naming the font or
sample.

## Typst Compatibility Constraints

These implementations deliberately favor:

- deterministic output,
- no ML model in the layout path,
- no rasterization in the layout path,
- small state that can be cached per font and glyph pair,
- dynamic values read from or computed from the font, not hand-set per font,
- deltas that can be represented as `em` advances after shaping.
