---
id: L.11
title: Native Clippy
phase: L
status: planned
branch: sprint/l-11-clippy-native
worktree: ../sc-compose-worktrees/sprint/l-11-clippy-native
target: integrate/phase-l
---

# Sprint L.11 — Native Clippy

## Goal

Integrate the single sc-lint command target clippy native through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Exact Targets

- .sc/sc-lint/targets/clippy-native.toml
- tests/fixtures/sc-lint/clippy-native/
- crates/sc-compose/tests/sc_lint_clippy_native.rs
- reports/inputs/lint/clippy-native/

## Deliverables

- One declarative target descriptor mapping clippy native to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Run cargo clippy --workspace --all-targets -- -D warnings through sc-lint and preserve step output and non-zero failure behavior.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- No representative Python helper exists for clippy native in sc-lint 0.4.0.
  Use `../sc-lint/crates/sc-lint/src/workflow.rs` and
  `../sc-lint/crates/sc-lint/src/command.rs` as the Rust workflow/identity
  contract; do not add a Python wrapper.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . clippy native

The focused test must assert this report identity:

    command = "clippy.native"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command clippy.native and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command clippy native.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

## Required Validation

- just clippy native
- cargo test -p sc-compose --test sc_lint_clippy_native
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
