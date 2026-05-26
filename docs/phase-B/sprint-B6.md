---
id: B6
title: Publish Manifest And CI Handoff
status: draft
branch: plan/phase-B
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/phase-B
---

# Sprint B6 — Publish Manifest And CI Handoff

## Goal

Implement the machine-readable handoff from generated report artifacts to CI or
wrapper-owned publication steps without pushing network or hosting behavior
into `sc-compose`.

## Hard Dependencies

- [docs/phase-A/sprint-A7.md](../phase-A/sprint-A7.md)
- [docs/phase-B/sprint-B1.md](./sprint-B1.md)
- [docs/phase-B/sprint-B5.md](./sprint-B5.md)

## Exact Targets

- `crates/sc-compose/src/reporting/publish_manifest.rs`
- `crates/sc-compose/src/json_output.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/publishing.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B6.md`

## Deliverables

- one implemented publish-manifest runtime listing generated artifacts and
  their intended publish destinations
- one explicit boundary:
  - producers and renderers create artifacts and manifest metadata
  - CI or wrapper tooling performs upload/copy/publication
- one explicit statement that network transport remains outside the core
  renderer boundary
- one stable JSON shape listing:
  - `report_id`
  - `kind`
  - `entrypoint`
  - artifact paths
  - archive snapshot path when present

## Explicit Code Samples

```json
{
  "report_id": "state-diagrams",
  "kind": "diagram",
  "entrypoint": "reports/latest/state-diagrams/index.html",
  "artifacts": [
    "reports/latest/state-diagrams/index.html",
    "reports/latest/state-diagrams/report.json"
  ],
  "archive_root": "reports/archive/2026-05-26T20-14-55Z/state-diagrams"
}
```

## This Sprint Does Not Close

- upload implementation
- GitHub Pages, bucket, or release-site-specific workflow logic
- example migration proof

## Acceptance Criteria

- the runtime defines a machine-readable publish-manifest output
- publish-manifest output is generated from report runtime state, not
  hard-coded paths
- the runtime keeps publish transport outside `sc-composer` and `sc-compose`
- the runtime makes CI publication possible without making `sc-compose` own
  hosting logic
- `report_id` stays aligned with the catalog `id`

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
