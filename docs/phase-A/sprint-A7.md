---
id: A7
title: Publish Manifest And CI Handoff
status: complete
branch: feat/sprint-A7
worktree: /Users/randlee/Documents/github/sc-compose-sprint-A7
---

# Sprint A7 — Publish Manifest And CI Handoff

## Goal

Define the machine-readable handoff from generated report artifacts to CI or
wrapper-owned publication steps without pushing network or hosting behavior
into `sc-compose`.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](./sprint-A1.md)
- [docs/phase-A/sprint-A6.md](./sprint-A6.md)

## Exact Targets

- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A7.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Deliverables

- one planned publish-manifest contract listing generated artifacts and their
  intended publish destinations
- one explicit boundary:
  - producers and renderers create artifacts and manifest metadata
  - CI or wrapper tooling performs upload/copy/publication
- one explicit statement that network transport remains outside the core
  renderer boundary

## Explicit Code Samples

```json
{
  "report_name": "state-diagrams",
  "generated_at": "2026-05-25T22:10:00Z",
  "files": [
    {
      "role": "latest_html",
      "path": "reports/latest/state-diagrams/index.html",
      "publish_to": "reports/state-diagrams/index.html"
    },
    {
      "role": "json_sidecar",
      "path": "reports/latest/state-diagrams/report.json",
      "publish_to": "reports/state-diagrams/report.json"
    }
  ]
}
```

## This Sprint Does Not Close

- upload implementation
- GitHub Pages, bucket, or release-site-specific workflow logic
- bundled example migration

## Acceptance Criteria

- the plan defines a machine-readable publish-manifest output
- the plan keeps publish transport outside `sc-composer` and `sc-compose`
- the plan makes CI publication possible without making `sc-compose` own
  hosting logic
- the plan keeps artifact roles and publish destinations explicit

## Required Validation

- `cargo fmt --all --check`
