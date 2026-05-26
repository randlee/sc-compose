---
id: A6
title: Latest And Archive Output Policy And Reports Aggregator
status: complete
branch: feat/sprint-A6
worktree: /Users/randlee/Documents/github/sc-compose-sprint-A6
---

# Sprint A6 — Latest And Archive Output Policy And Reports Aggregator

## Goal

Define how producers write stable latest outputs and optional timestamped
archive copies, and define the shared `just reports` aggregation and
verification behavior over those outputs.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](./sprint-A1.md)
- [docs/phase-A/sprint-A2.md](./sprint-A2.md)
- [docs/phase-A/sprint-A5.md](./sprint-A5.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A6.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Deliverables

- one planned output policy that supports:
  - overwrite latest artifact in place
  - optionally also write timestamped archive copies
- one canonical timestamp naming policy for archived outputs
- one explicit `just reports` contract for:
  - verify required evidence exists
  - summarize report status across producers
  - build or refresh a combined index if needed
  - print or summarize the latest report entrypoints/paths
- one explicit note that archive directories are file-system-local and may be
  consumer-managed or gitignored
- one explicit note that browser opening remains wrapper-owned and is not part
  of the shared Phase A command contract

## Explicit Code Samples

```json
{
  "latest": "reports/latest/sc-lint/index.html",
  "archive": "reports/archive/2026-05-25T22-10-00Z/sc-lint/index.html",
  "metadata": "reports/latest/sc-lint/report.json"
}
```

## This Sprint Does Not Close

- publish-manifest generation
- remote publish transport
- bundled example migration

## Acceptance Criteria

- the plan distinguishes latest overwrite from timestamped archive copy
- the plan keeps archive writing deterministic and file-system-local
- the plan makes `just reports` the shared aggregator and verifier rather than
  a producer that reruns all evidence collection
- the plan states how missing required evidence causes report verification to
  fail

## Required Validation

- `cargo fmt --all --check`
