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

Add a non-default lint mode to `validate` that reports the observed redundant
`frontmatter_safe|yaml_safe` chain. This cleanup intentionally scopes the
detector to that single pattern; it does not generalize linting into a rule
engine.

## Required Fix

- Add an opt-in `--lint` flag to `validate`.
- Report file/line and an actionable recommended filter.
- Keep default `validate` output unchanged.
- Keep the detector as a single hardcoded match for the observed
  `frontmatter_safe|yaml_safe` chain.
- Treat additional redundant or incorrect filter chains as out of scope; file
  them as separate follow-ups only when a real observed case justifies the
  maintenance cost.

## Acceptance Criteria

- Default-output regression test passes.
- `validate --lint` reports only the known chain with file/line/recommendation.
- No rule-engine abstraction or speculative additional pattern detection is
  required by this sprint.
- `--help`, format, clippy, and workspace tests pass.

## References

- Issue #294: https://github.com/randlee/sc-compose/issues/294
