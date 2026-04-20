# HTML Sprint Report Plan

## Status

Follow-on design exploration only. H1-H4 are shipped; this document now covers
H5-and-later work and does not change the delivered Phase HTML-Report
contract.

## Goal

Explore the next phase of the HTML sprint report so it can:

- renders as a self-contained single HTML/XHTML file with inline CSS,
- shows a top-level sprint summary panel with direct links to PRs and key docs,
- scales into repeated per-sprint panels,
- proves that `sc-compose` is useful for structured report composition rather
  than only for flat markdown/file generation.

## Shipped Baseline

Phase HTML-Report already delivered:

- H1 object/map inputs,
- H2 arrays of objects,
- H3 the bundled single-panel `sprint-report-html` example,
- H4 wrapper-owned HTML rendering integration without hook execution in
  `sc-compose`.

## Next Step Sequence

### H5: Multi-Panel XHTML Report

Objective:

- expand the shipped single-panel report into a multi-panel report with
  repeated sprint sections.

Scope:

- top summary panel,
- repeated per-sprint panels,
- stage-sensitive panel sections,
- optional reusable fragments if a later architecture amendment expands the
  example beyond the flat single-file H3 layout.

### H6: Wrapper View/Open Behavior

Objective:

- make the wrapper’s post-render viewing UX explicit without pushing it into
  `sc-compose`.

Scope:

- wrapper-owned `--open` or application-selection behavior,
- clearer separation between HTML mode selection and output viewing behavior,
- no browser-open behavior in `sc-compose` itself.

### H7: Post-Render Hook Exploration

Objective:

- evaluate whether reusable post-render behavior is worth formalizing after the
  wrapper UX settles.

Scope:

- possible post-render-hook design,
- explicit non-goal: no hook execution in `sc-composer`,
- explicit boundary: no implicit hook behavior in `sc-compose` without a later
  accepted architecture amendment.

## Proposed XHTML Template Structure

Initial H3 structure:

- `sprint-report-html.html.j2`
  - outer document shell
  - inline CSS
  - top summary panel
  - optional repeated sprint summary rows

Follow-on include fragments, deferred until H4 or a later architecture
amendment:

- `_includes/report-head.html.j2`
- `_includes/summary-table.html.j2`
- `_includes/pr-card.html.j2`
- `_includes/check-list.html.j2`
- `_includes/stage-badge.html.j2`

H3 intentionally keeps all markup in one flat file. Multi-panel expansion is
where `_includes/` begins to add clear value, and that layout change must be
documented explicitly before implementation.

## Proposed Example Input Shape

Target post-H2 input shape:

```json
{
  "report": {
    "title": "Sprint Status",
    "generated_at": "2026-04-20T00:00:00Z",
    "plan_url": "https://github.com/org/repo/blob/main/docs/project-plan.md",
    "findings_url": "https://github.com/org/repo/blob/main/docs/findings.md"
  },
  "sprints": [
    {
      "id": "S7",
      "title": "Examples and templates",
      "stage": "qa_pass",
      "branch": "feat/examples-command",
      "pr": {
        "number": 32,
        "title": "Add examples and templates support",
        "url": "https://github.com/org/repo/pull/32"
      },
      "ci_status": "pass",
      "ci_url": "https://github.com/org/repo/actions/runs/123"
    }
  ]
}
```

This shape is the main reason the follow-on input work matters. The current
scalar-plus-array-of-scalars model forces most of this structure to be flattened
into prebuilt HTML or markdown strings.

## Why This Is A Good `sc-compose` Showcase

This is a strong showcase if the structured-input work lands because it proves:

- one template system can produce both markdown and rich HTML artifacts,
- include-based composition works for UI/report fragments as well as prompt
  assets,
- structured inputs make `sc-compose` practical for higher-value generated
  outputs, not just simple string substitution,
- the same report can stay deterministic and version-controlled while still
  being clickable and visually useful.

Without the structured-input work, the HTML report would still be possible, but
it would mostly be a thin wrapper around precomputed HTML strings. That is less
compelling and does not demonstrate `sc-compose` at its best.

## Explicit Non-Goals For This Track

- browser-opening logic in `sc-compose` itself,
- hook execution inside `sc-compose`,
- external JavaScript/CSS dependencies,
- server-side report hosting requirements.
