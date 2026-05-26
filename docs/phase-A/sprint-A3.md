---
id: A3
title: Source Collection, Metadata Extraction, And Render-Many
status: planned
---

# Sprint A3 — Source Collection, Metadata Extraction, And Render-Many

## Goal

Define the generic source-driven rendering contract from GitHub issue `#56` so
text assets with embedded metadata can generate one artifact per source plus
aggregate pages without custom wrapper scripts per repo.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](./sprint-A1.md)
- [docs/phase-A/sprint-A2.md](./sprint-A2.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A3.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/html-sprint-report-plan.md`

## Deliverables

- one collection-input contract for discovering source files by glob or other
  stable collection definition
- one metadata-extraction contract for at least:
  - comment-prefix metadata
  - block-comment metadata
  - body/raw source access
- one render-many contract for one output per source file
- one generated manifest contract that aggregate templates and review tooling
  can consume
- one explicit statement that these collection capabilities are generic and are
  not Mermaid-only

## Explicit Code Samples

```json
{
  "source_path": "docs/atm/diagrams/atm-list.mmd",
  "output_path": "reports/latest/state-diagrams/panels/atm-list.xhtml",
  "stem": "atm-list",
  "meta": {
    "title": "`atm list`",
    "sets": ["cli", "query"]
  }
}
```

## This Sprint Does Not Close

- semantic diagram-spec design
- shared panel chrome/copy behavior
- latest/archive output policy
- publish-manifest behavior
- bundled example migration

## Acceptance Criteria

- the plan covers collection discovery, metadata extraction, render-many, and
  manifest output as one coherent source-driven contract
- the plan makes source body and parsed metadata available without external
  scripting
- the plan keeps the mechanism generic across Mermaid, SVG, and other text
  assets
- the plan keeps browser automation and site hosting out of scope

## Required Validation

- `cargo fmt --all --check`
