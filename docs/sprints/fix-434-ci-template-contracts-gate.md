---
id: FIX-434
title: Gate production template-contracts in CI, close scanner-parity nit
status: complete
branch: fix/o6-ci-template-contracts-gate
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/o6-ci-template-contracts-gate
target: integrate/phase-o
---

## Root Cause

comp's Phase O production-readiness review (REVIEW-433) found:

- Finding 1 (BLOCKER): `just lint-ci-consumer` never actually ran the
  repository-level `template-contracts` scanner in CI — the gate existed as a
  documented target but had no enforcing invocation.
- Finding 4 (IMPORTANT): `docs/phase-O/o5-task-checklist.md`'s O5-015 item and
  its mirrored "Final verification" bullet were still unchecked despite the
  underlying work being complete.
- Finding 7 (MINOR): the Jinja comment/raw-block/variable-expression scanner
  in `crates/sc-composer/src/validation/diagnostics.rs` and the equivalent
  scanner in `crates/sc-compose/src/commands/template_lint.rs` had drifted out
  of parity with no shared implementation or test.

## Fix Design

- Add a real, enforcing `template-contracts` invocation to the Justfile's
  `lint-ci-consumer` recipe, scoped to `production` via
  `SC_COMPOSE_TEMPLATE_CONTRACTS_SCOPE=production`, asserting a structured
  pass result with `jq -e` under `set -euo pipefail` so a non-pass result
  fails CI.
- Exclude known negative/non-production fixtures from the production scope
  while keeping them exercised by existing targeted tests
  (`sc_lint_template_contracts.rs`, `extract_integration.rs`).
- Add a scanner-parity regression test comparing the two scanners' output on
  a shared fixture (later fully deduplicated into one shared implementation
  by FIX-439/PR #436).
- Check the O5-015 checklist item and mirrored bullet, citing the new CI gate
  as closing rationale.

## Required Changes / Tests

- `Justfile`: `lint-ci-consumer` recipe gains the scoped, asserting
  `template-contracts` invocation.
- `crates/sc-composer/src/validation/diagnostics.rs`: scanner-parity fix
  (`next_json_variable_expression` skips Jinja comments/raw blocks), new test
  `quoted_json_scanner_skips_comments_raw_blocks_and_literals`.
- `docs/phase-O/o5-task-checklist.md`: O5-015 and Final verification bullet
  checked.

## Out of Scope

- Full scanner deduplication into a shared module — deferred to a follow-up
  (delivered by FIX-439/PR #436).

## Acceptance Criteria

- Production-scoped `template-contracts` scan runs in CI and fails the build
  on any finding.
- Excluded negative/non-production fixtures remain covered by existing tests.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --workspace`, `git diff --check` all clean.

## References

- REVIEW-433-PHASE-O-PRODUCTION-READINESS, Findings 1, 4, 7.
- QA-434PR-O6-CI-TEMPLATE-CONTRACTS-GATE verdict (PASS).

## Priority

High — closed a BLOCKER (CI gate never ran) from the phase's own
production-readiness review.

## Closeout Evidence

- implementation commit: `d3a683d`
- PR: https://github.com/randlee/sc-compose/pull/434 (merged
  `fda36d57a14b4eb80b131320c8ac88139c656538`)
- QA-434PR: PASS, 5/5 deliverables met, CI 12/12 green.
