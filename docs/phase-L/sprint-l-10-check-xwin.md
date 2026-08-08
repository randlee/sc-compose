---
id: L.10
title: Windows Cross-Target Check
phase: L
status: planned
branch: sprint/l-10-check-xwin
worktree: ../sc-compose-worktrees/sprint/l-10-check-xwin
target: integrate/phase-l
---

# Sprint L.10 — Windows Cross-Target Check

## Goal

Integrate the single sc-lint command target check xwin through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Parallel Execution

After L.2 is merged, L.10 may execute in parallel with L.3, L.4, L.5, L.6,
L.7, L.8, L.9, L.11, L.12, L.13, L.14, L.15, and L.16. It does not run in
parallel with prerequisite L.1/L.2 or final L.17.

## Exact Targets

- .sc/sc-lint/targets/check-xwin.toml
- tests/fixtures/sc-lint/check-xwin/
- crates/sc-compose/tests/sc_lint_check_xwin.rs
- reports/inputs/lint/check-xwin/

## Deliverables

- One declarative target descriptor mapping check xwin to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Run the cargo xwin check path when the capability is available; when unavailable, preserve the explicit capability error and never silently report pass.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- No representative Python helper exists for check xwin in sc-lint 0.4.0.
  Use `../sc-lint/crates/sc-lint/src/workflow.rs` and
  `../sc-lint/crates/sc-lint/src/command.rs` as the Rust workflow/identity
  contract; do not add a Python wrapper.

## sc-lint Cleanup Routing

Run `check xwin` on the final sprint commit when the capability is available,
and preserve explicit capability results otherwise. Fix minor xwin/cfg,
portability, process, or command-identity findings immediately. For remaining
findings, create `fix/l-10-<class>-<owner>` from this sprint worktree's final
commit; keep cfg/portability fixes separate from process refactors and keep
length refactors one per violation. Send the worktree and fix commit to
team-lead for PR creation; team-lead sends the PR to quality-mgr for QA. L.10
cannot close until fixes are QA-approved, merged, and revalidated.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . check xwin

The focused test must assert this report identity:

    command = "check.xwin"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.
- It does not make xwin a required host capability; unavailable xwin must remain an explicit, testable result.

## Acceptance Criteria

- The positive fixture produces a pass report with command check.xwin and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command check xwin.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

## Required Validation

- just check xwin
- cargo test -p sc-compose --test sc_lint_check_xwin
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
