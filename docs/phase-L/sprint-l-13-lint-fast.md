---
id: L.13
title: Fast Lint Profile
phase: L
status: planned
branch: sprint/l-13-lint-fast
worktree: ../sc-compose-worktrees/sprint/l-13-lint-fast
target: integrate/phase-l
---

# Sprint L.13 — Fast Lint Profile

## Goal

Integrate the single sc-lint command target lint fast through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Exact Targets

- .sc/sc-lint/targets/lint-fast.toml
- tests/fixtures/sc-lint/lint-fast/
- crates/sc-compose/tests/sc_lint_lint_fast.rs
- reports/inputs/lint/lint-fast/

## Deliverables

- One declarative target descriptor mapping lint fast to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Verify the exact fast profile step list and its xwin-free policy. Do not add analyzer-specific steps to the profile in this sprint.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- Representative profile sources: `../sc-lint/.just/run_lint.py` and
  `../sc-lint/crates/sc-lint/src/workflow.rs`. The profile transitively uses
  shared Python utilities such as `../sc-lint/.just/python_adapter.py`.
- Reuse profile semantics through `sc-lint lint fast`; do not copy the runner
  or utility scripts into sc-compose.

## sc-lint Cleanup Routing

Run `lint fast` on the final sprint commit. Fix minor profile/runner findings
immediately. For remaining findings, create `fix/l-13-profile-<class>` from
this sprint worktree's final commit only for profile-owned changes; route
utility/analyzer findings to their originating L sprint rather than creating a
duplicate fix. Keep profile changes, constant strings, and length refactors
separate. Send each worktree and fix commit to team-lead for PR creation;
team-lead sends the PR to quality-mgr for QA. L.13 cannot close until required
fixes are QA-approved, merged, and rerun.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . lint fast

The focused test must assert this report identity:

    command = "lint.fast"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command lint.fast and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command lint fast.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

## Required Validation

- just lint fast
- cargo test -p sc-compose --test sc_lint_lint_fast
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
