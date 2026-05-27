---
id: B7
title: Semantic Diagram Spec Runtime
status: complete
branch: feat/sprint-B7
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B7
---

# Sprint B7 — Semantic Diagram Spec Runtime

## Goal

Implement the long-term semantic source model for state-machine and SQL-query
reports so Mermaid becomes one renderer or migration input rather than remains
the long-term source of truth.

## Hard Dependencies

- [docs/phase-A/sprint-A4.md](../phase-A/sprint-A4.md)
- [docs/phase-B/sprint-B3.md](./sprint-B3.md)
- [docs/phase-B/sprint-B4.md](./sprint-B4.md)

## Exact Targets

- `crates/sc-compose/src/reporting/spec.rs`
- `crates/sc-compose/src/reporting/mermaid.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B7.md`

## Deliverables

- one typed semantic spec runtime for at least:
  - `state_machine`
  - `sql_query`
- one explicit statement that semantic spec input files use TOML format,
  consistent with the report catalog and source-set TOML established in B1 and
  B3
- one transitional rule that Mermaid may still be emitted as an output during
  migration but is not the long-term semantic source model
- one Mermaid renderer that consumes typed specs
- one validation direction for semantic QA against the typed spec rather than
  only string-comparing rendered Mermaid text
- one explicit extension point so repos can add new report-spec kinds later
  without rewriting the catalog or producer contracts
- one render-many integration path so diagram reports participate in the same
  catalog/output pipeline as other report families

## Explicit Code Samples

```rust
pub enum ReportSpec {
    StateMachine(StateMachineSpec),
    SqlQuery(SqlQuerySpec),
}

pub fn render_mermaid(spec: &ReportSpec) -> Result<String, MermaidRenderError>;
```

```toml
[spec]
kind = "state_machine"
renderer_targets = ["mermaid"]
```

## This Sprint Does Not Close

- shared panel chrome/copy behavior beyond the B4 contract
- example migration proof across multiple families
- observability `1.2` uplift

## Acceptance Criteria

- Mermaid is defined as a renderer/output during transition rather than the
  long-term semantic source of truth
- typed diagram specs are first-class runtime inputs
- the runtime names the required semantic fields for `state_machine` and
  `sql_query` reports
- the CLI can render Mermaid output from both supported spec families
- diagram reports use the same latest/archive and publish-manifest pipeline as
  non-diagram reports
- the runtime preserves room for future report-spec kinds beyond the first two

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
