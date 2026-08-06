---
id: CLEANUP-297
title: Simplify has_bare_for_loop_over
status: complete
branch: cleanup/bare-loop-discovery-simplify
worktree: ../sc-compose-worktrees/cleanup/bare-loop-discovery-simplify
target: develop
---

# Cleanup 297 — simplify has_bare_for_loop_over

## Goal

Use the existing template walker and a focused loop-expression parser instead
of full token/scoping collection for the yes/no bare-loop query.

## Acceptance Criteria

- Simple, nested, filtered, and negative loop cases retain identical results.
- Format, clippy, and workspace tests pass.

## References

- Issue #297: https://github.com/randlee/sc-compose/issues/297
