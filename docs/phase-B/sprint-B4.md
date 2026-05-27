---
id: B4
title: Template Families And Shared Panel Chrome
status: complete
branch: feat/sprint-B4
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B4
---

# Sprint B4 — Template Families And Shared Panel Chrome

## Goal

Implement how shared templates, includes, and panel chrome are selected and
overridden so report UI and copy behavior stop being reimplemented in each
consumer repo.

## Hard Dependencies

- [docs/phase-A/sprint-A5.md](../phase-A/sprint-A5.md)
- [docs/phase-B/sprint-B2.md](./sprint-B2.md)
- [docs/phase-B/sprint-B3.md](./sprint-B3.md)

## Exact Targets

- `crates/sc-compose/src/template_store.rs`
- `crates/sc-compose/src/reporting/templates.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `crates/sc-compose/assets/reports/templates/base/report.html.j2`
- `crates/sc-compose/assets/reports/templates/diagram/report.html.j2`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B4.md`

## Deliverables

- one implemented template-family model for at least:
  - lint/test/smoke evidence reports
  - public API / CLI / ICD style reports
  - diagram/state-machine and SQL-query reports
- one implemented override contract so a repo can point a report family at
  repo-local templates without forking shared behavior
- one shared panel contract with:
  - report header
  - panel body content
  - panel footer
  - required copy-text action
  - optional copy-JSON action
  - optional fragment/open link
- one explicit split between shared panel chrome and consumer-specific panel
  body content

## Explicit Code Samples

```toml
[reporting.templates.diagram]
source = "shared:diagram"

[reporting.templates.lint]
path = "reports/templates/lint/report.html.j2"
```

```jinja2
{% block report_header %}{% endblock %}
{% block panel_body %}{% endblock %}
{% block panel_footer %}{% endblock %}
```

## This Sprint Does Not Close

- latest/archive output policy
- publish-manifest behavior
- bundled example migration

## Acceptance Criteria

- the runtime names the initial template families and intended use cases
- `shared:<family>` resolves to bundled assets without passing the URI into
  `sc-composer`
- repo-local overrides can replace one family without forking unrelated shared
  families
- the runtime makes per-panel text copy mandatory
- the runtime makes per-panel JSON copy optional but first-class
- the runtime keeps panel chrome in shared template behavior rather than
  wrapper-only logic

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
