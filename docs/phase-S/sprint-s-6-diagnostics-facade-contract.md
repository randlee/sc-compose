---
id: S.6
title: Diagnostics Facade Contract
status: planned
branch: sprint/s-6-diagnostics-facade-contract
worktree: ../sc-compose-worktrees/sprint/s-6-diagnostics-facade-contract
target: sprint/s-5-boundary-invariant-guardrails
---

# Sprint S.6 — Diagnostics Facade Contract

## Goal

This is a coverage-only sprint: it freezes the existing diagnostics public
facade, schema version, spelling, and envelope defaults through regression
tests and makes no production code change to its target files. It closes S-T9
only in the sense that the existing contract is frozen, not refactored.

## Hard Dependencies

- S.5 is this branch's required `gh stack` parent. There is no functional code
  dependency on S.1–S.5; the parent keeps this PR incremental.

## Exact Targets

- `crates/sc-composer/src/diagnostics.rs`
- `crates/sc-composer/src/diagnostics/envelope.rs`
- `crates/sc-composer/src/diagnostics/record.rs`
- `crates/sc-composer/src/diagnostics/schema.rs`
- existing diagnostics unit-test locations only
- `docs/plans/phase-S.md`

## Deliverables

- Contract tests for the existing schema-version value, four public re-exports,
  their spelling, and `DiagnosticEnvelope::new` defaults.

## Required Work

- Test the existing public boundary; do not add types, exports, serialized
  fields, or a new diagnostics abstraction.
- Follow `CLAUDE.md` Rule 1: remain pure `sc-composer` library work with no
  CLI, adapter, filesystem-policy, or ATM dependency.
- **Production-ready closure:** every listed facade assertion and envelope
  default regression must land in this sprint; partial export or schema-default
  coverage does not close S-T9.

## Explicit Code Samples

```rust
pub const DIAGNOSTIC_SCHEMA_VERSION: &str = "1";
pub use envelope::DiagnosticEnvelope;
pub use record::Diagnostic;
pub use schema::{DiagnosticCode, DiagnosticSeverity};
```

## This Sprint Does Not Close

- Repository-boundary test organization (S.5).
- Path-normalization coverage (S.7), runner work (S.8), or a diagnostic schema
  revision.

## Acceptance Criteria

- [ ] Tests freeze `DIAGNOSTIC_SCHEMA_VERSION == "1"` and all four listed
  public re-exports with their current spelling.
- [ ] Tests freeze `DiagnosticEnvelope::new` defaults without changing the
  serialized diagnostic schema.
- [ ] No new public API, dependency, CLI, adapter, or filesystem-policy change
  occurs.

## gh-stack Workflow

```bash
# The phase plan added this branch directly on top of S.5.
git config rerere.enabled true
git config remote.pushDefault origin
git add crates/sc-composer/src/diagnostics.rs crates/sc-composer/src/diagnostics docs/plans/phase-S.md docs/phase-S/sprint-s-6-diagnostics-facade-contract.md
git commit -m "test(diagnostics): freeze facade contract"
gh stack submit --auto
gh pr ready <sprint-s-6-pr-number>
gh stack view --json
# Do not merge an individual sprint layer; phase close merges the full stack.
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-composer --all-targets --all-features -- -D warnings`
- `cargo test -p sc-composer`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
