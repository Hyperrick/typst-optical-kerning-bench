# Path To A Typst Prototype

This document describes what would have to happen between the current external
workbench and a focused Typst implementation proposal. It is a roadmap for
discussion, not a commitment from the Typst project.

## Current State

The repository currently provides three separate things:

1. a reproducible evaluation pipeline that validates font parity, renders Typst
   and InDesign outputs, measures differences, and produces visual review
   sheets;
2. a research algorithm, `guarded-profile-hybrid`, that works after shaping,
   keeps metric kerning as a prior, derives optical evidence from glyph
   outlines, and applies dynamic safety guards;
3. a smaller dependency-free `compact-guarded` runtime kernel and a separate
   Typst compiler prototype that inserts it after shaping.

The current Latin display-text evidence contains 30 no-ligature cases and 31
ligature-capable cases. The candidate's combined mean rendered difference from
InDesign Optical is `0.0146em`; the worst current case is `0.0304em`.

The compact kernel has also been checked against the 30-case no-ligature suite:
its mean rendered difference from InDesign Optical is `0.0200em` and its worst
current difference is `0.0432em`. The prototype reproduces the workbench's
expected word-width corrections within `0.000049em` on all 30 cases. On the
31-case ligature suite, the compact kernel reaches `0.0132em` mean and
`0.0240em` worst rendered difference and passes the same compiler-width gate.

This is enough to make the direction and implementation boundary concrete. It
is not an upstream decision, a final API, or a merge-ready patch.

## What Is Already Resolved

- Optical decisions must happen after shaping, so the algorithm sees actual
  glyph ids and clusters rather than adjacent Unicode characters.
- Active ligature and OpenType feature settings must be respected.
- Metric kerning must remain the protected prior and default behavior.
- Font-local values must be computed from the active font instance, not stored
  as per-font exceptions.
- Runtime behavior should use outlines and cached numeric profiles, not raster
  analysis or machine learning.
- Visual comparisons are only valid after Typst and InDesign metric output pass
  the same-font, same-shaping parity gate.

## Open Upstream Decisions

### Behavior

Typst would need to decide whether an optical mode:

- replaces shaped metric positioning;
- adjusts shaped metric positioning conservatively, as the current candidate
  does; or
- only acts as a fallback when no effective kerning is present.

The workbench supports comparison between these directions. The current
candidate favors conservative adjustment because it preserves good typeface
spacing and reduces the risk of making ordinary text worse.

### Metric Semantics

The word `metric` is not precise enough by itself. Font layout can involve base
advance widths, the legacy `kern` table, and GPOS pair positioning. A future API
or implementation contract must define how these sources interact, especially
when both `kern` and GPOS data are present.

The benchmark currently treats the final shaped positions produced by
Rustybuzz as the prior. A Typst prototype should start from Typst's own shaping
result so it does not introduce a second interpretation of font tables.

### API Shape

The benchmark does not settle a public API. The smallest illustrative extension
is to preserve today's boolean behavior and add an optical mode:

```typst
#set text(kerning: true)      // current behavior
#set text(kerning: false)     // current disabled behavior

// Possible future direction only:
#set text(kerning: "optical")
```

Whether Typst should instead use a full enum, an `auto` mode, or separate
control over `kern` and GPOS is an upstream design decision.

### Complexity Budget

The research implementation includes diagnostics, multiple rejected
algorithms, calibration experiments, render integration, and comparison
metrics. That breadth is useful in a workbench but should not define the final
compiler patch.

The prototype therefore does not port the whole research crate. It extracts and
measures the smallest runtime kernel found so far that preserves the
candidate's important behavior:

- shaped-glyph input;
- cached outline profiles;
- metric prior;
- font-local outlier signal;
- collision/aperture safety;
- minimal run context.

Nearest-contour distance and fallback-only behavior remain simple controls in
the benchmark rather than user-facing modes.

## Proposed Milestones

### 1. External Evaluation

**Status: complete for the current Latin display-text slice.**

The workbench establishes metric parity, compares several algorithm families,
and records visual and numeric failures. InDesign Optical is a publishing
reference rather than assumed ground truth.

### 2. Focused Typst Prototype

**Status: implemented in a separate branch for review and measurement.**

The prototype:

- reuses Typst's font, shaping, and memoization structures;
- runs after glyph substitution and positioning;
- caches profiles and pair geometry by font instance and glyph ids;
- changes no default behavior;
- supports left-to-right Latin display text first;
- preserves shaped clusters and handles ligatures as replacement glyphs.

See [`typst-integration-map.md`](typst-integration-map.md) and
[`runtime-prototype-results.md`](runtime-prototype-results.md). The concrete
compiler experiment is published as
[`Hyperrick/typst-upstream@908e895`](https://github.com/Hyperrick/typst-upstream/tree/908e89562d72a6b27fc903a67584dbc0fffca3e4),
without an upstream PR.

### 3. Upstream Design Discussion

Use [Typst issue #8514](https://github.com/typst/typst/issues/8514) to determine
whether maintainers want to explore a conservative optical adjustment, a
missing-kerning fallback, or neither. Clarify the expected complexity budget
and API constraints before opening a code PR.

### 4. Compiler Measurements

**Status: initial same-commit measurements complete.**

Measure separately:

- one-time font profile preprocessing;
- cold pair/run evaluation;
- warm cache behavior;
- normal metric-only compilation cost;
- large documents where optical mode is enabled only for headings;
- worst-case documents that enable it broadly.

The current prototype's metric PDF is byte-identical to the same-commit
baseline, and metric-only timing shows no material measured regression. The
opt-in pass adds about `48.4ms` to the median compilation time of the current
120-page, 240-heading workload after caches were split by responsibility.
These numbers are evidence for discussion, not a stable performance guarantee.

### 5. Broader Correctness Evidence

Before a merge proposal, expand the corpus beyond the current Latin display
focus. Important additions include:

- more variable-font axes and optical-size instances;
- combining marks and non-Latin shaping systems;
- mixed-font and mixed-size runs;
- symbols, math-adjacent text, punctuation, and figures;
- professional fonts with strong native spacing as preservation controls;
- intentionally sparse or broken kerning data as fallback controls.

Academic display fonts such as Libertinus and STIX are particularly useful next
cases because posters, presentation titles, and project acronyms expose sparse
pair coverage at large sizes.

### 6. Focused Upstream PR

Only after the behavior, API boundary, runtime kernel, and performance budget
are accepted should the work become a normal Typst PR. The benchmark should then
serve as external evidence and regression material rather than be copied into
the compiler repository.

## How The Community Can Help

Useful contributions at the current stage are concrete and reviewable:

- identify display words or fonts that expose a repeatable spacing failure;
- review the visual sheets and point to a specific pair or run that is worse;
- run the metric-parity gate on another machine or InDesign version;
- discuss the `kern`/GPOS and API semantics in the upstream issue;
- help reduce the runtime candidate while preserving benchmark behavior;
- propose performance workloads representative of real Typst posters,
  presentations, reports, and long documents.

General statements that one row "looks better" are still useful as design
feedback, but the workbench is most effective when the exact font, sample,
feature settings, and rendered comparison are named.
