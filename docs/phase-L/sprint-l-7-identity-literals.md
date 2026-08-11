---
id: L.7
title: Identity-Literal Utility
phase: L
status: complete
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
L.1's bootstrap contract must first record whether the pinned sc-lint 0.4.0
distribution can resolve `.just/lint_identity_literals.py` and
`.just/python_adapter.py` without a consumer-side copy; this unresolved
packaging question is tracked in sc-lint issue #83. If it cannot, this sprint
must characterize the explicit `CLI.CONFIG_ERROR` class/non-pass result and
actionable diagnostic rather than adding copied scripts.

## Parallel Execution

After L.2 is merged, L.7 may execute in parallel with L.3, L.4, L.5, L.6,
L.8, L.9, L.10, L.11, L.12, L.13, L.14, L.15, and L.16. It does not run in
parallel with prerequisite L.1/L.2 or final L.17.

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

## sc-lint Reuse Reference

- Representative script: `../sc-lint/.just/lint_identity_literals.py`.
- Shared adapter and tests: `../sc-lint/.just/python_adapter.py` and
  `../sc-lint/.just/tests/test_lint_identity_literals.py`. Reuse the script
  through sc-lint; do not duplicate it or its adapter in sc-compose.

## sc-lint Cleanup Routing

Run `lint identity-literals` on the final sprint commit. Fix minor identity or
constant-string findings immediately. For remaining findings, create
`fix/l-7-identity-<crate>` from this sprint worktree's final commit, normally
one worktree per owning crate and at most three where ownership/conflict risk
requires a split—not one per literal. Keep length refactors and semantic
identity redesigns separate. Send each worktree and fix commit to team-lead
for PR creation; team-lead sends the PR to quality-mgr for QA. L.7 cannot
close until fixes are QA-approved, merged, and revalidated.

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

- When L.1 records the Python utility as resolvable, the positive fixture
  produces a pass report with command lint.identity-literals and a retained raw
  JSON artifact. When L.1 records it as unresolvable, the positive-path
  characterization instead produces the documented CLI.CONFIG_ERROR-class
  structured diagnostic with retained raw JSON and report evidence.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command lint identity-literals.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

- All required cleanup fixes are QA-approved, merged, and revalidated before sprint closure.

## Required Validation

- just lint identity-literals
- cargo test -p sc-compose --test sc_lint_identity_literals
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
