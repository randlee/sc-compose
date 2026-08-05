---
id: J.1
title: CLI Argument and Pass-Input Seams
phase: J
status: complete
branch: sprint/j-1-cli-argument-seams
worktree: ../sc-compose-worktrees/sprint/j-1-cli-argument-seams
target: integrate/phase-j
---

# Sprint J.1 — CLI Argument and Pass-Input Seams

## Purpose

Reduce `crates/sc-compose/src/cli.rs`'s hot-spot risk (Repowise score 3.31,
issue #212) by splitting one module's mixed ownership of the Clap command
schema, shared argument structs, pass-scoped argument preprocessing, format
mapping, JSON-capability mapping, and the process-argument adapter into
focused internal modules, without changing any observable CLI behavior.

## Dependencies and exact targets

- `crates/sc-compose/src/cli.rs:15-411` (command schema and shared argument
  structs);
- `crates/sc-compose/src/cli.rs:413-614` (pass-input preprocessing, format
  mapping, JSON-capability mapping, process-argument adapter);
- existing pure seams to build around: `parse_var`, `parse_pass_inputs`,
  `filtered_args_for_clap`, `command_wants_json`;
- consumers that must not require call-site changes: `main.rs`,
  `commands/dispatch.rs`, `commands/compose.rs`, `commands/verify.rs`,
  `template_init.rs`, `render_request/*`, and all CLI/JSON CLI tests.

No dependency on J.2-J.4; this is the independent first sprint.

## Deliverables

- Split `cli.rs` into internal modules (e.g. `cli/schema.rs`,
  `cli/pass_input.rs`, `cli/capability.rs` — exact names are comp's
  implementation call as long as re-exports hold) that separate: (a) the
  Clap command/argument schema, (b) pass-scoped argument normalization, and
  (c) format/JSON-capability mapping.
- Preserve `crate::cli::*` as the sole public import surface; every existing
  `use crate::cli::...` call site in `main.rs`, `commands/*`,
  `template_init.rs`, and `render_request/*` continues to compile unchanged.
- Keep command semantics (dispatch/execution logic) in `commands/*` — this
  sprint moves argument/schema/normalization code only, not command
  behavior.
- Add characterization tests for `parse_var`, `parse_pass_inputs`,
  `filtered_args_for_clap`, and `command_wants_json` before moving any code,
  covering their current input/output pairs exactly as they behave today.

## Planned internal seam

The implementation may use private submodules with the following ownership;
the re-export surface remains `crate::cli::*`:

```rust
mod schema;
mod pass_input;
mod capability;

pub(crate) use schema::*;
pub(crate) use pass_input::{filtered_args_for_clap, parse_pass_inputs};
pub(crate) use capability::command_wants_json;
```

The exact module names may differ, but no command implementation or alternate
public argument model is introduced.

## Acceptance criteria

- Every existing CLI flag, subcommand, default value, and error message is
  byte-for-byte unchanged (verified by running the existing CLI/JSON CLI test
  suites unchanged before and after the split).
- No public API in `sc-compose` changes; `bindings/python` requires no
  changes.
- The split shows material NLOC/complexity reduction for `cli.rs` and its new
  internal modules in the diff and decomposition evidence. A fresh Repowise
  scan is a `quality-mgr`-owned, post-integration diagnostic requested after
  the Phase J integration tip is available and recorded in the plan-gate
  report; it is informational rather than a sprint hard gate because scan
  timing is outside sprint control.
- NLOC evidence (baseline `8eb239e` → integration tip `3703035`) uses
  nonblank, non-comment lines before the first `#[cfg(test)]` marker for
  production and counts test lines separately: `cli.rs` is 568 production /
  194 test NLOC before the split; `cli/` is 578 production / 207 moved+added
  test NLOC after the split, with `schema.rs` as the largest production module
  at 371 NLOC.
- Existing multi-pass, format-mapping, and JSON-capability CLI test suites
  pass unchanged.

## Required validation

Use the [Phase J authoritative validation
checklist](phase-J-plan.md#authoritative-validation-checklist). The focused
evidence must include the characterization tests added before the move, run
both pre-move and post-move, plus a diff review confirming no CLI-observable
change.

## Removal path

If the split destabilizes any consumer, revert to the single-module `cli.rs`
and keep only the added characterization tests. Do not partially land a
module split with broken re-exports.

## Out of scope

- any change to command dispatch/execution logic in `commands/*`;
- any new CLI flag, subcommand, or behavior change;
- `crates/sc-composer/src/discovery.rs` or `crates/sc-composer/src/extract/*`
  (excluded from Phase J entirely, per the phase plan's hard boundaries).
