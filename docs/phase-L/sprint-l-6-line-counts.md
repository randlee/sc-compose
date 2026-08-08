---
id: L.6
title: Line-Count Utility
phase: L
status: planned
branch: sprint/l-6-line-counts
worktree: ../sc-compose-worktrees/sprint/l-6-line-counts
target: integrate/phase-l
---

# Sprint L.6 — Line-Count Utility

## Goal

Integrate the single sc-lint command target lint line-counts through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Exact Targets

- .sc/sc-lint/targets/line-counts.toml
- tests/fixtures/sc-lint/line-counts/
- crates/sc-compose/tests/sc_lint_line_counts.rs
- reports/inputs/lint/line-counts/

## Deliverables

- One declarative target descriptor mapping lint line-counts to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Use a below-limit fixture and an over-limit fixture. Treat the Python-backed adapter schema as opaque input and preserve its findings and structured error kinds.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- Representative script: `../sc-lint/.just/lint_line_counts.py`.
- Shared adapter and tests: `../sc-lint/.just/python_adapter.py` and
  `../sc-lint/.just/tests/test_lint_line_counts.py`. Reuse the script through
  sc-lint; do not duplicate it or its adapter in sc-compose.

## sc-lint Cleanup Routing

Run `lint line-counts` on the final sprint commit. Fix minor findings
immediately in this worktree. Every remaining length violation/refactor gets
its own `fix/l-6-length-<owner>` worktree branched from this sprint worktree's
final commit; do not combine unrelated refactors or hide them in a broad
cleanup. Constant-string findings, if any, use a separate crate-level fix
worktree. Send each worktree and fix commit to team-lead for PR creation;
team-lead sends it to quality-mgr for QA. L.6 cannot close until all fixes are
QA-approved, merged, and revalidated.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . lint line-counts

The focused test must assert this report identity:

    command = "lint.line-counts"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command lint.line-counts and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command lint line-counts.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

## Required Validation

- just lint line-counts
- cargo test -p sc-compose --test sc_lint_line_counts
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
