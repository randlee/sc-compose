---
id: B8
title: Cross-Use-Case Proof By Implemented Examples
status: draft
branch: plan/phase-B
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/phase-B
---

# Sprint B8 — Cross-Use-Case Proof By Implemented Examples

## Goal

Prove that the shared reporting model is genuinely reusable by implementing
multiple distinct report families instead of optimizing only for one consumer
repo.

## Hard Dependencies

- [docs/phase-A/sprint-A8.md](../phase-A/sprint-A8.md)
- [docs/phase-B/sprint-B2.md](./sprint-B2.md)
- [docs/phase-B/sprint-B5.md](./sprint-B5.md)
- [docs/phase-B/sprint-B6.md](./sprint-B6.md)
- [docs/phase-B/sprint-B7.md](./sprint-B7.md)

## Exact Targets

- `Justfile`
- `examples/sprint-report-html.html.j2`
- `examples/sprint-report-html.sample-vars.json`
- `examples/report-evidence-summary.html.j2`
- `examples/report-evidence-summary.sample-vars.json`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B8.md`

## Deliverables

- one implemented `atm-core` style example family covering:
  - repeated state-machine and SQL-query panels
  - panel fragments or per-panel entrypoints where applicable
  - text copy and optional JSON copy actions
  - JSON sidecars and publish-manifest output
- one implemented `sc-lint` style example family covering:
  - lint evidence reports
  - test evidence reports
  - smoke evidence reports
  - latest plus archive output policy
  - publish-manifest output
- one backward-compatibility test harness that exercises the existing
  `sprint-report-html` and bundled examples through the shared reporting
  runtime after migration so consumer workflow regressions are caught before
  they reach consumers
- one set of edge-case inputs that stress the pipeline:
  - templates with nested includes
  - missing optional fields
  - extra unknown fields
  - large variable payloads
  - unusual whitespace
- one explicit rule in runtime/docs that repo-specific custom producers can be
  added without changing the shared report discovery or verification contract
- one explicit note that `atm-core` and `sc-lint` are illustrative family
  labels only; the B4 template-family key remains the catalog discriminator
- one explicit note that producer extension-point typing remains owned by the
  B1 report artifact runtime
- one explicit ownership statement:
  - `sc-compose` provides the example fixtures and harness scaffolding as part
    of the bundled examples tree
  - consumer repos own the repo-specific test bodies invoked by `just smoke`
- one explicit note that `report-evidence-summary` is a new proof vehicle
  introduced in Phase B rather than a Phase A commitment

## Explicit Code Samples

```text
just lint
just test
just smoke
just state-diagrams
just sql-diagrams
just reports
just reports-verify
```

## This Sprint Does Not Close

- browser-open behavior inside `sc-compose`
- remote publish behavior inside `sc-compose`
- observability `1.2` uplift

## Acceptance Criteria

- the runtime implements at least two clearly different consumer shapes and
  shows why the same reporting model serves both
- the reporting line no longer treats multi-panel reporting as a
  sprint-report-only concern
- the runtime keeps producer-command variation repo-local while preserving the
  shared report contract
- the implemented examples act as proof of generality rather than product-local
  exceptions
- the backward-compatibility harness and edge-case fixtures remain part of the
  shared proof so existing bundled examples keep passing after migration

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
