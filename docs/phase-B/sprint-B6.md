---
id: B6
title: Publish Manifest And CI Handoff
status: complete
branch: feat/sprint-B6
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B6
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
- one explicit deferral that the remaining `crates/sc-compose/src/main.rs`
  simplification delta on this branch is owned by B9 rather than B6, because
  B6 owns the publish-manifest reporting surface and its direct command seam

## Explicit Code Samples

```json
{
  "generated_at": "2026-05-26T20:14:55Z",
  "reports": [
    {
      "report_id": "state-diagrams",
      "kind": "diagram",
      "entrypoint": "reports/latest/state-diagrams/index.html",
      "archive_root": "reports/archive/2026-05-26T20-14-55Z/state-diagrams",
      "files": [
        {
          "role": "entrypoint",
          "path": "reports/latest/state-diagrams/index.html",
          "publish_to": "reports/state-diagrams/index.html"
        },
        {
          "role": "metadata",
          "path": "reports/latest/state-diagrams/report.json",
          "publish_to": "reports/state-diagrams/report.json"
        }
      ]
    }
  ]
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
- at least one test exercises the publish-manifest output path and validates
  the emitted manifest shape
- the scaffold owns creation of `reports/latest/smoke/`, and
  report-smoke execution on this branch writes into that prepared path rather
  than creating it implicitly at runtime

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
