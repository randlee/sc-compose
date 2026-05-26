---
id: A5
title: Template Families And Shared Panel Chrome
status: complete
branch: feat/sprint-A5
worktree: /Users/randlee/Documents/github/sc-compose-sprint-A5
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

## Authoritative Override Contract

This sprint doc is the authoritative home of the A5 override contract.

Shared lookup namespace:

- `shared:<family>` is a reserved selector owned by the `sc-compose` CLI
  boundary
- the CLI resolves `shared:<family>` against the bundled shared template root
  shipped with `sc-compose`, under the family tree
  `reports/templates/<family>/`
- consumer repos do not pass `shared:` URIs into `sc-composer`; the library
  receives resolved filesystem paths only

Minimum consumer activation config:

- shared family activation uses:
  - `[reporting.templates.<family>] source = "shared:<family>"`
- repo-local override activation uses:
  - `[reporting.templates.<family>] path = ".sc-compose/templates/<file>.html.j2"`
- selecting one repo-local override must not require forking unrelated bundled
  families

Template interface boundary:

- shared chrome exposes the Jinja2 blocks:
  - `report_header`
  - `panel_body`
  - `panel_footer`
- consumer body templates are expected to provide `panel_body` and may
  override `report_header` when a family needs repo-specific framing text
- shared chrome expects the top-level template variables:
  - `title`
  - `panels`
  - optional `report_metadata`
- each `panels[]` entry must provide:
  - `panel_id`
  - `title`
  - `body`
  - `copy_text`
  - optional `copy_json`
  - optional `fragment_href`

Include boundary:

- Cross-family include composition is deferred to a later sprint; single-family
  panel rendering is sufficient for Phase A validation and keeps the first
  override contract narrow enough to verify cleanly.

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
- `cargo clippy --all-targets --all-features`
- `cargo test --all`
