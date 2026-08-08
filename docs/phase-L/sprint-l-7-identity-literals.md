---
id: L.7
title: Identity-Literal Utility
phase: L
status: planned
branch: sprint/l-7-identity-literals
worktree: ../sc-compose-worktrees/sprint/l-7-identity-literals
target: integrate/phase-l
---

# Sprint L.7 — Identity-Literal Utility

## Goal

Integrate the single sc-lint command target lint identity-literals through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Exact Targets

- .sc/sc-lint/targets/identity-literals.toml
- tests/fixtures/sc-lint/identity-literals/
- crates/sc-compose/tests/sc_lint_identity_literals.rs
- reports/inputs/lint/identity-literals/

## Deliverables

- One declarative target descriptor mapping lint identity-literals to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Use a fixture with approved identity values and one with an undeclared or duplicated identity literal. Preserve the Python-backed adapter schema and diagnostics.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . lint identity-literals

The focused test must assert this report identity:

    command = "lint.identity-literals"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command lint.identity-literals and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command lint identity-literals.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

## Required Validation

- just lint identity-literals
- cargo test -p sc-compose --test sc_lint_identity_literals
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
