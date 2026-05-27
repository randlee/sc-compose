---
id: B1
title: Report Artifact Runtime And Catalog
status: complete
branch: feat/sprint-B1
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B1
---

# Sprint B1 — Report Artifact Runtime And Catalog

## Goal

Implement the reusable report artifact contract and report catalog so every
repo can generate evidence into the same filesystem and metadata shape without
reintroducing consumer-specific catalog loaders or verification logic.

## Hard Dependencies

- [docs/phase-A/sprint-A1.md](../phase-A/sprint-A1.md)
- [docs/phase-A/sprint-A2.md](../phase-A/sprint-A2.md)
- [docs/phase-B/phase-B-plan.md](./phase-B-plan.md)

## Exact Targets

- `crates/sc-compose/src/reporting/mod.rs`
- `crates/sc-compose/src/reporting/catalog.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/phase-B-plan.md`
- `docs/phase-B/sprint-B1.md`

## Deliverables

- one report artifact runtime that defines:
  - source/catalog locations
  - generated latest output locations
  - generated archive output locations
  - per-report metadata sidecars
- one report catalog loader for `reports/catalog/reports.toml`
- one normalized `ReportDefinition` runtime type with:
  - `id`
  - `kind`
  - `producer`
  - `required`
  - `entrypoint`
  - `metadata`
- one clear ownership split:
  - producer recipes own data gathering and report generation
  - `sc-compose` owns rendering semantics where it is used
  - consumer repos own domain-specific inputs and publish surfaces
- one explicit statement in runtime/docs that authored docs and generated
  evidence stay in separate trees
- one CLI inspection/validation entrypoint that fails fast on:
  - duplicate report ids
  - missing required fields
  - non-normalized entrypoint/metadata paths
  - invalid required-vs-optional verification flags

## Explicit Code Samples

```rust
pub struct ReportDefinition {
    pub id: String,
    pub kind: String,
    pub producer: String,
    pub required: bool,
    pub entrypoint: PathBuf,
    pub metadata: PathBuf,
}

pub fn load_report_catalog(repo_root: &Path) -> Result<ReportCatalog, CatalogError>;
```

```toml
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
```

## This Sprint Does Not Close

- producer command semantics
- repo scaffolding or `Justfile` generation
- source collection or render-many behavior
- template-family resolution
- latest/archive writing
- publish-manifest output

## Explicit Deferral

- `ARCH-001`: [crates/sc-compose/src/main.rs](../../crates/sc-compose/src/main.rs)
  remains above the `1000` non-test line gate in Sprint B1. The line-count
  reduction is deferred to Sprint `B8`, where the `CommandError` extraction
  reduces `main.rs` to `911` non-test lines on the integrated branch.

## Acceptance Criteria

- `requirements.md` and `architecture.md` describe reporting as a generic
  artifact runtime rather than as a one-off sprint-report HTML feature
- `sc-compose` can load `reports/catalog/reports.toml` from the repo root
- malformed catalog entries fail with stable text and JSON diagnostics
- duplicate report ids are rejected before any generation step starts
- the runtime names the canonical report contract members:
  - report id
  - report kind
  - producer owner
  - requiredness for verification
  - latest entrypoint
  - metadata sidecar
- the runtime keeps network publishing and browser-open behavior outside the
  core engine

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
