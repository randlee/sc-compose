---
id: L.14
title: Full Lint Profile
phase: L
status: planned
branch: sprint/l-14-lint-full
worktree: ../sc-compose-worktrees/sprint/l-14-lint-full
target: integrate/phase-l
---

# Sprint L.14 — Full Lint Profile

## Goal

Integrate the single sc-lint command target lint full through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Exact Targets

- .sc/sc-lint/targets/lint-full.toml
- tests/fixtures/sc-lint/lint-full/
- crates/sc-compose/tests/sc_lint_lint_full.rs
- reports/inputs/lint/lint-full/

## Deliverables

- One declarative target descriptor mapping lint full to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Verify the exact full profile step list, conditional xwin behavior, and analyzer/utility membership. Do not redefine backend rules.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- Representative profile sources: `../sc-lint/.just/run_lint.py` and
  `../sc-lint/crates/sc-lint/src/workflow.rs`. Full transitively uses
  `lint_sc_portability.py`, `lint_line_counts.py`, and
  `lint_identity_literals.py` through sc-lint's adapter contract.
- Reuse profile semantics through `sc-lint lint full`; do not copy the runner
  or utility scripts into sc-compose.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . lint full

The focused test must assert this report identity:

    command = "lint.full"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command lint.full and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command lint full.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

## Required Validation

- just lint full
- cargo test -p sc-compose --test sc_lint_lint_full
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
