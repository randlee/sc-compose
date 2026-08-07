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

- `crates/sc-compose/src/commands/compose.rs`, including
  `execute_render_with_extra_warnings`, `execute_render_with_expanded`,
  `execute_custom_delimiter_render`, `emit_render_output`,
  `preflight_template`, `build_custom_render_context`, and `assemble_output`.
- Create private submodules for render orchestration, preflight/request assembly, and output/diagnostic presentation; keep existing `run_render`, `run_validate`, `run_verify`, and `run_resolve` entry points unchanged.
- Add or strengthen characterization tests for text/JSON output, dry-run,
  custom delimiters, multi-pass `--all`, stdin, output-file, validation
  failure, and observer events before moving code.

## Planned seam

The command entry points stay in `commands::compose`; only private request,
execution, and presentation seams may move. The contract is represented by
these existing signatures:

```rust
pub(crate) fn run_render(
    args: &RenderArgs,
    observer: &mut dyn CompositionObserver,
) -> Result<i32, CommandError>;
fn preflight_template(
    request: &ComposeRequest,
) -> Result<(ResolveResult, ExpandedTemplate, Vec<Frontmatter>), CommandError>;
fn emit_render_output(
    request: &ComposeRequest,
    args: &RenderBehaviorArgs,
    resolved_path: &Path,
    rendered_text: &str,
    warnings: Vec<Diagnostic>,
) -> Result<(), CommandError>;
```

Private module names may differ, but `run_render`, `run_validate`,
`run_verify`, and `run_resolve` remain at their existing paths. No command
source path is deleted or renamed.

## Acceptance criteria

- Existing flags, exit codes, JSON envelopes, diagnostics, observer events, output paths, and newline behavior are unchanged.
- No command behavior moves into `sc-composer`; no public CLI API changes.
- Production-NLOC evidence shows each new module has one primary responsibility and `compose.rs` no longer owns all execution paths.
- No executable behavior is deleted or moved into `sc-composer`; a proposed
  split that cannot preserve the signature contract is abandoned with
  evidence rather than closed as a partial refactor.

## Required validation

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-compose --test cli`
- `cargo test -p sc-compose --test json_cli`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

Record the unchanged CLI/JSON public surface and before/after production-NLOC
evidence.

## Dependencies and non-closure

Independent from K.1 and K.3-K.8. This sprint does not add commands, flags, output formats, or behavior changes.
