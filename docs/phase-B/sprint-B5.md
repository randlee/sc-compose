---
id: B5
title: Latest And Archive Output Policy And Reports Aggregator
status: complete
branch: feat/sprint-B5
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B5
---

# Sprint B5 — Latest And Archive Output Policy And Reports Aggregator

## Goal

Implement how producers write stable latest outputs and optional timestamped
archive copies, and implement the shared `just reports` aggregation and
verification behavior over those outputs.

## Hard Dependencies

- [docs/phase-A/sprint-A6.md](../phase-A/sprint-A6.md)
- [docs/phase-B/sprint-B1.md](./sprint-B1.md)
- [docs/phase-B/sprint-B3.md](./sprint-B3.md)

## Exact Targets

- `Justfile`
- `crates/sc-compose/src/reporting/output.rs`
- `crates/sc-compose/src/reporting/index.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B5.md`

## Deliverables

- one implemented output policy that supports:
  - overwrite latest artifact in place
  - optionally also write timestamped archive copies
- one canonical timestamp naming policy for archived outputs
- one per-report `report.json` sidecar with:
  - `report_id`
  - `kind`
  - `produced_at`
  - `status`
  - `entrypoint`
  - artifact list
- one real `sc-compose`-backed implementation of `just reports` and
  `just reports-verify`, where B2 owns only the scaffold-generated stubs
- one `sc-compose reports index` path printer/summarizer
- one `sc-compose reports verify` path that fails when required report evidence
  is missing
- one explicit note that archive directories are file-system-local and may be
  consumer-managed or gitignored
- one explicit note that browser opening remains wrapper-owned and is not part
  of the shared command contract
- one explicit deferral that the remaining small `crates/sc-compose/src/main.rs`
  simplification delta on this branch is owned by B9 rather than B5, because
  B5 only owns the latest/archive reporting surface and its direct command seam

## Explicit Code Samples

```json
{
  "report_id": "smoke",
  "kind": "smoke",
  "produced_at": "2026-05-26T20:14:55Z",
  "status": "pass",
  "entrypoint": "reports/latest/smoke/index.html",
  "artifacts": [
    "reports/latest/smoke/index.html",
    "reports/latest/smoke/report.json"
  ]
}
```

```text
reports/latest/smoke/
reports/archive/2026-05-26T20-14-55Z/smoke/
```

## This Sprint Does Not Close

- publish-manifest generation
- remote publish transport
- example migration proof

## Acceptance Criteria

- the runtime distinguishes latest overwrite from timestamped archive copy
- producer output can be written to `reports/latest/<report-id>/`
- the same run can emit a timestamped archive copy
- the runtime keeps archive writing deterministic and file-system-local
- the runtime makes `just reports` the shared aggregator and verifier rather
  than a producer that reruns all evidence collection
- missing required evidence fails `reports verify`
- the scaffold owns creation of `reports/latest/smoke/`, and
  report-smoke execution on this branch writes into that prepared path rather
  than creating it implicitly at runtime

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
