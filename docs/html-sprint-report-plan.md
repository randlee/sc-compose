# SC-Reporting Follow-On Plan

## Status

Planning line only. H1-H4 are shipped. This document covers the reusable
reporting line that follows the shipped single-panel HTML example and does not
change the delivered `1.0` contract until a later review accepts it.

## Goal

Lay down one reusable reporting pattern for `sc-compose` consumers so repos can
add lint, test, smoke, diagram, and custom publishable reports without
reinventing report layout, output policy, or handoff conventions.

The follow-on line must support:

- report generation through the repo's domain `just` recipes such as
  `just lint`, `just test`, `just smoke`, and repo-specific custom recipes,
- a shared evidence contract for generated artifacts and metadata,
- one stable latest output plus timestamped archive copies where the producer
  recipe enables them,
- one shared `just reports` surface for aggregation, verification, and
  opening/viewing,
- reusable templates and panel chrome where they add value,
- reusable diagram/state-machine and SQL-query reporting patterns across many
  repos,
- future renderer changes without keeping Mermaid as the long-term semantic
  source of truth.

## Shipped Baseline

Phase HTML-Report already delivered:

- H1 object/map inputs,
- H2 arrays of objects,
- H3 the bundled single-panel `sprint-report-html` example,
- H4 wrapper-owned HTML rendering integration without hook execution in
  `sc-compose`.

## Follow-On Rules

- Producer recipes own report generation. `just lint`, `just test`,
  `just smoke`, and repo-specific producer recipes generate their own evidence.
- `just reports` is an aggregator and verifier, not the primary producer.
- Authored docs and generated evidence stay separate:
  - `docs/` for authored policy and design notes
  - report specs/templates/catalogs under a report-specific tree
  - generated latest/archive outputs under generated-evidence paths
- The report contract must allow repo-specific custom reports without changing
  the shared aggregation pattern.
- GitHub issue `#56` is in-scope for the follow-on line as the generic
  source-collection and render-many capability, but Mermaid-as-SSOT is treated
  as transitional rather than the long-term semantic end state.
- Network publish behavior and browser-open behavior remain outside
  `sc-composer` and `sc-compose`.

## Phase A Sprint Sequence

The authoritative sprint order for this line is the Phase A plan in
[docs/phase-A/phase-A-plan.md](phase-A/phase-A-plan.md):

1. `A1` report artifact contract and catalog
2. `A2` producer-recipe and `just` command contract
3. `A3` source-collection, metadata-extraction, and render-many contract
4. `A4` semantic diagram-spec contract
5. `A5` template-family and shared panel-chrome contract
6. `A6` latest/archive output policy and `just reports` aggregator contract
7. `A7` publish-manifest and CI handoff contract
8. `A8` cross-use-case proof through multiple report families
9. `A9` `sc-observability` `1.1.0` adoption for report-producing CLI flows

## Output Direction

The follow-on line should converge on a shared evidence shape with:

- a report catalog/manifest
- source specs and templates separated from generated outputs
- one latest artifact location per report
- optional timestamped archive outputs
- one machine-readable sidecar per generated report
- one machine-readable handoff for downstream publication tooling

## Example Consumer Shapes

See [docs/phase-A/phase-A-plan.md](./phase-A/phase-A-plan.md) `## Cross-Use-Case Proof Shape` for the canonical minimum proof families and shared custom-producer rule.

## Explicit Non-Goals

- browser-opening logic inside `sc-compose`
- hook execution inside `sc-composer`
- network upload or hosting behavior inside `sc-compose`
- locking the long-term diagram source model to Mermaid text

## Retained HTML-Specific Context

The reporting line is intentionally broader than the original HTML-report
follow-on, but the earlier HTML-specific exploration still provides useful
example direction.

### HTML-Specific Next Steps

- H5: multi-panel XHTML report expansion with repeated sprint panels
- H6: wrapper-owned view/open behavior without moving browser logic into
  `sc-compose`
- H7: post-render-hook exploration only after wrapper UX stabilizes

Multi-panel composition is no longer treated as sprint-report-only behavior.
The same panel-shell and repeated-rendering model must also support
state-machine, SQL-query, lint, test, smoke, and future custom report
families.

### Proposed XHTML Template Structure

Initial H3 structure:

- `sprint-report-html.html.j2`
  - outer document shell
  - inline CSS
  - top summary panel
  - optional repeated sprint summary rows

Follow-on include fragments, deferred until a later accepted architecture
amendment:

- `_includes/report-head.html.j2`
- `_includes/summary-table.html.j2`
- `_includes/pr-card.html.j2`
- `_includes/check-list.html.j2`
- `_includes/stage-badge.html.j2`

H3 intentionally keeps all markup in one flat file. Multi-panel expansion is
where `_includes/` begins to add clear value, and that layout change must be
documented explicitly before implementation.

### Example Structured Input Shape

```json
{
  "report": {
    "title": "Sprint Status",
    "generated_at": "2026-04-20T00:00:00Z",
    "plan_url": "https://github.com/org/repo/blob/main/docs/project-plan.md",
    "findings_url": "https://github.com/org/repo/blob/main/docs/html-sprint-report-plan.md"
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

This example remains useful because it shows why the structured-input work is
valuable: the current scalar-plus-array-of-scalars model forces most of this
shape to be flattened into prebuilt HTML or markdown strings.

### Why The HTML Example Still Matters

- one template system can produce both markdown and rich HTML artifacts
- include-based composition works for UI/report fragments as well as prompt
  assets
- structured inputs make `sc-compose` practical for higher-value generated
  outputs, not just simple string substitution
- the same report can stay deterministic and version-controlled while still
  being clickable and visually useful
