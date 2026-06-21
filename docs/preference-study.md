# Human Preference Study

Optical kerning is partly subjective. The benchmark should therefore separate
three questions:

1. Does an algorithm behave deterministically and cheaply enough for Typst?
2. Does it avoid mechanical failures such as collisions or collapsed words?
3. Do people with typography judgment prefer it in blind comparisons?

The `survey` command addresses the third question. It generates a static
browser-based five-way A-E choice suite:

```sh
cargo run -p optikern-cli -- survey
open reports/survey.html
```

The command also writes a publishable static bundle:

```sh
open site/index.html
```

`site/` contains `index.html`, `methods.html`, `results.html`, `.nojekyll`,
and a short README. That directory can be used as the GitHub Pages artifact.

The browser suite is intentionally a pre-screening tool. It embeds SVG paths
generated from the pinned font outlines and applies the same pairwise `em`
deltas that the Typst sheets simulate with `#h(...)`. This avoids browser text
layout for the samples and keeps the survey deterministic, but it is still not
a replacement for the PDF/InDesign comparison.

## Study Design

- Trials are blind by default: users see A-E, not algorithm names.
- The suite includes pairs and real words.
- Choices include only candidate optical methods by default:
  `nearest-contour-distance`, `profile-whitespace`, `area-balance`,
  `metric-prior-hybrid`, and
  `safe-fallback-only`.
- The submitted result contains the hidden modes, font, sample, vote, and
  timestamp.
- Algorithm labels can be revealed during debugging, but should remain hidden
  for real preference collection.

The default study is deliberately small for each participant: the generated
pool has one five-way trial per selected sample, and each browser receives a
balanced random subset of 30.

## GitHub Pages Collection Model

GitHub Pages can host the suite, but it cannot store votes by itself. The page
autosaves progress locally and submits completed sessions to the configured
endpoint. `results.html` reads public aggregate results from the configured
results endpoint. For central collection, generate the site with
`--submit-endpoint` and point it at the Cloudflare Worker. See
`docs/data-persistence.md`.

## How This Helps Typst Maintainers

The result should not be "algorithm X won a taste contest, merge it." Instead,
it should help narrow the direction:

- `safe-fallback-only`: lowest behavioral risk because it only acts when metric
  kerning is absent or near zero.
- `metric-prior-hybrid`: strongest architecture if tuned carefully; it respects
  font kerning first and applies optical correction only when the metric answer
  is missing or visibly weak.
- `nearest-contour-distance`: currently the most conservative visual baseline
  because it produces smaller deltas and fewer collapsed words.
- `profile-whitespace` and `area-balance`: useful as stress tests, but the V1
  constants are too aggressive for real words.

For Typst, the most convincing evidence is a triangle:

- human preference from blind click tests,
- PDF/InDesign visual examples for print-like review,
- compiler-facing cost model: deterministic code, outline-only operation,
  profile cache per font/glyph, and bounded pair lookup.

Only an algorithm that does reasonably well in all three dimensions should be
presented as a serious direction for Typst.
