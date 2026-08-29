---
id: S.7
title: Path Normalization Contract
status: planned
branch: sprint/s-7-path-normalization-contract
worktree: ../sc-compose-worktrees/sprint/s-7-path-normalization-contract
target: integrate/phase-s
---

# Sprint S.7 — Path Normalization Contract

## Goal

This is a coverage-only sprint: it freezes existing CLI-owned relative-path
normalization and serialization-adjacent manifest path behavior through
regression tests and makes no production code change to its target files. It
closes S-T10 only in the sense that the existing contract is frozen, not
refactored.

## Hard Dependencies

- `integrate/phase-s` exists from `develop` before this sprint branch exists.
- No hard code dependency on S.1–S.6; merge-forward the latest integration
  branch before implementation and submission.

## Exact Targets

- `crates/sc-compose/src/path_utils.rs`
- `crates/sc-compose/src/reporting/publish_manifest/tests.rs`
- existing path-utils unit-test locations only
- `docs/plans/phase-S.md`

## Deliverables

- Regression cases for empty, absolute, parent, platform-separator, and
  normalized relative paths in the existing helper and manifest-path surfaces.

## Required Work

- Preserve CLI ownership of path policy; do not move it into `sc-composer`.
- Do not change serialized path output, error strings, or public report schema.
- Follow `CLAUDE.md` Rule 2: no dependency-direction change or new adapter.
- **Production-ready closure:** every listed normalization and serialization
  regression case must land in this sprint; partial edge-path coverage does not
  close S-T10.

## Explicit Code Samples

```rust
pub(crate) fn is_normalized_relative_path(path: &Path) -> bool;
pub(crate) fn normalize_relative_path(path: &Path) -> Result<PathBuf, String>;
```

## This Sprint Does Not Close

- Repository-boundary test organization (S.5).
- Diagnostics-facade coverage (S.6), runner work (S.8), or a path-policy
  change.

## Acceptance Criteria

- [ ] Tests cover empty, absolute, parent, platform-separator, and normalized
  relative paths as applicable to the existing helper contract.
- [ ] Manifest-path coverage preserves existing serialized forward-slash output.
- [ ] No path policy, error string, public schema, or dependency changes occur.

## gh-stack Workflow

```bash
git switch integrate/phase-s
git pull --ff-only origin integrate/phase-s
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base integrate/phase-s sprint/s-7-path-normalization-contract
git add crates/sc-compose/src/path_utils.rs crates/sc-compose/src/reporting/publish_manifest/tests.rs crates/sc-compose/tests docs/plans/phase-S.md docs/phase-S/sprint-s-7-path-normalization-contract.md
git commit -m "test(paths): freeze normalization contract"
gh stack submit --auto
gh stack view --json
gh stack merge <sprint-s-7-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-compose --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
