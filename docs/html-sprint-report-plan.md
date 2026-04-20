# HTML Sprint Report Plan

## Status

Design exploration only. This document does not change the shipped `1.0`
implementation contract. It defines a follow-on plan for richer structured
inputs and an XHTML sprint-report example/template flow.

## Goal

Deliver a genuinely useful HTML sprint report that:

- renders as a self-contained single HTML/XHTML file with inline CSS,
- shows a top-level sprint summary panel with direct links to PRs and key docs,
- scales into repeated per-sprint panels,
- proves that `sc-compose` is useful for structured report composition rather
  than only for flat markdown/file generation.

## Why This Needs Follow-on Input Work

The current `1.0` input model accepts:

- scalars,
- arrays of scalars.

That is enough for simple examples and the current markdown `/sprint-report`
skill, but it is not enough for a structured HTML report with:

- sprint rows carrying multiple named fields,
- per-sprint PR metadata,
- per-PR CI check lists,
- stage-to-icon or stage-to-class mapping.

This plan therefore starts with an input-model expansion before any HTML report
implementation.

## Sprint Sequence

### Sprint H1: Map/Object Input Support

Objective:

- allow object/map render inputs with string keys.

Scope:

- extend the `InputValue` contract to support objects,
- allow object values in `--var-file`,
- allow object values in frontmatter `defaults`,
- allow object values in user-template `template.json` `input_defaults`,
- document object access patterns such as `sprint.stage` and `pr.title`.

Validation focus:

- object parsing and validation,
- string-key enforcement,
- deterministic merge and precedence behavior,
- clear rejection of unsupported shapes.

Usable outcome:

- templates can consume one structured record instead of many flattened scalar
  variables.

### Sprint H2: Arrays Of Objects

Objective:

- allow arrays whose members are objects.

Scope:

- allow arrays of objects in the same input paths as Sprint H1,
- support loops such as `{% for sprint in sprints %}`,
- support nested object fields inside each array member,
- use a scope-tracker for loop-body discovery instead of a MiniJinja AST
  dependency,
- continue to reject nested arrays and other hard-to-govern shapes.

Validation focus:

- arrays of objects through `--var-file`,
- arrays of objects in frontmatter defaults,
- arrays of objects in `template.json` `input_defaults`,
- stable and actionable validation errors for rejected nested shapes.

Usable outcome:

- one structured input file can describe a sprint report with repeated sections
  without flattening each sprint row into unrelated scalar variables.

### Sprint H3: XHTML Sprint Report v1

Objective:

- render the current `/sprint-report` as a single self-contained HTML/XHTML
  panel that is directly viewable in a browser.

Scope:

- add one bundled example/template conceptually named
  `sprint-report-html`,
- render one top-level panel that carries:
  - report title,
  - sprint status summary,
  - PR number/title/branch,
  - DEV/QA/CI status,
  - clickable links to:
    - GitHub PR,
    - CI run logs,
    - plan doc,
    - findings doc when present,
- use inline CSS and a palette compatible with the existing
  `xhtml-plugin-expert` guidance,
- keep H3 as one flat file `examples/sprint-report-html.html.j2` with all
  markup inline,
- keep browser opening in the `/sprint-report` skill or wrapper flow rather
  than in `sc-compose` itself.

Usable outcome:

- the existing sprint-report workflow gains a clickable HTML artifact even
  before multi-panel report composition exists.

### Sprint H4: Multi-Panel XHTML Report

Objective:

- expand the single-panel report into a multi-panel report with repeated sprint
  sections.

Scope:

- top summary panel,
- repeated per-sprint panels,
- stage-sensitive panel sections,
- reusable includes/fragments for headers, summary tables, PR cards, and CI
  status callouts if a later architecture amendment expands the example beyond
  the flat single-file H3 layout.

### Deferred Next Step: Wrapper-Owned Orchestration

After H1-H4, the next logical extension is a wrapper-owned multi-render flow:

- one source JSON drives multiple `sc-compose render` calls,
- report fragments render first,
- final HTML report shell renders last,
- optional browser-open step lives in the `/sprint-report` skill or a wrapper
  script, not in `sc-compose`.

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
