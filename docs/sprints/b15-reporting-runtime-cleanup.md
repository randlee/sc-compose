---
id: B15
title: Reporting Runtime Cleanup
status: planned
branch: feat/b15-reporting-runtime-cleanup
worktree: ../sc-compose-worktrees/feat/b15-reporting-runtime-cleanup
target: integrate/phase-B
---

# Sprint B15 — Reporting Runtime Cleanup

## Goal

- Remove the remaining dead reporting seams and over-scoped helpers that survived the Phase B merge.
- Keep the reporting runtime behavior stable while closing the maintainability gaps called out by production-readiness review.
- Finish the reporting-layer cleanup separately from CLI extraction so closure stays reviewable and production-ready.
## Hard Dependencies

- `integrate/phase-B` at the current merged Phase B tip.
- Sprint B14 CLI extraction is complete so this sprint only carries reporting-runtime cleanup.
- Production-readiness review findings around the dead `_observer` seam, duplicated relative-path helpers, and zero-caller reporting constants.
## Exact Targets

- `crates/sc-compose/src/reporting/spec.rs`
- `crates/sc-compose/src/reporting/init.rs`
- `crates/sc-compose/src/reporting/output.rs`
- `crates/sc-compose/src/path_utils.rs`
- `crates/sc-compose/src/reporting/mod.rs`

Phase B branch note:

- Exact Targets are verified against `integrate/phase-B`, which is the target
  branch for this cleanup work.
- `crates/sc-compose/src/reporting/spec.rs`,
  `crates/sc-compose/src/reporting/init.rs`, and
  `crates/sc-compose/src/reporting/output.rs` are Phase B-origin files and must
  be reviewed on that branch rather than against older `develop` or `main`
  baselines.
## Deliverables

- The dead `_observer` parameter in `reporting/spec.rs` is removed or wired to real behavior.
- Duplicated `resolve_relative_path(...)` reporting helpers are consolidated into one shared implementation.
- Zero-caller `pub(crate)` reporting constants are removed or scope-reduced to the module that actually owns them.
- Reporting runtime behavior and command contracts remain unchanged after the cleanup.
## Required Work

- Resolve the dead `_observer` seam in `reporting/spec.rs` by either deleting it or wiring it to the shipped observer behavior.
- Move duplicated reporting-layer path resolution through one shared helper and update every current call site.
- Audit report constants in `reporting/init.rs` and `reporting/output.rs` so constants with only one caller are no longer exposed wider than necessary.
- Keep the sprint scoped to reporting-runtime cleanup only; no new CLI extraction or command-surface redesign lands here.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.


```rust
pub(crate) fn resolve_reporting_relative_path(
    workspace_root: &Path,
    path: &Path,
) -> Result<PathBuf, CommandError>;

pub(crate) fn run_render_spec_report(
    root: &Path,
    spec_path: &Path,
    archive: bool,
    observer: &mut dyn CompositionObserver,
) -> Result<ReportsSmokeResult, CommandError>;
```

## This Sprint Does Not Close

- No new reporting features.
- No CLI command-surface redesign.
- No JSON or text contract changes beyond what is required to preserve the shipped behavior during cleanup.
## Acceptance Criteria

- No dead `_observer` seam remains in the reporting runtime.
- The reporting-layer relative-path helper exists in one place only after the sprint.
- Reporting constants touched by the cleanup are no more visible than their actual callers require.
- `cargo test --workspace` and `cargo clippy --all-targets --all-features -- -D warnings` pass on the implementation branch.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
