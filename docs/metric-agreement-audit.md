# Font Metric Agreement Audit

InDesign Optical is a useful publishing reference, but it is not ground truth.
This audit provides a second, independent control: compare the compact optical
candidate with effective kerning already shipped by many fonts.

## Method

The audit uses the 15 Google Fonts entries currently available in the pinned
corpus. For every ordered pair in a defined 82-character Latin, figure, and
punctuation set, Rustybuzz shapes the pair twice with ligatures and contextual
alternates disabled:

1. kerning enabled;
2. kerning disabled.

The advance difference is the `availableKernEm` value. It represents the
effective pair positioning visible to Typst after shaping, whether that
positioning came from GPOS, the legacy `kern` table, or their shaping-engine
interaction. Rustybuzz does not expose reliable table provenance for this
result, so the report does not pretend to classify it further.

Only pairs with an effective font correction of at least `0.0001em` are kept.
The same shaped glyph ids are passed to `compact-guarded`; its final pair delta
is `calculatedKernEm`.

## Initial Result

- candidate combinations evaluated: `100,860`;
- effective font-kerning pairs retained: `15,271`;
- mean absolute candidate-versus-font difference: `0.0206em`;
- median: `0.0130em`;
- 95th percentile: `0.0800em`;
- maximum: `0.2720em`;
- sign changes: `659` pairs.

This does not prove that every font pair is correct. It does show where the
candidate contradicts existing font design decisions and where manual review
has the highest value.

## Bounded-Prior Result

The audit directly led to a generic preservation rule. Existing nonzero font
positioning can no longer change sign, and optical movement around it is
dynamically limited to at most `0.030em`. The same audit then produces:

- mean absolute difference: `0.0095em`;
- median: `0.0105em`;
- 95th percentile: `0.0300em`;
- maximum: `0.0300em`;
- sign changes: `0` pairs.

No font name, glyph name, glyph id, or sample-specific exception was added.
The full visual and compiler trade-off is documented in
[`metric-preservation-results.md`](metric-preservation-results.md).

## Initial Top-100 Review

The 100 largest differences are not random:

| Observation | Count |
| --- | ---: |
| Pacifico | 82 |
| Positive available font kerning | 89 |
| Punctuation as the right glyph | 86 |
| Candidate changes the sign | 62 |

The largest rows are mostly Pacifico capitals before closing punctuation,
slashes, and dashes. The font deliberately opens many of these pairs with
positive positioning, while the candidate sometimes applies negative optical
spacing. The remaining 18 rows are concentrated in Merriweather, Comic Neue,
Libre Baskerville, Oswald, and Source Sans 3. Several begin with a very small
font correction and are pulled toward the candidate's general `-0.16em` lower
bound.

This was a concrete preservation failure, not a reason to discard outline
evidence. It narrowed the design choice to:

- make nonzero shaped font positioning a stronger prior and limit how far an
  optical correction may move away from it; or
- ship a missing-kerning-only fallback first and accept weaker agreement with
  the InDesign display reference.

The implemented bounded-prior variant follows the first direction without
becoming a missing-only fallback: metricless pairs still receive the full
optical decision, while metricful pairs receive a bounded adjustment.

## Files

- [`metric-agreement-audit-v1.tsv`](../baselines/metric-agreement-audit-v1.tsv): all retained pairs;
- [`metric-agreement-audit-v1-top100.tsv`](../baselines/metric-agreement-audit-v1-top100.tsv): manual review queue;
- [`metric-agreement-audit-v1.json`](../baselines/metric-agreement-audit-v1.json): metadata, distributions, per-font counts, and the top 100;
- [`metric-agreement-audit-v2.tsv`](../baselines/metric-agreement-audit-v2.tsv): all rows after bounded preservation;
- [`metric-agreement-audit-v2-top100.tsv`](../baselines/metric-agreement-audit-v2-top100.tsv): bounded-prior review queue;
- [`metric-agreement-audit-v2.json`](../baselines/metric-agreement-audit-v2.json): V2 metadata and distribution.

Each TSV row contains:

```text
fontFileId | fontFamily | fontPath | leftCharacter | rightCharacter |
leftGlyphId | rightGlyphId | leftGlyphName | rightGlyphName |
availableKernEm | calculatedKernEm | differenceEm | absDifferenceEm |
signChanged
```

Reproduce the audit with:

```sh
cargo run --release -p optikern-cli -- metric-audit \
  --output baselines/metric-agreement-audit-v2 \
  --top 100
```
