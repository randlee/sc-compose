---
id: L.8
title: Findings View
phase: L
status: planned
branch: sprint/l-8-view-findings
worktree: ../sc-compose-worktrees/sprint/l-8-view-findings
target: integrate/phase-l
---

# Sprint L.8 — Findings View

## Goal

Integrate the single sc-lint command target view findings through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.
L.1's bootstrap contract must first record whether the pinned sc-lint 0.4.0
distribution can resolve `.just/view_findings.py`, `.just/view_common.py`, and
`.just/python_adapter.py` without a consumer-side copy; this unresolved
packaging question is tracked in sc-lint issue #83. If it cannot, this sprint
must characterize the explicit `CLI.CONFIG_ERROR` class/non-pass result and
actionable diagnostic rather than adding copied scripts.

## Parallel Execution

After L.2 is merged, L.8 may execute in parallel with L.3, L.4, L.5, L.6,
L.7, L.9, L.10, L.11, L.12, L.13, L.14, L.15, and L.16. It does not run in
parallel with prerequisite L.1/L.2 or final L.17.

## Exact Targets

- .sc/sc-lint/targets/view-findings.toml
- tests/fixtures/sc-lint/view-findings/
- crates/sc-compose/tests/sc_lint_view_findings.rs
- reports/inputs/lint/view-findings/

## Deliverables

- One declarative target descriptor mapping view findings to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Use a stored findings payload and verify the view is rendered as a report without changing finding identity or losing raw JSON fields.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- Representative script: `../sc-lint/.just/view_findings.py`.
- Supporting sources/tests: `../sc-lint/.just/view_common.py`,
  `../sc-lint/.just/python_adapter.py`, and
  `../sc-lint/.just/tests/test_view_findings.py`. Reuse through sc-lint and
  retain the structured findings contract; do not copy a viewer into sc-compose.

## sc-lint Cleanup Routing

Run `view findings` and the report-path checks on the final sprint commit. Fix
minor report schema, HTML/template, or identity findings immediately. For
remaining findings, create `fix/l-8-<class>-<owner>` from this sprint
worktree's final commit; keep schema/serialization changes separate from
rendering refactors, and keep each length refactor separate. Group same-crate
constant strings rather than individual literals. Send the worktree and fix
commit to team-lead for PR creation; team-lead sends it to quality-mgr for QA.
L.8 cannot close until fixes are QA-approved, merged, and revalidated.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . view findings

The focused test must assert this report identity:

    command = "view.findings"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- When L.1 records the Python utility as resolvable, the positive fixture
  produces a pass report with command view.findings and a retained raw JSON
  artifact. When L.1 records it as unresolvable, the positive-path
  characterization instead produces the documented CLI.CONFIG_ERROR-class
  structured diagnostic with retained raw JSON and report evidence.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command view findings.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

- All required cleanup fixes are QA-approved, merged, and revalidated before sprint closure.

## Required Validation

- just view findings
- cargo test -p sc-compose --test sc_lint_view_findings
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
