---
id: L.4
title: Portability Analyzer
phase: L
status: planned
branch: sprint/l-4-sc-portability
worktree: ../sc-compose-worktrees/sprint/l-4-sc-portability
target: integrate/phase-l
---

# Sprint L.4 — Portability Analyzer

## Goal

Integrate the single sc-lint command target lint sc-portability through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Parallel Execution

After L.2 is merged, L.4 may execute in parallel with L.3, L.5, L.6, L.7,
L.8, L.9, L.10, L.11, L.12, L.13, L.14, L.15, and L.16. It does not run in
parallel with prerequisite L.1/L.2 or final L.17.

## Exact Targets

- .sc/sc-lint/targets/sc-portability.toml
- tests/fixtures/sc-lint/sc-portability/
- crates/sc-compose/tests/sc_lint_sc_portability.rs
- reports/inputs/lint/sc-portability/

## Deliverables

- One declarative target descriptor mapping lint sc-portability to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Use a representative portable source fixture and a platform-specific path, environment, shell, or cfg-parity violation fixture. Preserve the sc-portability rule family and finding payload.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- Representative script: `../sc-lint/.just/lint_sc_portability.py`.
- Supporting tests/contract: `../sc-lint/.just/tests/test_lint_sc_portability.py`
  and `../sc-lint/crates/sc-lint/src/dispatch.rs`. Reuse this logic through
  the supported sc-lint CLI; do not copy the script into sc-compose.

## sc-lint Cleanup Routing

Run `lint sc-portability` on the final sprint commit. Fix minor path,
environment, shell, cfg, or platform-gating findings immediately. For
remaining findings, create `fix/l-4-<class>-<owner>` from this sprint
worktree's final commit; keep path/env/shell/cfg changes in separate worktrees
unless the same mechanical edit owns them. Group same-crate constant strings
together and keep length refactors one per violating file/refactor. Send the
worktree and fix commit to team-lead for PR creation; team-lead sends it to
quality-mgr for QA. L.4 cannot close until fixes are QA-approved, merged, and
revalidated.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . lint sc-portability

The focused test must assert this report identity:

    command = "lint.sc-portability"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command lint.sc-portability and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command lint sc-portability.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

## Required Validation

- just lint sc-portability
- cargo test -p sc-compose --test sc_lint_sc_portability
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
