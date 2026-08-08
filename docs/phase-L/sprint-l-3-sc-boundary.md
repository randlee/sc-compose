---
id: L.3
title: Boundary Analyzer
phase: L
status: planned
branch: sprint/l-3-sc-boundary
worktree: ../sc-compose-worktrees/sprint/l-3-sc-boundary
target: integrate/phase-l
---

# Sprint L.3 — Boundary Analyzer

## Goal

Integrate the single sc-lint command target lint sc-boundary through the shared L.2 runner,
with target-specific evidence and no changes to shared orchestration.

## Hard Dependencies

L.2 must be merged to integrate/phase-l. This sprint has no dependency on
any other post-infrastructure sprint and may run in parallel with all of them.

## Parallel Execution

After L.2 is merged, L.3 may execute in parallel with L.4, L.5, L.6, L.7,
L.8, L.9, L.10, L.11, L.12, L.13, L.14, L.15, and L.16. It does not run in
parallel with prerequisite L.1/L.2 or final L.17.

## Exact Targets

- .sc/sc-lint/targets/sc-boundary.toml
- tests/fixtures/sc-lint/sc-boundary/
- crates/sc-compose/tests/sc_lint_sc_boundary.rs
- reports/inputs/lint/sc-boundary/

## Deliverables

- One declarative target descriptor mapping lint sc-boundary to the canonical sc-compose
  lint runner and report kind lint.
- A passing fixture and a meaningful failing or capability-negative fixture
  exercising the actual sc-lint 0.4.0 command.
- Focused integration tests proving command identity, JSON envelope, exit status,
  diagnostics, finding payload, and report-panel materialization.
- Raw JSON and rendered-panel evidence with stable paths under the shared
  sc-lint report artifact layout.

## Required Work

- Use a representative boundary inventory pass and a deliberate dependency-policy violation fixture. Preserve sc-lint-boundary findings, rule IDs, source locations, and configuration diagnostics.
- Add only target-owned files listed under Exact Targets. Do not edit Justfile,
  the shared runner, shared report templates, or another target descriptor.
- Invoke sc-lint through the L.2 allowlist with --json --root; do not call the
  backend binary directly and do not parse human output.
- Verify that a failing analysis remains a failing command/report result rather
  than being converted to a successful report with warning text.

## sc-lint Reuse Reference

- Legacy helper for inventory evidence: `../sc-lint/.just/lint_sc_boundary.py`.
- Supported 0.4.0 behavior is Rust-backed; use
  `../sc-lint/crates/sc-lint-boundary/README.md` and
  `../sc-lint/crates/sc-lint/src/dispatch.rs` as the authoritative command
  and finding contract. Do not invoke or copy the legacy helper directly.

## sc-lint Cleanup Routing

Run `lint sc-boundary` on the final sprint commit. Fix minor boundary,
manifest, or caller-edge findings immediately. For remaining findings, create
`fix/l-3-<class>-<owner>` from this sprint worktree's final commit; keep
boundary graph changes, manifest policy changes, and unrelated refactors in
separate worktrees. Group same-crate constant-string findings together, not
one worktree per string. Send the worktree, parent commit, evidence, tests,
and fix commit to team-lead for PR creation; team-lead sends the PR to
quality-mgr for QA. L.3 cannot close until required fixes are QA-approved,
merged, and revalidated.

## Explicit Code Samples

The descriptor must resolve to this stable command identity:

    sc-lint --json --root . lint sc-boundary

The focused test must assert this report identity:

    command = "lint.sc-boundary"
    report_kind = "lint"

## This Sprint Does Not Close

- It does not change sc-lint analyzer rules, Python utilities, or backend
  schemas.
- It does not modify the shared Justfile or add a repository-specific Python
  runner.
- It does not claim any other L sprint target is integrated.

## Acceptance Criteria

- The positive fixture produces a pass report with command lint.sc-boundary and a retained
  raw JSON artifact.
- The negative/capability fixture produces the expected non-pass result,
  structured diagnostics, and a rendered panel that identifies the tested input.
- The target can be run through the standard just command lint sc-boundary.
- Target-specific integration tests pass without modifying files owned by other
  L sprints.
- No Python script or duplicated report template is introduced.

- All required cleanup fixes are QA-approved, merged, and revalidated before sprint closure.

## Required Validation

- just lint sc-boundary
- cargo test -p sc-compose --test sc_lint_sc_boundary
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
