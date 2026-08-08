---
id: L.16
title: Top-Level CI
phase: L
status: planned
branch: sprint/l-16-ci
worktree: ../sc-compose-worktrees/sprint/l-16-ci
target: integrate/phase-l
---

# Sprint L.16 — Top-Level CI

## Goal

Integrate the single sc-lint command target ci through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Parallel Execution

After L.2 is merged, L.16 may execute in parallel with L.3, L.4, L.5, L.6,
L.7, L.8, L.9, L.10, L.11, L.12, L.13, L.14, and L.15. It does not run in
parallel with prerequisite L.1/L.2 or final L.17.

## Exact Targets

- .sc/sc-lint/targets/ci.toml
- tests/fixtures/sc-lint/ci/
- crates/sc-compose/tests/sc_lint_ci.rs
- reports/inputs/lint/ci/

## Deliverables

- One declarative target descriptor mapping ci to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Verify top-level ci runs the CI lint profile plus cargo test --workspace, and distinguish it from lint ci and from xwin preflight.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- No separate Python runner exists for top-level ci. Use
  `../sc-lint/crates/sc-lint/src/workflow.rs` as the authoritative composition
  of lint ci plus workspace tests, and reuse any transitively configured
  utilities through sc-lint. Do not add a sc-compose Python wrapper.

## sc-lint Cleanup Routing

Run top-level `ci` on the final sprint commit. Fix minor CI composition,
workflow, or command-identity findings immediately. For remaining findings,
create `fix/l-16-ci-<class>-<owner>` from this sprint worktree's final commit;
route lint-target findings to their originating sprint instead of duplicating
them here. For each routed finding, write a note keyed by `<rule-id> +
<file-path>` under `reports/inputs/lint/ci/`, naming the atomic owner sprint
and known fix branch/PR. Keep workflow changes separate from crate-level
constants and keep each length refactor separate. Send the worktree or routing
note and its commit to team-lead for PR creation; team-lead sends the PR to
quality-mgr for QA. L.16 cannot close until fixes are QA-approved, merged, and
rerun.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . ci

The focused test must assert this report identity:

    command = "ci"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command ci and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command ci.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

- All required cleanup fixes are QA-approved, merged, and revalidated before sprint closure.

## Required Validation

- just ci
- cargo test -p sc-compose --test sc_lint_ci
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
