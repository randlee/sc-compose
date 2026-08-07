---
id: K.2
title: Compose Command Orchestration
phase: K
status: planned
branch: sprint/k-2-compose-command-orchestration
worktree: ../sc-compose-worktrees/sprint/k-2-compose-command-orchestration
target: integrate/phase-k
---

# Sprint K.2 — Compose Command Orchestration

## Purpose and evidence

Issue #311 ranks `crates/sc-compose/src/commands/compose.rs` at 3.09/10 and 593 NLOC. The module combines request construction, custom-delimiter preflight, render execution, output writing, validation diagnostics, and CLI presentation. This sprint separates those private responsibilities while preserving the command surface.

## Exact targets and deliverables

- `crates/sc-compose/src/commands/compose.rs:95-624`, including `execute_render_with_extra_warnings`, `execute_render_with_expanded`, `execute_custom_delimiter_render`, `emit_render_output`, `preflight_template`, `build_custom_render_context`, and `assemble_output`.
- Create private submodules for render orchestration, preflight/request assembly, and output/diagnostic presentation; keep existing `run_render`, `run_validate`, `run_verify`, and `run_resolve` entry points unchanged.
- Characterize text/JSON output, dry-run, custom delimiters, multi-pass `--all`, stdin, output-file, validation failure, and observer events before moving code.

## Acceptance criteria

- Existing flags, exit codes, JSON envelopes, diagnostics, observer events, output paths, and newline behavior are unchanged.
- No command behavior moves into `sc-composer`; no public CLI API changes.
- Production-NLOC evidence shows each new module has one primary responsibility and `compose.rs` no longer owns all execution paths.

## Required validation

Run focused CLI and JSON CLI characterization tests before and after, `cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

## Dependencies and non-closure

Independent from K.1 and K.3-K.8. This sprint does not add commands, flags, output formats, or behavior changes.
