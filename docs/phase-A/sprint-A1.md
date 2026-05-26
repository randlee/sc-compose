---
id: A1
title: Report Artifact Contract And Catalog
status: planned
---

# Sprint A1 — Report Artifact Contract And Catalog

## Goal

Define the reusable report artifact contract and report catalog so every repo
can generate evidence into the same filesystem and metadata shape before any
consumer-specific templates or panel UI are planned.

## Hard Dependencies

- [docs/project-plan.md](../project-plan.md)
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/html-sprint-report-plan.md](../html-sprint-report-plan.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A1.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Deliverables

- one planned report artifact contract that defines:
  - source/catalog locations
  - generated latest output locations
  - generated archive output locations
  - per-report metadata sidecars
- one planned report catalog manifest listing report ids, kinds, entrypoints,
  and producer ownership
- one clear ownership split:
  - producer recipes own data gathering and report generation
  - `sc-compose` owns rendering semantics where it is used
  - consumer repos own domain-specific inputs and publish surfaces
- one explicit statement that authored docs and generated evidence stay in
  separate trees

## Explicit Code Samples

```toml
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"

[[report]]
id = "state-diagrams"
kind = "diagram"
producer = "just state-diagrams"
entrypoint = "reports/latest/state-diagrams/index.html"
metadata = "reports/latest/state-diagrams/report.json"
```

## This Sprint Does Not Close

- producer command semantics
- source-collection/render-many behavior
- template-family behavior
- panel chrome/copy behavior
- latest/archive output policy
- publish transport or CI workflow logic

## Acceptance Criteria

- `requirements.md` and `architecture.md` define reporting as a generic
  artifact contract rather than as a one-off sprint-report HTML feature
- the plan names the canonical report contract members:
  - report id
  - report kind
  - producer owner
  - latest entrypoint
  - metadata sidecar
- the plan defines a stable separation between authored docs and generated
  report evidence
- the plan keeps network publishing and browser-open behavior outside the
  core engine

## Required Validation

- `cargo fmt --all --check`
