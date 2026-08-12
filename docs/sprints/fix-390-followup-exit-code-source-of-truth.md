---
id: FIX-390-FOLLOWUP
title: "FIX-390 follow-up: exit-code single source of truth + project-plan.md sha sync"
status: assigned
branch: fix/390-followup-exit-code-source-of-truth
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/390-followup-exit-code-source-of-truth
target: crates/sc-compose/src/main.rs, docs/project-plan.md
---

## Root Cause

Two non-blocking minor findings surfaced during FIX-390 QA
(`docs/sprints/fix-390-clap-exit-code-fr7b.md`, PR #391), both routed here per
the standing "fix every finding, no unrouted backlog" rule:

1. (simplification-reviewer) `report_cli_parse_error`
   (`crates/sc-compose/src/main.rs`) computes its returned exit code locally
   (`exit_codes::USAGE_FAIL`) instead of reading it from
   `CommandError::exit_code`, which `usage_with_code` already sets to the
   same value. Two independent sources of truth for the same fact. This gap
   pre-existed FIX-390 and was not widened by it, but should be closed.
2. (req-qa + arch-qa, independently) `docs/project-plan.md`'s FIX-390 status
   line cites completion commit `ff5d74b`, one commit behind the actual
   branch HEAD `6528f67` at merge time. Cosmetic sha sync.

## Fix Design

1. In `report_cli_parse_error`, build the `CommandError` via
   `usage_with_code` first, then return `command_error.exit_code` instead of
   a separately-computed `exit_codes::USAGE_FAIL` literal, so there is one
   source of truth for the exit code on this path. Keep behavior identical
   (both currently resolve to the same value `3`).
2. Update `docs/project-plan.md`'s FIX-390 entry to cite the correct final
   commit `6528f67` (or whatever the actual merge-commit SHA is once #391 is
   merged into develop — check `git log --oneline -1 -- docs/sprints/fix-390-clap-exit-code-fr7b.md`
   on develop for the accurate reference).

## Required Changes / Tests

- `crates/sc-compose/src/main.rs`: single source of truth for the exit code
  in `report_cli_parse_error`.
- `docs/project-plan.md`: correct FIX-390 commit reference.
- No behavior change — existing FIX-390 regression tests must continue to
  pass unmodified.

## Out of Scope

- Any other exit-code call site; this is scoped to `report_cli_parse_error`
  only.
- Re-litigating FIX-390's own scope or design.

## Acceptance Criteria

1. `report_cli_parse_error` returns `CommandError::exit_code` rather than a
   separately-computed literal.
2. `docs/project-plan.md`'s FIX-390 entry cites the correct commit SHA.
3. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
   `cargo test --workspace` all clean.
4. `docs/project-plan.md` gets its own Follow-on Fix Sprint entry for this
   follow-up sprint.

## References

- FIX-390 QA verdict (PR #391), simplification-reviewer + req-qa/arch-qa
  findings.
- `docs/sprints/fix-390-clap-exit-code-fr7b.md`

## Priority

Minor, non-blocking cleanup — no release impact.
