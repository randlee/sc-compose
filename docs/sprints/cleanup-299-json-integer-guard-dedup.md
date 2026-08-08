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

## Closeout Evidence

Validation was independently re-run for commit `e7e2e4f`:

- `cargo test --workspace` — PASS; all workspace tests passed with 0 failures.
- `cargo clippy --all-targets --all-features -- -D warnings` — PASS; 0 warnings.
- `cargo fmt --all --check` — PASS; no formatting changes required.

The standing red/green regression-test protocol does not require a red commit
for this removal-only cleanup: the dead visitor path was deleted while the
existing integer-boundary tests already prove the retained lexical scan's
behavior, so no new behavior scenario was introduced.
