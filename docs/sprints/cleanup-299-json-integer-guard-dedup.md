---
id: CLEANUP-299
title: Remove dead duplicate JSON integer guard path
status: complete
branch: cleanup/json-integer-guard-dedup
worktree: ../sc-compose-worktrees/cleanup/json-integer-guard-dedup
target: develop
---

# Cleanup 299 — remove dead duplicate JSON integer guard

## Goal

Keep the lexical out-of-range JSON integer scan as the sole authoritative
mechanism and remove the documented-dead visitor path.

## Acceptance Criteria

- i64/u64 boundaries, negative boundaries, and normal integers retain their
  existing behavior.
- Format, clippy, and workspace tests pass.

## References

- Issue #299: https://github.com/randlee/sc-compose/issues/299
