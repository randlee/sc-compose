---
id: L.17
title: sc-lint Script Inventory and Packaging Issue
phase: L
status: planned
branch: sprint/l-17-sc-lint-script-packaging
worktree: ../sc-compose-worktrees/sprint/l-17-sc-lint-script-packaging
target: integrate/phase-l
---

# Sprint L.17 — sc-lint Script Inventory and Packaging Issue

## Goal

After all L.1-L.16 implementation sprints complete, create one actionable
GitHub issue in `randlee/sc-lint` that inventories Python scripts used by
sc-compose/sc-lint, records representative evidence, and recommends a
pip-installable distribution for the commonly used utilities.

## Hard Dependencies

L.1 through L.16 must be complete on `integrate/phase-l`. This sprint is not
part of the parallel target wave and must not be started early: its inventory
must describe the final integrated command/report surface.

## Parallel Execution

This sprint has no parallel Phase L sprint. L.17 starts only after L.1-L.16
and all required cleanup worktrees are merged and QA-approved.

## Exact Targets

- docs/phase-L/sc-lint-script-packaging-inventory.md

The GitHub issue created in `randlee/sc-lint` is an external deliverable. Its
URL and issue number must be recorded in the inventory together with the
sc-compose planning commit that records the result.

## Deliverables

- A complete table comparing sc-compose-owned scripts with the corresponding
  sc-lint `.just` scripts, shared adapters, tests, and Rust-owned replacements.
- Evidence for each row: exact source path, command/adapter identity, current
  0.4.0 behavior, and whether the script is reusable without being copied into
  a consumer repository.
- A concrete issue body in `randlee/sc-lint` recommending a pip-installable
  package for commonly used scripts, including package layout, console/module
  entrypoints, resource loading, version/schema compatibility, cross-platform
  behavior, and migration/acceptance tests.
- A link to the related maturin/Python bindings issue #83, with scope
  boundaries that prevent the two issues from duplicating one another.
- Recorded issue URL/number and the final inventory document linking to the
  exact issue text.

## Required Work

- Inspect the final sc-compose tree and the pinned sc-lint 0.4.0 source/release;
  do not infer script ownership from filenames alone.
- Include the reusable scripts (`lint_sc_boundary.py` as a legacy helper,
  `lint_sc_portability.py`,
  `lint_line_counts.py`, `lint_identity_literals.py`, `view_findings.py`,
  `python_adapter.py`, and the profile runner) and explicitly list targets
  with no Python implementation (`sc-boundary`, `sc-runtime`, check, clippy).
- Document the current consumer-root resolution problem: the Rust adapter
  looks for Python utilities under the analyzed repository's `.just/` path.
- Recommend one ownership model that avoids per-repository duplication, such
  as an embedded/resource-backed Python package with a stable module entrypoint
  invoked by sc-lint. Include a fallback packaging model only if it preserves
  the same versioned JSON contract.
- Create exactly one final inventory issue with `gh issue create --repo
  randlee/sc-lint`, then record its URL and verify the remote body contains the
  inventory, evidence, link to #83, and concrete recommendations.

## This Sprint Does Not Close

- It does not implement pip packaging or modify the sc-lint repository.
- It does not copy Python scripts into sc-compose.
- It does not change sc-compose runtime behavior or claim the packaging gap is
  fixed by creating the issue.

## Acceptance Criteria

- The inventory covers every Python script proposed for reuse and every L
  target, with an explicit “no representative script” result where applicable.
- Every reusable-script row names its sc-lint source path and at least one
  supporting test or adapter contract.
- The issue has a stable URL/number, is in `randlee/sc-lint`, and contains
  concrete pip-install/package-layout, entrypoint, resource, compatibility,
  and migration recommendations.
- The issue links #83, explains the scope split, and does not restate the
  maturin implementation work as if it were complete.
- The issue explains why copying `.just` scripts into sc-compose is rejected
  and identifies the current 0.4.0 consumer-root failure mode.
- The inventory and issue are internally consistent and contain no claim that
  packaging has already been implemented.

## sc-lint Cleanup Routing

Run the applicable sc-lint documentation/configuration checks on the final
sprint commit. Fix minor documentation findings immediately. If a remaining
finding affects the inventory or issue evidence, create
`fix/l-17-docs-<class>` from this sprint worktree's final commit; keep issue
scope corrections separate from document-length refactors. Send the worktree
and fix commit to team-lead for PR creation; team-lead sends the PR to
quality-mgr for QA. L.17 cannot close until required fixes are QA-approved,
merged, and the remote issue/inventory are rechecked.

## Required Validation

- `rg`/source inspection of the final sc-compose tree and pinned sc-lint 0.4.0
- `sc-lint version --json`
- `gh issue view <recorded-number> --repo randlee/sc-lint --json number,url,title,body`
- `git diff --check`

This is a documentation/external-coordination sprint and produces no
executable artifact; `cargo test --workspace` is not a closure gate for this
sprint.
