---
id: S.5
title: Boundary Invariant Guardrails
status: complete
branch: sprint/s-5-boundary-invariant-guardrails
worktree: ../sc-compose-worktrees/sprint/s-5-boundary-invariant-guardrails
target: sprint/s-4-json-capability-seams
---

# Sprint S.5 — Boundary Invariant Guardrails

## Goal

Split the concentrated repository-boundary test into independently named
invariant-family checks while retaining every existing prohibition and required
dependency assertion. This closes S-T7.

## Hard Dependencies

- S.4 is this branch's required `gh stack` parent. There is no functional code
  dependency on S.1–S.4; the parent keeps this PR incremental.

## Exact Targets

- `crates/sc-compose/tests/repo_boundaries.rs`
- existing repository-boundary fixture locations only
- `docs/plans/phase-S.md`

## Deliverables

- Named boundary-test helpers/tests for source scanning, manifest dependency
  checks, Python-adapter checks, and required dependency presence.
- Negative regression cases proving ATM, reverse-dependency, and
  adapter-dependency patterns remain prohibited.

## Required Work

- Preserve every pre-existing forbidden pattern and required dependency check;
  the refactor may strengthen failures but may not loosen them.
- This is test-only organization; do not modify production dependency direction
  or public API behavior.
- **Production-ready closure:** every named invariant-family assertion and its
  committed negative regression coverage must land in this sprint; partial
  rule coverage does not close S-T7.

## Explicit Code Samples

```rust
// Test-only organization; each helper reports actionable failures.
fn assert_source_boundary_rules(root: &Path, violations: &mut Vec<String>);
fn assert_manifest_boundary_rules(root: &Path, violations: &mut Vec<String>);
```

## This Sprint Does Not Close

- Diagnostics-facade contract coverage (S.6).
- Path-normalization coverage (S.7) or runner lifecycle changes (S.8).
- A new boundary policy or relaxation of existing policy.

## Acceptance Criteria

- [x] Boundary failures identify the violated invariant without losing the
  aggregate failure report.
- [x] Existing standalone, adapter, and forbidden-dependency cases still fail
  exactly as before.
- [x] Every pre-existing prohibited pattern and required dependency check
  remains covered by a named invariant-family assertion.
- [x] No production dependency or public API changes occur.

## gh-stack Workflow

```bash
# The phase plan added this branch directly on top of S.4.
git config rerere.enabled true
git config remote.pushDefault origin
git add crates/sc-compose/tests/repo_boundaries.rs crates/sc-compose/tests docs/plans/phase-S.md docs/phase-S/sprint-s-5-boundary-invariant-guardrails.md
git commit -m "test(boundaries): isolate invariant guardrails"
gh stack submit --auto
gh pr ready <sprint-s-5-pr-number>
gh stack view --json
# Do not merge an individual sprint layer; phase close merges the full stack.
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-compose --test repo_boundaries -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
