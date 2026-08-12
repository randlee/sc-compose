---
id: CLEANUP-300
title: Evaluate collapsing CurrentIncludeDepth/IncludeDepth wrapper layers
status: complete
branch: cleanup/include-depth-wrapper-simplify
worktree: ../sc-compose-worktrees/cleanup/include-depth-wrapper-simplify
target: develop
---

# Cleanup 300 — include-depth wrapper simplification

## Goal

Evaluate whether `CurrentIncludeDepth` and `IncludeDepth` can be simplified
without changing the maximum recursion limit or over-limit error behavior.

## Required Fix

- Collapse the wrappers only if the result is clearly simpler and retains the
  distinction between configured limits and traversal state.
- Otherwise document why both layers remain.

## Decision

No code collapse was made. `IncludeDepth` is the public, serializable policy
bound, while `CurrentIncludeDepth` is private traversal state with saturating
increment semantics. Keeping the distinct types preserves that boundary and
is clearer than merging them.

## Acceptance Criteria

- A chain exactly at the configured limit succeeds.
- A chain one level over the configured limit fails with the existing error.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace` pass.

## References

- Issue #300: https://github.com/randlee/sc-compose/issues/300
