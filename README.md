# Typst Optical Kerning Bench

Rust-first benchmark suite for evaluating optical kerning algorithms against
Typst-rendered PDFs and optional Adobe InDesign baselines.

The goal is not to patch Typst directly. The goal is to produce reproducible
evidence: pinned fonts, critical pairs, algorithmic deltas, rendered PDFs,
performance data, and reports that can support a future Typst RFC or PR.

## Quick Start

```sh
cargo run -p optikern-cli -- fetch-fonts
cargo run -p optikern-cli -- bench
cargo run -p optikern-cli -- render-typst
cargo run -p optikern-cli -- report
cargo run -p optikern-cli -- survey
```

Outputs are written to `metrics/`, `renders/`, and `reports/`.

## InDesign Baselines

Generate the InDesign ExtendScript and sidecar data:

```sh
cargo run -p optikern-cli -- render-indesign
```

Then run it from InDesign's Scripts panel or execute:

```sh
osascript scripts/run-indesign-export.scpt "$(pwd)/renders/indesign/export-baselines.jsx"
```

The generated document uses fixed A4 pages, black text on white background,
tracking `0`, horizontal/vertical scale `100`, no hyphenation, and exports both
`$ID/Metrics` and `$ID/Optical` PDFs. It also exports
`indesign-comparison.pdf`, a side-by-side visual sheet with Metrics and Optical
columns for pairs and real words.

See [`docs/indesign-baseline.md`](docs/indesign-baseline.md) for the exact
document construction rules.

## Human Preference Study

Generate a blind five-way click suite:

```sh
cargo run -p optikern-cli -- survey
open reports/survey.html
```

The survey is a fast subjective screening layer. It renders samples as inline
SVG paths from the pinned font outlines, submits vote sessions, and helps
compare human preference against algorithm simplicity, runtime cost, and
PDF/InDesign evidence. See [`docs/preference-study.md`](docs/preference-study.md).

The same command also writes a GitHub Pages-ready static bundle to `site/`:

```sh
open site/index.html
```

To enable central persistence and point the public page back to the source repo,
pass a Cloudflare Worker submit endpoint and repository URL when generating the
site. If the submit endpoint ends in `/submit`, the public results endpoint is
derived as `/results`:

```sh
cargo run -p optikern-cli -- survey \
  --submit-endpoint https://typst-optical-kerning-bench.hyperrick.workers.dev/submit \
  --repo-url https://github.com/Hyperrick/typst-optical-kerning-bench
```

Use `--results-endpoint` only when the public aggregate endpoint does not live
next to `/submit`.

The intended public URL is
<https://hyperrick.github.io/typst-optical-kerning-bench/>. The Pages workflow
publishes `site/` and includes:

- `index.html`: the survey,
- `methods.html`: algorithm and repository notes,
- `results.html`: public aggregate results from the configured Worker.

Progress is stored locally in each browser; completed sessions are submitted to
the configured endpoint.
See [`docs/github-pages.md`](docs/github-pages.md) and
[`docs/data-persistence.md`](docs/data-persistence.md). The Cloudflare Worker
setup and reset flow are documented in
[`docs/cloudflare-worker.md`](docs/cloudflare-worker.md); the reset call is also
available as `scripts/reset-cloudflare-db.sh`.

## Algorithms

Implemented V1 algorithms:

- `nearest-contour-distance`
- `profile-whitespace`
- `area-balance`
- `metric-prior-hybrid`
- `safe-fallback-only`

See [`docs/algorithms.md`](docs/algorithms.md) for the current heuristics and
the constraints that make them plausible for a future Typst implementation.

## Design Constraints

- Deterministic algorithms only.
- No ML or runtime raster analysis in the layout path.
- Deltas are emitted as `em` values and simulated in Typst with `#h(...)`.
- InDesign Optical is a comparison baseline, not an absolute truth.
