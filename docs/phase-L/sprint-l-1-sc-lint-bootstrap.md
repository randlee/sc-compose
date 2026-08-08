---
id: L.1
title: sc-lint Repository and Tool Bootstrap
phase: L
status: planned
branch: sprint/l-1-sc-lint-bootstrap
worktree: ../sc-compose-worktrees/sprint/l-1-sc-lint-bootstrap
target: integrate/phase-l
---

# Sprint L.1 — sc-lint Repository and Tool Bootstrap

## Goal

Establish the repository, tool-version, boundary-inventory, and CI prerequisites
that let sc-lint 0.4.0 analyze sc-compose successfully.

## Hard Dependencies

This sprint starts from develop/origin/develop and requires the Phase L
plan-gate approval. It must land before L.2 and all target sprints.

## Parallel Execution

This sprint has no parallel Phase L sprint. It is the serial prerequisite for
L.2 and L.3-L.16; L.17 also waits for all earlier sprints.

## Exact Targets

- boundaries/
- sc-lint.toml
- .github/actions/setup-sc-lint/action.yml
- docs/phase-L/sc-lint-bootstrap-contract.md
- docs/adrs/0016-sc-lint-integration-boundary.md
- crates/sc-compose/tests/repo_boundaries.rs
- tests/fixtures/sc-lint/bootstrap/

## Deliverables

- A complete sc-compose boundary inventory in the canonical TOML layout
  required by sc-lint 0.4.0, covering sc-composer, sc-compose, and the Python
  adapter without introducing forbidden dependency edges.
- A repository-owned sc-lint configuration with an explicit supported version
  of 0.4.0, logging/artifact locations, and no analyzer-rule duplication.
- A cross-platform CI setup step that installs or verifies the pinned sc-lint
  release and its sibling backends, then fails with an actionable diagnostic
  when the required version is unavailable.
- Documentation of the external-tool ownership boundary: sc-lint analyzes;
  sc-compose invokes, normalizes, and reports.
- ADR-0016 recording the ownership, version-pinning, and no-duplicate-runner
  decision for this integration.
- Bootstrap characterization proving the command can discover the root and
  returns a machine-readable version result.

## Required Work

- Keep the sc-composer library free of CLI, process, and sc-lint concerns.
- Make the boundary inventory reflect actual workspace packages and dependency
  direction; do not copy sc-lint's own boundary files verbatim.
- Make version verification use sc-lint version --json, not human-output
  parsing and not a duplicated Python checker.
- Add a CI smoke step that runs a harmless version/root-discovery command
  before any analyzer target.
- Before creating the ADR, inspect `docs/adrs/` and confirm 0016 remains the
  next open number; if another phase lands an ADR first, select the next open
  number and update this sprint's exact target and references consistently.

## sc-lint Reuse Reference

- Repository/config evidence: `../sc-lint/crates/sc-lint/src/config.rs`,
  `../sc-lint/crates/sc-lint/src/python_adapter.rs`, and
  `../sc-lint/.just/lint-config.toml`.
- No Python script should be added for version or root discovery. Reuse the
  installed sc-lint CLI contract and record whether the release makes its
  Python-backed utilities available without a consumer `.just/` copy.
- This is the prerequisite documented for the final L.17 inventory and the
  related maturin/Python bindings issue #83.

## sc-lint Cleanup Routing

Run the bootstrap sc-lint smoke targets on the final sprint commit. Fix minor
boundary, manifest, configuration, or CI portability findings immediately in
this worktree. For remaining findings, create `fix/l-1-<class>-<owner>` from
this sprint worktree's final commit; keep boundary/manifest findings separate
from portability/config refactors, and group constant-string findings by
owning crate rather than by finding. Send the worktree/branch, parent commit,
evidence, tests, and fix commit to team-lead for PR creation; team-lead sends
the PR to quality-mgr for independent QA. L.1 cannot close until those fixes
are QA-approved, merged, and revalidated.

## Explicit Code Samples

The repository-owned version contract must be equivalent to:

~~~
SC_LINT_VERSION=0.4.0
sc-lint version --json
sc-lint --json --root . lint sc-boundary
~~~

The boundary inventory must keep the existing dependency direction:

~~~
sc-compose -> sc-composer
bindings/python -> sc-composer
sc-composer -/-> sc-compose
sc-composer -/-> bindings/python
~~~

## This Sprint Does Not Close

- It does not add the shared sc-compose lint command or HTML report
  materialization; those are L.2.
- It does not claim any analyzer target is integrated merely because root
  discovery succeeds.
- It does not add view graph, which is reserved by sc-lint 0.4.0.

## Acceptance Criteria

- sc-lint version --json reports version 0.4.0 in the repository's smoke path.
- Every sc-lint command that requires a repository root can discover sc-compose
  without CLI.CONFIG_ERROR caused by a missing boundaries/ directory.
- The inventory describes every workspace package and no forbidden edge.
- CI verifies the same tool version on supported runners.
- The bootstrap contract explicitly records whether the pinned distribution
  supplies Python-backed utilities to a clean consumer without a `.just/`
  script copy and links sc-lint issue #83 if it does not.
- No Python runner or analyzer implementation is added to sc-compose.

- All required cleanup fixes are QA-approved, merged, and revalidated before sprint closure.

## Required Validation

- sc-lint version --json
- sc-lint --json --root . lint sc-boundary
- cargo test -p sc-compose --test repo_boundaries
- cargo fmt --all --check
- git diff --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --workspace
