---
id: repowise-render-request-split
title: Render Request Module Split Cleanup
status: complete
branch: refactor/render-request-real-module
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/refactor/render-request-real-module
target: develop
---

# Render Request Module Split Cleanup

## Scope

This standalone Repowise-hotspot cleanup replaces the 730-line
`render_request.rs` monolith with a real `render_request/` module directory.
The PR description is the authoritative implementation scope. The assignment
stated:

> no dedicated sprint doc exists for this item -- it is a standalone Repowise-hotspot cleanup pass; PR #156's own description is the authoritative scope source for this task. If a documented closeout gate applies (sprint doc + docs/project-plan.md entry), satisfy it the same way you did for publish-manifest-split-merge-review-010.

This document records the completed closeout for that item.

## Intentional Testability Addition

This was not a pure relocation. As an intentional, narrowly scoped addition,
`blocks.rs` introduces the `pub(super) read_optional_block_with` seam and the
`read_stdin_to_string` / `read_file_to_string` helpers so unit tests can cover
inline, file, and stdin success paths without touching process I/O. The
production `read_optional_block` wrapper retains the pre-split observable
behavior; the seam exists solely to make those paths directly testable.

## Deliverables

- `blocks.rs` owns guidance/prompt input sources and stdin-read validation.
- `mode.rs` owns mode, profile, runtime, and confinement-root construction.
- `request.rs` owns composition request assembly.
- `vars.rs` owns CLI, var-file, environment, and pass-variable handling.
- `tests.rs` owns focused unit coverage for each helper group.
- `mod.rs` remains a thin crate-internal re-export boundary.

## Validation

- `cargo test --workspace` — passed, 0 failures.
- `cargo build --workspace` — passed.
- `cargo fmt --all --check` — passed.
- `cargo clippy --all-targets --all-features -- -D warnings` — passed.
- `git diff --check` — passed.

## Exit Verdict

The split follows genuine responsibility boundaries, introduces no material
duplication or awkward coupling, and keeps visibility appropriately narrow.
The refactor is complete and ready for independent regression QA by
`quality-mgr`.
