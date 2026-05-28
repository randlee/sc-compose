---
id: B14
title: CLI Extraction
status: planned
branch: feat/b14-cli-extraction
worktree: ../sc-compose-worktrees/feat/b14-cli-extraction
target: integrate/phase-B
---

# Sprint B14 — CLI Extraction

## Goal

- Decompose oversized CLI files into focused modules with clear ownership boundaries.
- Remove dead reporting seams and over-scoped constants that survived the Phase B merge.
- Consolidate duplicated relative-path helpers without changing the shipped command surface.
## Hard Dependencies

- `integrate/phase-B` at the current merged Phase B tip.
- Production-readiness review findings around oversized CLI files and remaining maintainability debt.
## Exact Targets

- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/src/commands/mod.rs`
- `crates/sc-compose/src/commands/reports.rs`
- `crates/sc-compose/src/reporting/spec.rs`
- `crates/sc-compose/src/reporting/init.rs`
- `crates/sc-compose/src/reporting/output.rs`
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `crates/sc-compose/src/main.rs` is decomposed so no single CLI file remains above the 400-line cap.
- `crates/sc-compose/src/commands/reports.rs` is split into focused report-command modules.
- The dead `_observer` parameter in `reporting/spec.rs` is removed or wired to real behavior.
- Zero-caller `pub(crate)` reporting constants are removed or scope-reduced, and duplicated relative-path helpers are consolidated.
## Required Work

- Extract subcommand dispatch and helper logic out of `main.rs` into focused modules while preserving the current command contract.
- Split `commands/reports.rs` by subcommand family or responsibility so review and ownership are localized.
- Resolve the dead `_observer` seam in `reporting/spec.rs` and audit over-scoped report constants in `reporting/init.rs` / `reporting/output.rs`.
- Consolidate duplicated `resolve_relative_path(...)` logic from the reporting layer into one shared helper and update call sites.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.


```rust
mod commands;
mod command_error;
mod observability;
mod observer_impl;

use crate::commands::{run_reports_command, run_templates_command, run_examples_command};
```

## This Sprint Does Not Close

- No new command-surface behavior.
- No change to JSON or text output contracts beyond what extraction requires to preserve them.
- No new reporting features.
## Acceptance Criteria

- No touched source file remains above 400 lines.
- Dead code called out in the production-readiness findings is removed or wired.
- The duplicated relative-path helper exists in one place only after the sprint.
- `cargo test --workspace` and `cargo clippy --all-targets --all-features -- -D warnings` pass on the implementation branch.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
