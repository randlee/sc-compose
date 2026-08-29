---
id: S.6
title: Diagnostics Facade Contract
status: complete
branch: sprint/s-6-diagnostics-facade-contract
worktree: ../sc-compose-worktrees/sprint/s-6-diagnostics-facade-contract
target: integrate/phase-s
---

# Sprint S.6 — Diagnostics Facade Contract

## Goal

Add regression coverage that freezes the existing diagnostics public facade,
schema version, spelling, and envelope defaults without adding public API.
This closes S-T9.

## Hard Dependencies

- `integrate/phase-s` exists from `develop` before this sprint branch exists.
- No hard code dependency on S.1–S.5; merge-forward the latest integration
  branch before implementation and submission.

## Exact Targets

- `crates/sc-composer/src/diagnostics.rs`
- `crates/sc-composer/src/diagnostics/{envelope,record,schema}.rs`
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
git switch integrate/phase-s
git pull --ff-only origin integrate/phase-s
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base integrate/phase-s sprint/s-6-diagnostics-facade-contract
git add crates/sc-composer/src/diagnostics.rs crates/sc-composer/src/diagnostics docs/plans/phase-S.md docs/phase-S/sprint-s-6-diagnostics-facade-contract.md
git commit -m "test(diagnostics): freeze facade contract"
gh stack submit --auto
gh stack view --json
gh stack merge <sprint-s-6-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-composer --all-targets --all-features -- -D warnings`
- `cargo test -p sc-composer`
- `cargo test --workspace`
- `git diff --check`
