---
id: A8
title: Cross-Use-Case Proof By Examples
status: complete
branch: feat/sprint-A8
worktree: /Users/randlee/Documents/github/sc-compose-sprint-A8
---

# Sprint A8 — Cross-Use-Case Proof By Examples

## Goal

Prove that the shared reporting model is genuinely reusable by planning
multiple distinct report families instead of optimizing only for one consumer
repo.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](./sprint-A1.md)
- [docs/phase-A/sprint-A2.md](./sprint-A2.md)
- [docs/phase-A/sprint-A3.md](./sprint-A3.md)
- [docs/phase-A/sprint-A4.md](./sprint-A4.md)
- [docs/phase-A/sprint-A5.md](./sprint-A5.md)
- [docs/phase-A/sprint-A6.md](./sprint-A6.md)
- [docs/phase-A/sprint-A7.md](./sprint-A7.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A8.md`
- `docs/html-sprint-report-plan.md`

## Deliverables

- one planned `atm-core` style example family covering:
  - repeated state-machine and SQL-query panels
  - panel fragments or per-panel entrypoints where applicable
  - text copy and optional JSON copy actions
  - JSON sidecars and publish-manifest output
- one planned `sc-lint` style example family covering:
  - lint evidence reports
  - test evidence reports
  - smoke evidence reports
  - latest plus archive output policy
  - publish-manifest output
- one explicit rule that repo-specific custom producers can be added without
  changing the shared report discovery or verification contract

## Explicit Code Samples

```text
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

- full implementation of every example family
- browser-open behavior inside `sc-compose`
- remote publish behavior inside `sc-compose`

## Acceptance Criteria

- the plan names at least two clearly different consumer shapes and explains
  why the same reporting model serves both
- the follow-on line no longer treats multi-panel reporting as a
  sprint-report-only concern
- the plan keeps producer-command variation repo-local while preserving the
  shared report contract
- the examples act as proof of generality rather than product-local exceptions

## Required Validation

- `cargo fmt --all --check`
