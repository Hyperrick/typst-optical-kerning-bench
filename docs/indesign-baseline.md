# InDesign Baseline Construction

The InDesign export is intentionally boring. The point is to compare kerning,
not page design.

## Document

- Page: A4, portrait, `210mm x 297mm`.
- Facing pages: disabled.
- Margins: `10mm` on every side.
- Background: white.
- Text: black.
- Export: `ExportFormat.PDF_TYPE` without selecting a localized preset, so the
  script works across InDesign installations.
- Outputs:
  - `renders/indesign/indesign-metrics.pdf`
  - `renders/indesign/indesign-metrics.json`
  - `renders/indesign/indesign-optical.pdf`
  - `renders/indesign/indesign-optical.json`
  - `renders/indesign/indesign-comparison.pdf`
  - `renders/indesign/indesign-comparison.json`

## Text Frames

Each test case is placed into a fixed text frame. The sidecar JSON records:

- case kind: `pair`, `word`, or `paragraph`
- font id and family
- sample text
- point size
- page number
- frame ROI in points: `[top, left, bottom, right]`

Pair frames use `48pt`, word frames use `42pt`, and paragraph frames use
`12pt`. Pair tests disable ligatures so `fi`, `fl`, and similar pairs remain
visible as pair-spacing cases. Word and paragraph baselines should preserve the
renderer's normal shaping behavior: if InDesign or Typst substitutes a ligature,
the comparison treats that ligature as the glyph to kern against its neighbors,
not as separate internal letters.

The comparison PDF uses one page per font with fixed side-by-side columns:

- sample label
- InDesign Metrics
- InDesign Optical

The selected comparison rows use a deterministic per-font rotation: four pair
examples and six word examples per font. This keeps the InDesign visual baseline
compact while still covering different categories.

The JSX computes frame bounds relative to `page.bounds`, not spread origin.
This avoids alternating blank pages or overlapped frames when InDesign's spread
coordinate system would otherwise affect even and odd pages differently.

## Kerning Modes

Two separate PDFs are exported:

```js
story.kerningMethod = "$ID/Metrics";
story.kerningMethod = "$ID/Optical";
```

All frames also set:

```js
story.tracking = 0;
story.horizontalScale = 100;
story.verticalScale = 100;
story.hyphenation = false;
story.justification = Justification.LEFT_ALIGN;
```

## Fonts

The CLI copies fetched fonts into `renders/indesign/Document fonts/` when it
generates the JSX. The script first tries the requested Google Font family. If
InDesign cannot see that font, it falls back to local system fonts:
`Times New Roman`, `Arial`, or `Courier New`. For exact font-matched baselines,
install the fetched fonts from `corpus/fonts/` manually and rerun the script.

The Google Fonts commit is pinned in `corpus/fonts.toml`; rerun
`optikern fetch-fonts --force` to refresh local copies from the pinned source.
