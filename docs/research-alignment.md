# Research Alignment

This benchmark is not trying to clone Adobe InDesign. The goal is to test
deterministic, outline-based directions that could plausibly fit Typst.

## What the sources suggest

- Adobe describes InDesign Optical Kerning as spacing based on adjacent
  character shapes rather than predefined font data. It is useful when built-in
  kern data is sparse, mixed fonts appear in one word, or mixed sizes appear on
  one line:
  <https://helpx.adobe.com/indesign/desktop/format-and-style-text/tabs-indents-and-spacing/about-kerning-and-tracking.html>
- CreativePro describes InDesign Optical as determining spacing between all
  character pairs instead of using the font's built-in kern tables:
  <https://creativepro.com/typetalk-metrics-versus-optical-kerning/>
- Butterick's Practical Typography argues for metric spacing as the normal
  choice and optical spacing as an emergency option for fonts with bad spacing
  or fonts used beyond their designed capacity, such as body fonts used for
  headlines. It also shows why a geometric optical method can over-tighten
  rounded and asymmetric forms:
  <https://practicaltypography.com/metrics-vs-optical-spacing.html>
- FontForge AutoKern tries to estimate optical separation between two glyphs and
  then applies a kern value to reach a desired optical spacing:
  <https://fontforge.org/archive/autowidth.html>
- FontForge's metrics docs expose two useful concepts for benchmarking:
  a desired default separation and a minimum kern threshold, so tiny changes do
  not become noise:
  <https://fontforge.org/docs/ui/dialogs/lookups.html>
- `psoptkern` normalizes the area between adjacent glyphs, removes statistical
  and optical outliers, and uses robust statistics such as Median Absolute
  Deviation:
  <https://github.com/scriptituk/psoptkern>
- HalfKern is raster/blur based, so it is not a direct Typst layout-path model,
  but it contributes the useful idea of calibrating the spacing target from
  reference pairs such as `ll`, `nn`, and `oo`:
  <https://github.com/behdad/halfkern>
- "Learning to Kern" frames kerning as a full set-wise problem over many letter
  pairs and notes the lack of a general automatic criterion. Its ML approach is
  intentionally out of scope for Typst's runtime path, but its set-wise framing
  supports per-font calibration rather than isolated pair hacks:
  <https://arxiv.org/abs/2402.14313>
- Typst issue #8514 asks specifically for deriving kerning from adjacent glyph
  outlines as a fallback when font kerning is sparse or absent:
  <https://github.com/typst/typst/issues/8514>

## Benchmark rules derived from that

1. Word examples must evaluate every adjacent non-space glyph pair after text
   shaping, not merely every adjacent Unicode character.
2. Algorithms may return zero for normal pairs; they must not be forced to
   change every pair.
3. Per-font values must be read from the font or computed from its outlines.
   They should not be hand-set in the corpus manifest.
4. A Typst-plausible candidate should preserve good metric kerning and use
   optical estimates mainly as a fallback or disagreement signal.
5. Monospaced fonts should be detected dynamically and preserved by candidate
   algorithms, because optical tightening can break alignment-sensitive text.
6. Ligature substitutions must be respected. When active OpenType features
   replace a sequence such as `fi` with one glyph, that replacement glyph is
   kerned against its shaped neighbors; the internal `f-i` boundary is not a
   spacing decision anymore.
7. Raster and ML approaches can be reference baselines, but not the primary
   compiler-path proposal.
8. Optical behavior should remain opt-in and must be evaluated against
   well-spaced metric controls. A candidate that improves a weak display case
   but damages reliable native spacing is not acceptable.

## Current interpretation

The current suite uses outline profiles plus robust per-font distribution
calibration:

- x-height/cap-height are read from the font when available,
- outline gaps are sampled over a reference Latin alphabet,
- the font's median gap and MAD define the normal spacing band,
- ordinary pairs return zero optical delta,
- outliers move partway back toward the normal band,
- metric-prior candidates preserve existing font kerning and monospaced fonts.

This makes failures visible instead of hiding them. For example, pure
profile/area methods can still over-tighten some word rows, while hybrid/fallback
methods show whether a Typst-safe strategy can avoid that.
