---
id: S.9
title: Go Native Module Remediation Plan
status: complete
branch: sprint/s-9-go-native-module-remediation-plan
worktree: ../sc-compose-worktrees/sprint/s-9-go-native-module-remediation-plan
target: sprint/s-8-beads-runner-reliability
---

# Sprint S.9 — Go Native Module Remediation Plan

## Goal

Produce a written remediation plan for GitHub issue #583 (the `sc-sha-go`
bundle job calling a `stage-go-native-module` subcommand that no longer exists
in `.github/scripts/release_artifacts.py`, dropped during the sc-publish
cutover in `71b9d7f`). This sprint is **planning-only**: no changes to
`release_artifacts.py`, `ci.yml`, or any other production/CI source file.

## Hard Dependencies

- S.8 is this branch's required `gh stack` parent. There is no functional code
  dependency on S.1–S.8.

## Exact Targets

- New document: `docs/plans/go-native-module-remediation.md`
- Read-only investigation of:
  - `.github/workflows/ci.yml` (`sc-sha-go-plan`, `sc-sha-go` jobs)
  - `.github/scripts/release_artifacts.py`
  - `.github/workflows/release.yml` (reference pattern for target-matrix usage)
  - commit `71b9d7f` (sc-publish cutover) and commit `fe2614d` / PR #515
    (Phase P.2, original `stage-go-native-module` addition)
  - GitHub issue #583

## Deliverables

- `docs/plans/go-native-module-remediation.md` containing:
  - Confirmed root cause narrative (verify/refine the issue #583 analysis
    against actual `sc-publish` capabilities, not just restate it).
  - An explicit determination of whether `sc-publish` already provides an
    equivalent Go-native-module staging capability that should be
    vendored/wrapped, or whether this logic must be reimplemented directly in
    `release_artifacts.py`.
  - A concrete, reviewable remediation plan: proposed subcommand contract
    (inputs/outputs), file(s) to change, and an explicit list of the exact
    `ci.yml` lines that need to change (including the unrelated
    `matrix.goos`/`matrix.goarch` naming bug in the `sc-sha-go` job's `name:`
    field, noted in issue #583).
  - A verification plan: how the fix will be proven correct (which CI jobs
    must go green, on which matrix targets).
  - Explicit non-goals: this sprint does not implement any of the proposed
    changes.
- No modification to `release_artifacts.py`, `ci.yml`, `release.yml`, or any
  other production/CI source file.

## Required Work

- Read issue #583 in full and verify each claim in it against the current
  repository state (do not simply copy the issue body into the plan).
- Identify what, if anything, `sc-publish` (as the canonical release tool this
  repo migrated to per the Phase Q cutover) already provides for Go-native
  module staging, and cite the specific evidence used to reach that
  conclusion.
- The plan must be directly actionable by a future dev sprint without further
  investigation — a reviewer should be able to approve or reject it without
  needing to re-derive the root cause.

## This Sprint Does Not Close

- The actual `stage-go-native-module` fix (a future sprint, dispatched only
  after this plan is reviewed and approved).
- PR #582's merge (it remains held open, red CI, pending the approved fix).

## Acceptance Criteria

- [x] `docs/plans/go-native-module-remediation.md` exists and is internally
  consistent with the current repository state (not just a restatement of
  issue #583).
- [x] The plan states a clear, single recommended remediation path (not just a
  list of options) with rationale.
- [x] The plan lists exact file paths and line ranges expected to change.
- [x] No production or CI source file is modified in this sprint.

## gh-stack Workflow

```bash
# The phase plan added this branch directly on top of S.8.
git config rerere.enabled true
git config remote.pushDefault origin
git add docs/plans/go-native-module-remediation.md docs/phase-S/sprint-s-9-go-native-module-remediation-plan.md
git commit -m "docs(plan): draft go-native-module remediation plan (issue #583)"
gh stack submit --auto
gh pr ready <sprint-s-9-pr-number>
gh stack view --json
# Do not merge an individual sprint layer; phase close merges the full stack.
```

## Required Validation

- `git diff --check`
- Manual read-through confirming no non-doc files changed:
  `git diff --stat sprint/s-8-beads-runner-reliability..HEAD`
