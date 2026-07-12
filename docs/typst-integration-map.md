# Typst Integration Map

This document maps the external optical-kerning workbench to Typst's current
text-shaping pipeline. It describes a prototype boundary, not an accepted Typst
API or merge plan.

## Prototype Goal

The current experiment tests whether a small deterministic pass can adjust an already shaped glyph run
without replacing Rustybuzz, reinterpreting `kern` or GPOS tables, rasterizing
text, or charging metric-only documents for optical preprocessing.

The prototype reuses Typst's final shaped positions as the metric prior:

```text
Typst text styles and OpenType features
                  |
                  v
        Rustybuzz shape_with_plan
                  |
                  v
        Vec<ShapedGlyph> in em units
                  |
                  v
       apply_optical_kerning (opt-in)
                  |
                  v
          track_and_space
                  |
                  v
       calculate_adjustability
```

## Current Typst Locations

### Public text setting

`crates/typst-library/src/text/mod.rs`

- `TextElem::kerning` is currently a boolean with a default of `true`.
- `features(styles)` disables the OpenType `kern` feature when the setting is
  `false`.
- The experiment accepts the existing booleans plus `"optical"`. This is a
  prototype cast, not a proposed final enum contract.

### Shaping and glyph positions

`crates/typst-layout/src/inline/shaping.rs`

- `shape_segment` calls `rustybuzz::shape_with_plan` and converts the returned
  glyph ids, advances, offsets, clusters, fonts, and script data into
  `ShapedGlyph` values.
- `shape` collects all shaped segments into `ShapingContext::glyphs`, then calls
  `track_and_space` and `calculate_adjustability`.
- The prototype insertion point is after all `shape_segment` calls and before
  `track_and_space`.

Conceptually:

```rust,ignore
if !text.is_empty() {
    shape_segment(&mut ctx, base, text, families(styles));
}

if optical_kerning_enabled(ctx.styles) {
    apply_optical_kerning(&mut ctx.glyphs, &mut optical_cache);
}

track_and_space(&mut ctx);
calculate_adjustability(&mut ctx, lang, region);
```

The pass adjusts the left glyph's `x_advance` by the computed pair delta. It
does not change glyph ids, clusters, text ranges, `x_offset`, or `y_offset`.

## Runtime Contract

The workbench now isolates the decision logic in `crates/optikern-runtime`.
That crate does not depend on Rustybuzz, Typst, image libraries, PDF tooling, or
the CLI.

The host is responsible for producing `PairEvidence`:

```rust,ignore
pub struct PairEvidence {
    pub left: GlyphClass,
    pub right: GlyphClass,
    pub metric_delta: f32,
    pub optical_delta: f32,
    pub nearest_delta: f32,
    pub target_gap: f32,
    pub gap_mad: f32,
    pub min_gap: f32,
    pub robust_gap: f32,
    pub x_height: f32,
    pub cap_height: f32,
    pub left_side: SideShape,
    pub right_side: SideShape,
    pub right_top_left_overhang: f32,
    pub monospaced: bool,
}
```

The kernel returns one deterministic `em` delta. `compact_guarded_run` then
uses only aggregate class and metric signals from the shaped run to keep
individually reasonable pair decisions from accumulating into poor word
rhythm.

## Pair Eligibility

The first prototype should evaluate a boundary only when all of these hold:

- both glyphs belong to the same `FontInstance` and variation coordinates;
- neither glyph is whitespace;
- the boundary is between different shaped clusters;
- both outlines are available;
- the run is left-to-right Latin text;
- the font is not detected as monospaced.

Ligatures are naturally preserved because the pass runs after shaping. A
ligature glyph can be spaced against its neighboring cluster, but its internal
source-character boundary is no longer visible and therefore cannot be kerned.

Unsupported pairs return a zero optical correction and preserve the shaped
metric positions.

## Host-Side Geometry

The Typst adapter derives four deterministic, cacheable inputs:

1. flattened outlines for each glyph id;
2. left/right outline profiles over a font-relative vertical band;
3. a font-local robust gap distribution;
4. pair geometry such as minimum gap, side roundness/stemness, and top-left
   overhang.

Raster output and InDesign are evaluation-only. They are never part of the
compiler path.

## Cache Ownership

The current prototype separates four cache levels:

| Cache | Key | Value |
| --- | --- | --- |
| Glyph outline | font instance + variations + glyph id | flattened outline |
| Glyph side profile | font instance + variations + glyph id | sampled left/right side profiles |
| Font calibration | font instance + variations | target gap and MAD |
| Pair geometry | font instance + variations + left/right glyph ids | sampled gap and side-shape evidence |

The final pair delta is cheap once these values are warm. Metric-only shaping
must not populate any optical cache.

The implementation uses Typst's existing `comemo` conventions. Separating glyph
profiles from pair geometry reduced the 120-page optical-heading benchmark from
roughly `1.86s` in the first adapter to about `0.20s` without changing pixels in
the six-page visual sheet.

## Prototype API Boundary

The compiler experiment uses `kerning: "optical"` as an explicit prototype
surface while preserving `true` and `false`. The public discussion can still
choose between:

- conservative adjustment of shaped metric positions;
- fallback-only behavior when effective pair positioning is absent;
- a full optical replacement mode;
- or no upstream feature.

The workbench favors conservative adjustment. It should not expose separate
`kern`/GPOS selection as part of the optical algorithm: Rustybuzz's final shaped
positions are the prior regardless of which font data produced them.

## Validation Gates

Before an upstream PR is considered, the prototype should show:

1. identical output and no optical preprocessing for current metric-only text;
2. deterministic PDF and PNG output across repeated runs;
3. preserved glyph ids, clusters, ligatures, and text extraction;
4. comparison results for nearest contour, fallback only, and compact guarded
   on the same InDesign baselines;
5. cold and warm runtime measurements;
6. no regression in the current no-ligature and ligature suites;
7. explicit rejection or preservation behavior for unsupported scripts,
   cross-font boundaries, vertical text, and monospaced fonts.

## Deliberate V1 Limits

- Latin display text only.
- Left-to-right runs only.
- Same-font adjacent clusters only.
- No math-layout integration.
- No raster or ML work in layout.
- No claim that InDesign Optical is ground truth.
- No automatic activation and no change to the metric default.

The current implementation enforces these limits. They keep the prototype reviewable. Broader script and layout support
belongs after the runtime boundary and performance cost are accepted.
