---
id: A2
title: Producer Recipes And Just Surface
status: complete
branch: feat/sprint-A2
worktree: /Users/randlee/Documents/github/sc-compose-sprint-A2
---

# Sprint A2 — Producer Recipes And Just Surface

## Goal

Define the standard producer command contract so lint, test, smoke, and
repo-specific custom producers all generate evidence in the same shape, while
`just reports` remains the shared aggregator and verifier.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](./sprint-A1.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A2.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Deliverables

- one standard producer contract for:
  - `just lint`
  - `just test`
  - `just smoke`
  - repo-specific custom producers such as diagram or schema reports
- one explicit `just reports` contract for:
  - verify expected evidence exists
  - build or refresh a combined index if needed
  - open/view the latest report set
- one explicit statement that adding repo-specific producer commands must not
  require changing the shared aggregation contract

## Explicit Code Samples

```make
just lint
just test
just smoke
just state-diagrams
just sql-diagrams
just reports
just reports-verify
just reports-open
```

## This Sprint Does Not Close

- source-collection/render-many behavior
- semantic diagram-spec design
- template-family behavior
- panel chrome/copy behavior
- archive output policy
- publish-manifest behavior

## Acceptance Criteria

- the plan makes producer recipes the owners of report generation
- the plan reserves `just reports` for aggregation, verification, and
  opening/viewing rather than primary generation
- the plan allows repo-specific custom producers without changing the report
  discovery contract
- the plan states how a producer identifies the report ids it owns

## Required Validation

- `cargo fmt --all --check`
