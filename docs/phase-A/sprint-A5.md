---
id: A5
title: Template Families And Shared Panel Chrome
status: planned
---

# Sprint A5 — Template Families And Shared Panel Chrome

## Goal

Define how shared templates, includes, and panel chrome are selected and
overridden so report UI and copy behavior stop being reimplemented in each
consumer repo.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](./sprint-A1.md)
- [docs/phase-A/sprint-A4.md](./sprint-A4.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A5.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Deliverables

- one planned template-family model for at least:
  - lint/test/smoke evidence reports
  - public API / CLI / ICD style reports
  - diagram/state-machine and SQL-query reports
- one planned override contract so a repo can point a report family at
  repo-local templates without forking shared behavior
- one shared panel contract with:
  - stable panel id
  - title
  - body content
  - required copy-text action
  - optional copy-JSON action
  - optional fragment/open link
- one explicit split between shared panel chrome and consumer-specific panel
  body content

## Explicit Code Samples

```toml
[reporting.templates.diagram]
source = "shared:diagram-panels"

[reporting.templates.lint]
path = ".sc-compose/templates/lint-report.html.j2"
```

## This Sprint Does Not Close

- latest/archive output policy
- publish-manifest behavior
- bundled example migration

## Acceptance Criteria

- the plan names the initial template families and intended use cases
- the override path is explicit and does not require forking `sc-compose`
- the plan makes per-panel text copy mandatory
- the plan makes per-panel JSON copy optional but first-class
- the plan keeps panel chrome in shared template behavior rather than
  wrapper-only logic

## Required Validation

- `cargo fmt --all --check`
