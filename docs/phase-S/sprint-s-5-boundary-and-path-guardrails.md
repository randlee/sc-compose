---
id: S.5
title: Boundary and Path Guardrails
status: complete
branch: sprint/s-5-boundary-and-path-guardrails
worktree: ../sc-compose-worktrees/sprint/s-5-boundary-and-path-guardrails
target: integrate/phase-s
---

# Sprint S.5 — Boundary and Path Guardrails

## Goal

Turn the concentrated repository-boundary test into independently named
invariants and add focused diagnostics-facade and path-helper coverage. This
closes S-T7, S-T9, and S-T10 without changing the rules or public contracts
those checks enforce.

## Hard Dependencies

- `integrate/phase-s` exists.
- S.1 through S.4 have merged into `integrate/phase-s`; this sprint
  merge-forwards that tip so the full guardrail suite measures the phase result.

## Exact Targets

- `crates/sc-compose/tests/repo_boundaries.rs`
- `crates/sc-composer/src/diagnostics.rs`
- `crates/sc-composer/src/diagnostics/{envelope,filesystem,record,schema}.rs`
- `crates/sc-compose/src/path_utils.rs`
- `crates/sc-compose/src/reporting/publish_manifest/tests.rs`
- `docs/plans/phase-S.md`

## Deliverables

- Named boundary-test helpers/tests for source scanning, manifest dependency
  checks, Python-adapter checks, and required dependency presence.
- Regression coverage for the existing diagnostics facade (`DIAGNOSTIC_SCHEMA_VERSION
  == "1"`; `DiagnosticEnvelope`, `Diagnostic`, `DiagnosticCode`, and
  `DiagnosticSeverity`) and path serialization/relative-normalization edge cases.
- Negative coverage proving existing prohibited ATM, reverse-dependency, and
  adapter-dependency patterns remain prohibited.

## Required Work

- Preserve every pre-existing forbidden pattern and required dependency check;
  the refactor may strengthen failures but may not loosen them.
- Do not move path policy from `sc-compose` into `sc-composer`.
- Explicit boundary check: diagnostics stays under Rule 1, path utilities stay
  under Rule 2, and no Python/Go/Beads binding changes or new dependencies are
  allowed.

## Explicit Code Samples

```rust
// Existing public diagnostics facade: freeze these exact exports and value.
// Do not introduce a new public API merely for this test coverage.
pub const DIAGNOSTIC_SCHEMA_VERSION: &str = "1";
pub use envelope::DiagnosticEnvelope;
pub use record::Diagnostic;
pub use schema::{DiagnosticCode, DiagnosticSeverity};

// Existing CLI-owned normalization contract under focused regression coverage.
pub(crate) fn is_normalized_relative_path(path: &Path) -> bool;
pub(crate) fn normalize_relative_path(path: &Path) -> Result<PathBuf, String>;

// Test-only organization; each helper reports actionable failures.
fn assert_source_boundary_rules(root: &Path, violations: &mut Vec<String>);
fn assert_manifest_boundary_rules(root: &Path, violations: &mut Vec<String>);
```

## This Sprint Does Not Close

- A new boundary policy or relaxation of existing policy.
- Beads runner lifecycle changes (S.6).
- Release-package deduplication; the source/plugin mirror remains out of scope.

## Acceptance Criteria

- [ ] Boundary failures identify the violated invariant without losing the
  aggregate failure report.
- [ ] Existing standalone, adapter, and forbidden-dependency cases still fail
  exactly as before.
- [ ] Diagnostics and path edge tests cover empty, absolute, parent, platform
  separator, and normalized relative paths as applicable.
- [ ] Diagnostic facade/schema coverage freezes `DIAGNOSTIC_SCHEMA_VERSION == "1"`,
  the four listed public re-exports, their spelling, and `DiagnosticEnvelope::new`
  defaults without adding public APIs.
- [ ] No production dependency, public API, or serialized-format changes.

## gh-stack Workflow

```bash
git switch integrate/phase-s
git pull --ff-only origin integrate/phase-s
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base integrate/phase-s sprint/s-5-boundary-and-path-guardrails
git add crates/sc-compose/tests/repo_boundaries.rs crates/sc-compose/src/path_utils.rs crates/sc-composer/src/diagnostics.rs crates/sc-composer/src/diagnostics docs/plans/phase-S.md docs/phase-S/sprint-s-5-boundary-and-path-guardrails.md
git commit -m "test(boundaries): isolate invariant guardrails"
gh stack submit --auto
gh stack view --json
gh stack merge <sprint-s-5-pr-number> --yes --merge

# Phase close only, after every Phase S sprint is merged into integrate/phase-s.
git switch develop
gh stack init --base develop integrate/phase-s
gh stack view --json
gh stack merge <phase-s-integration-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-composer -p sc-compose`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
