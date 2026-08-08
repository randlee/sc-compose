---
id: L.9
title: Native Check
phase: L
status: planned
branch: sprint/l-9-check-native
worktree: ../sc-compose-worktrees/sprint/l-9-check-native
target: integrate/phase-l
---

# Sprint L.9 — Native Check

## Goal

Integrate the single sc-lint command target check native through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Parallel Execution

After L.2 is merged, L.9 may execute in parallel with L.3, L.4, L.5, L.6,
L.7, L.8, L.10, L.11, L.12, L.13, L.14, L.15, and L.16. It does not run in
parallel with prerequisite L.1/L.2 or final L.17.

## Exact Targets

- .sc/sc-lint/targets/check-native.toml
- tests/fixtures/sc-lint/check-native/
- crates/sc-compose/tests/sc_lint_check_native.rs
- reports/inputs/lint/check-native/

## Deliverables

- One declarative target descriptor mapping check native to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Run cargo check --workspace through sc-lint and verify command, step, status, and failure output are preserved.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- No representative Python helper exists for check native in sc-lint 0.4.0.
  Use `../sc-lint/crates/sc-lint/src/workflow.rs` and
  `../sc-lint/crates/sc-lint/src/command.rs` as the Rust workflow/identity
  contract; do not add a Python wrapper.

## sc-lint Cleanup Routing

Run `check native` on the final sprint commit. Fix minor process, command
identity, portability, or workflow findings immediately. For remaining
findings, create `fix/l-9-<class>-<owner>` from this sprint worktree's final
commit; keep process/workflow changes separate from crate-level constants and
one length refactor per worktree. Send the worktree and fix commit to
team-lead for PR creation; team-lead sends the PR to quality-mgr for QA. L.9
cannot close until fixes are QA-approved, merged, and revalidated.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . check native

The focused test must assert this report identity:

    command = "check.native"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command check.native and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command check native.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

- All required cleanup fixes are QA-approved, merged, and revalidated before sprint closure.

## Required Validation

- just check native
- cargo test -p sc-compose --test sc_lint_check_native
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
