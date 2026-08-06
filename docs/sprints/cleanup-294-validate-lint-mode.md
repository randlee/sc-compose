---
id: CLEANUP-294
title: Opt-in template-lint validate mode
status: complete
branch: cleanup/validate-template-lint-diagnostics
worktree: ../sc-compose-worktrees/cleanup/validate-template-lint-diagnostics
target: develop
---

# Cleanup 294 — opt-in template-lint validate mode

## Goal

Add a non-default lint mode to `validate` that reports common template
mistakes, beginning with the redundant `frontmatter_safe|yaml_safe` chain.

## Required Fix

- Add an opt-in `--lint` flag to `validate`.
- Report file/line and an actionable recommended filter.
- Keep default `validate` output unchanged.

## Acceptance Criteria

- Default-output regression test passes.
- `validate --lint` reports the known chain with file/line/recommendation.
- `--help`, format, clippy, and workspace tests pass.

## References

- Issue #294: https://github.com/randlee/sc-compose/issues/294
