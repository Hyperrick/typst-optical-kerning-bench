# Glyph Shape Parity

Glyph shape parity is the first check before any word-width or optical kerning
comparison.

The goal is simple: InDesign and Typst must render the same individual glyph
shapes from the same nominal font setup before the benchmark treats a word-level
difference as kerning evidence.

## Command

```sh
scripts/run-glyph-shape-parity.py \
  --fonts eb-garamond,libre-baskerville,inter \
  --glyphs G,o,l,d,f,i,s,h \
  --point-size 100 \
  --output renders/glyph-shape-parity/goldfish-glyphs-100pt-no-ligatures \
  --baseline-output baselines/glyph-shape-parity-v1.json
```

## Settings

Both engines are rendered with:

- `100pt` text size by default.
- kerning disabled.
- ligatures disabled.
- OpenType `liga` and `clig` disabled in Typst.
- InDesign text converted to outlines and fitted to visible bounds.
- Typst rendered to PNG and cropped to ink bounds.

## Outputs

For each font and glyph, the script writes:

- cropped InDesign glyph PNG.
- cropped Typst glyph PNG.
- raw overlay without scaling.
- height-normalized overlay as a diagnostic helper.
- JSON sidecar with crop bounds and overlap statistics.

The suite-level outputs are:

- `summary.json`
- `index.html`
- `contact-sheet.png`
- optional compact baseline JSON under `baselines/`

## Interpretation

Use the raw overlay first. If raw single-glyph overlays are visibly different,
the problem is not kerning yet. It is likely one of:

- InDesign selected a different installed font.
- InDesign and Typst selected different variable-font axes.
- One engine applied a different font instance or style.
- Export/rendering scale is not equivalent.

Only after glyph shapes match should the benchmark proceed to:

```text
glyph shape parity
-> unkerned word parity
-> metric kerning parity
-> optical kerning comparison
```
