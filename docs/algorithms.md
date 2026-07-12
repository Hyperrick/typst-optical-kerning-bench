# Algorithm Notes

All implemented candidate algorithms operate on font outlines, not raster
images. Raster analysis is used only after rendering PDFs to evaluate output.

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
- `compact-guarded`: extracts the metric prior, collision/aperture bounds,
  dynamic side-shape classification, and minimal run context into a
  dependency-free runtime kernel. The workbench computes its outline evidence;
  the kernel only decides the final deterministic delta.
- `safe-fallback-only`: preserves metric kerning if it exists; otherwise uses
  the robust optical outlier correction. Monospaced fonts are preserved.

## Candidate Selection

The guarded candidate emerged from comparing these approaches rather than from
choosing one formula upfront.

The simpler outline algorithms were useful as probes:

- `nearest-contour-distance` made real collisions visible and fixed some close
  contour cases, but nearest distance alone cannot model stem rhythm or word
  color.
- `profile-whitespace` captured broad optical whitespace better, but profile
  means can be inflated by apertures, counters, and diagonal shapes.
- `area-balance` reduced some outliers but still behaved like a single global
  spacing criterion.
- `metric-prior-hybrid` showed that font kerning should be a prior rather than
  something to replace wholesale.
- `safe-fallback-only` established the conservative lower-risk baseline for
  sparse or missing metric kerning.

`guarded-profile-hybrid` remains the research reference because it keeps the
useful parts of those tests and adds the missing safety model: metric kerning
remains the base signal, outline profiles provide optical evidence,
nearest-contour checks catch local danger, and run context prevents
individually plausible pair decisions from accumulating into a poor word.

`compact-guarded` is the compiler-facing candidate. It deliberately accepts a
small quality tradeoff to separate the host's font geometry and caching from a
reviewable pair/run decision kernel. This is why the result remains a guarded
hybrid rather than a pure nearest-distance or pure whitespace rule.

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

## V21 Sans Lowercase Run Notes

V21 adds a narrow run-context correction for sans-like lowercase words after
shaping. The trigger is dynamic: the run must look sans-like, consist only of
letter pairs, contain at least five lowercase pairs, and have mostly metricless
lowercase spacing. It does not check font names or sample strings.

This specifically fixes the ligature-suite failures where Comic Neue and Inter
lowercase words accumulated small pair errors across the whole word. Compact
sans runs can be opened when no pair was optically tightened, while severe local
tightening in compact runs can be relaxed back toward a safer target. Noncompact
sans runs keep strong metric kerning intact and only adjust metricless bridges
by a small amount.

These corrections are deliberately small per pair, but they matter over a full
word. For that reason the rule returns the raw run correction and lets the final
pair-normalization step happen once at the end. The no-ligature five-font suite
remained unchanged from V15, so this pass improves the ligature path without
moving the established no-ligature behavior.

## V22 Serif And Script Run Notes

V22 adds two dynamic corrections for the remaining ligature-suite outliers.

The first correction handles wide-serif lowercase runs with ligature clusters.
When a shaped word has a wide serif spacing profile, at least six lowercase
pairs, at least one multi-character cluster, mostly metricless lowercase pairs,
and no connected lowercase collisions, safe lowercase bridges can be tightened
slightly more. Entries into a right-side ligature cluster are excluded, and
pairs with a small local minimum gap are left alone. This fixes the Libre
Baskerville `efficient` case without naming the font or word.

The second correction handles short connected-script lowercase runs. The script
lowercase run threshold is four adjacent pairs instead of five, so short words
such as `fjord` can receive the same conservative compaction already used for
longer connected script runs. The trigger still requires a connected script-like
profile, metricless lowercase pairs, near-continuous connected gaps, and no
profile request to open the pair.

V22 also neutralizes small positive metric openings in wide serif lowercase
runs when the unkerned local contour gap is already near-touching. This lets the
guard override a font metric opening only in a narrow situation where InDesign
Optical also behaves more tightly.

## V23 Long Script Ligature Run Notes

V23 narrows the connected-script ligature handling for one shape class: long
fully connected lowercase script runs with multi-character clusters, no metric
tightening, and no optical request to open the pair. Earlier connected-script
rules opened these runs uniformly, which was too strong for long words where
InDesign Optical keeps the connected baseline relatively compact.

The rule is still dynamic. It requires at least six adjacent letter pairs, all
letter pairs to be connected, no metric-tightened letter pairs, and no
outline-profile opening signal. When those conditions hold, the script
ligature-run correction is capped to a smaller positive delta. This fixed the
remaining Lobster `efficient` over-opening without changing the no-ligature
suite or the shorter connected-script controls.

## V24 Short Serif Ligature Word Notes

V24 adds a defensive rule for short wide-serif lowercase words that shape into
one small ligature cluster. In those words, a generic "safe compaction" target
can make a compact bridge tighter even when the robust outline gap is already
well below the font-local target. The visible failure was Libre Baskerville
`final`, where the `n|a` bridge was tightened although InDesign Optical keeps
the word more open.

The trigger is deliberately narrow: the run must be wide-serif, consist of two
or three lowercase pairs, contain at least one multi-character cluster, have a
maximum cluster length of two characters, be fully metricless, and have no
connected lowercase collisions. The pair-level correction only neutralizes a
small negative delta when the robust gap is already compact. This improves the
short `fi` word case without changing the no-ligature suite or longer `ffi`
words such as `office`.

## V25 Serif Mixed-Case Gap Notes

V25 tightens one remaining no-ligature failure class without touching the
ligature suite. The target case was EB Garamond `ToTaL`, where the word was
still visibly wider than InDesign Optical after V24.

The rule is dynamic and limited to serif-like mixed-case geometry. When a
font-local serif profile has a clear robust gap excess, an upper-to-round-lower
pair with strong metric kerning is allowed to keep the full metric value instead
of being softened. A metricless round-lower-to-upper overhang pair in the same
shape class can also close slightly beyond the previous cap. This affects the
`T|o` and `o|T` shape class without naming the font or sample.

In the V25 no-ligature suite, only EB Garamond `ToTaL` changed:

```text
score: 0.0384em -> 0.0168em
width: -0.0384em -> -0.0168em
ink:   0.0294em -> 0.0159em
```

The V25 ligature suite stayed unchanged from V24.

## Guarded Constraint Model

The guarded candidate is intentionally not a single lightweight heuristic. It is
a small decision pipeline that separates responsibilities:

1. A metric-prior base delta decides the first candidate from metric kerning and
   outline-profile spacing.
2. Hard bounds protect against unsafe movement. These bounds can require at
   least the metric delta, require no tightening when apertures are already
   critical, open local collisions, or cap false diagonal openings back to zero.
3. Additive tightening targets handle safe optical improvements such as
   round-to-overhang gaps, uppercase punctuation, digit side shapes, and
   sans-like lowercase/run-context spacing.

This is implemented as a `DeltaPlan`: each rule no longer mutates an
unstructured `adjusted += ...` value. Instead, rules either tighten the desired
delta, raise the lower bound, or lower the upper bound. The final delta is
clamped and normalized once. The important behavior from earlier iterations is
preserved: safe tightening targets may still stack sequentially, but safety
bounds remain centralized and can override optical nudges.

This shape is closer to what a Typst-side implementation would need: the
algorithm stays deterministic, cacheable, and inspectable, while still admitting
that optical kerning needs several interacting guards rather than one universal
formula.

## Compact Runtime Boundary

`crates/optikern-runtime` contains no font parser, shaper, image library, PDF
tooling, CLI, or Typst dependency. Its host supplies normalized `PairEvidence`
for adjacent shaped glyphs. The kernel then provides three directly comparable
decisions:

- `nearest_contour`, the intentionally naive geometric control;
- `fallback_only`, the conservative control that preserves nonzero metric
  positioning;
- `compact_guarded`, the current extracted pair and run decision.

The pair/run decision is roughly 660 nonblank lines before tests. It contains
no font names, sample strings, glyph ids, raster checks, or learned model. A
release microbenchmark over eight representative evidence records currently
measures about `10ns` per warm compact pair on the development machine. Outline
flattening, profile sampling, calibration, and cache misses are measured
separately because they dominate real cold-path cost.

## Typst Compatibility Constraints

These implementations deliberately favor:

- deterministic output,
- no ML model in the layout path,
- no rasterization in the layout path,
- small state that can be cached per font and glyph pair,
- dynamic values read from or computed from the font, not hand-set per font,
- deltas that can be represented as `em` advances after shaping.
