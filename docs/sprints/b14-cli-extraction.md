---
id: B14
title: CLI Extraction
status: complete
branch: feat/b14-cli-extraction
worktree: ../sc-compose-worktrees/feat/b14-cli-extraction
target: integrate/phase-B
---

# Sprint B14 — CLI Extraction

## Goal

- Decompose oversized CLI files into focused modules with clear ownership boundaries.
- Finish the command-surface extraction work needed to make the CLI layer reviewable and maintainable on the shipped Phase B branch.
- Preserve the shipped command surface and runtime behavior while reducing file-level review risk.

## Hard Dependencies

- `integrate/phase-B` at the current merged Phase B tip.
- Production-readiness review findings around oversized CLI files and remaining maintainability debt.
- Pre-condition gate: if `crates/sc-compose/src/commands/reports.rs` is absent
  at sprint start, the implementation branch is on the wrong baseline and the
  sprint must stop until it is rebased onto `integrate/phase-B`.

## Exact Targets

- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/src/commands/mod.rs`
- `crates/sc-compose/src/commands/reports.rs`
- `crates/sc-compose/src/commands/examples.rs`
- `crates/sc-compose/src/commands/templates.rs`
- `crates/sc-compose/src/commands/reports/mod.rs`
- `crates/sc-compose/src/commands/reports/scaffold.rs`
- `crates/sc-compose/src/commands/reports/render.rs`
- `crates/sc-compose/src/commands/reports/publish.rs`

Phase B branch note:

- Exact Targets are verified against `integrate/phase-B`, which is the target
  branch for this cleanup work.
- `crates/sc-compose/src/commands/reports.rs` is the Phase B-origin reports
  command surface that remains above the cleanup size cap on
  `integrate/phase-B`.

## Deliverables

- `crates/sc-compose/src/main.rs` is decomposed so no single CLI file remains above the 400-line cap.
- `crates/sc-compose/src/commands/reports.rs` is replaced by a focused reports module tree rooted at `crates/sc-compose/src/commands/reports/mod.rs`.
- The post-extraction reports module structure is reviewable as:
  - `crates/sc-compose/src/commands/reports/mod.rs`
  - `crates/sc-compose/src/commands/reports/scaffold.rs`
  - `crates/sc-compose/src/commands/reports/render.rs`
  - `crates/sc-compose/src/commands/reports/publish.rs`
- Command dispatch and helper ownership are localized under `crates/sc-compose/src/commands/` instead of remaining centralized in `main.rs`.
- The shipped command surface and JSON/text contracts remain unchanged after the extraction.

## Required Work

- Extract subcommand dispatch and helper logic out of `main.rs` into focused modules while preserving the current command contract.
- Split `commands/reports.rs` by subcommand family or responsibility so review and ownership are localized.
- Move any CLI-only helper logic that blocks the file-size cap into focused sibling command modules rather than leaving partial extraction in place.
- Keep the sprint scoped to CLI extraction only; reporting-runtime seam cleanup lands in a follow-on sprint.

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
// crates/sc-compose/src/commands/mod.rs
pub(crate) mod examples;
pub(crate) mod reports;
pub(crate) mod templates;

// crates/sc-compose/src/commands/reports/mod.rs
pub(crate) mod publish;
pub(crate) mod render;
pub(crate) mod scaffold;

use crate::commands::examples::{run_examples_list, run_examples_render};
use crate::commands::reports::{run_reports_init, run_reports_smoke, run_reports_verify};
use crate::commands::templates::{run_templates_add, run_templates_list, run_templates_render};
```

## This Sprint Does Not Close

- No new command-surface behavior.
- No change to JSON or text output contracts beyond what extraction requires to preserve them.
- No reporting-runtime seam cleanup inside `reporting/spec.rs`, `reporting/init.rs`, or `reporting/output.rs`.

## Acceptance Criteria

- No touched source file remains above 400 lines.
- `main.rs` and the reports command surface are both below the size cap with focused module boundaries.
- Command behavior, argument parsing, and JSON/text output remain compatible with the shipped `integrate/phase-B` contract.
- `cargo test --workspace` and `cargo clippy --all-targets --all-features -- -D warnings` pass on the implementation branch.

## Required Validation

- `git show origin/integrate/phase-B:crates/sc-compose/src/commands/reports.rs >/dev/null`
- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `test -f crates/sc-compose/src/commands/reports/mod.rs`
- `test -f crates/sc-compose/src/commands/reports/scaffold.rs`
- `test -f crates/sc-compose/src/commands/reports/render.rs`
- `test -f crates/sc-compose/src/commands/reports/publish.rs`
- `wc -l crates/sc-compose/src/main.rs crates/sc-compose/src/commands/reports/mod.rs crates/sc-compose/src/commands/reports/scaffold.rs crates/sc-compose/src/commands/reports/render.rs crates/sc-compose/src/commands/reports/publish.rs`
