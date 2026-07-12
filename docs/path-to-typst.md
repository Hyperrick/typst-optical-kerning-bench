# Path To A Typst Prototype

This document describes what would have to happen between the current external
workbench and a focused Typst implementation proposal. It is a roadmap for
discussion, not a commitment from the Typst project.

## Current State

The repository currently provides two separate things:

1. a reproducible evaluation pipeline that validates font parity, renders Typst
   and InDesign outputs, measures differences, and produces visual review
   sheets;
2. an algorithm candidate, `guarded-profile-hybrid`, that works after shaping,
   keeps metric kerning as a prior, derives optical evidence from glyph
   outlines, and applies dynamic safety guards.

The current Latin display-text evidence contains 30 no-ligature cases and 31
ligature-capable cases. The candidate's combined mean rendered difference from
InDesign Optical is `0.0146em`; the worst current case is `0.0304em`.

This is enough to make the direction concrete. It is not enough to copy the
research implementation into Typst unchanged.

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

The next implementation task is therefore not "port the whole crate." It is to
extract and measure the smallest runtime kernel that preserves the candidate's
important behavior:

- shaped-glyph input;
- cached outline profiles;
- metric prior;
- font-local outlier signal;
- collision/aperture safety;
- minimal run context.

Nearest-contour distance and fallback-only behavior should remain available as
simple controls in the benchmark, even if they are not the final user-facing
mode.

## Proposed Milestones

### 1. Upstream Design Discussion

Use [Typst issue #8514](https://github.com/typst/typst/issues/8514) to determine
whether maintainers want to explore a conservative optical adjustment, a
missing-kerning fallback, or neither. Clarify the expected complexity budget
and API constraints before opening a code PR.

### 2. Focused Typst Prototype

Build a deliberately small, opt-in prototype against Typst's real shaped glyph
run. The first prototype should:

- reuse Typst's font and shaping structures;
- run after glyph substitution and positioning;
- cache font profiles by font instance and variation coordinates;
- expose no new default behavior;
- support a very small Latin display-text slice first;
- be easy to remove or revise while the design is still unsettled.

This prototype can live in a branch or draft PR for measurement without
claiming that the public API is final.

### 3. Compiler Measurements

Measure separately:

- one-time font profile preprocessing;
- cold pair/run evaluation;
- warm cache behavior;
- normal metric-only compilation cost;
- large documents where optical mode is enabled only for headings;
- worst-case documents that enable it broadly.

Metric-only documents should not pay the optical preprocessing cost.

### 4. Broader Correctness Evidence

Before a merge proposal, expand the corpus beyond the current Latin display
focus. Important additions include:

- more variable-font axes and optical-size instances;
- combining marks and non-Latin shaping systems;
- mixed-font and mixed-size runs;
- symbols, math-adjacent text, punctuation, and figures;
- professional fonts with strong native spacing as preservation controls;
- intentionally sparse or broken kerning data as fallback controls.

### 5. Focused Upstream PR

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
