---
id: FIX-RELEASE-1.4.2-PREFLIGHT
title: Correct 1.4.2 publish credentials and release records
status: in_progress
branch: release/1.4.2-prep
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/release/1.4.2-prep
target: main
---

## Goal

Make the 1.4.2 release path use the actual PyPI credential contract without
adding protected-environment approval prompts, and record the shipped channel
and bug-fix content before the tag is created.

## Deliverables

1. `.github/workflows/pypi-publish.yml` retains manifest-driven asset and
   repository selection while using independently conditional TestPyPI and
   PyPI upload steps, each with its own correctly scoped API token.
   `.github/workflows/release.yml` uses `TEST_PYPI_API_TOKEN` for its
   TestPyPI upload.
2. `.github/workflows/release-preflight.yml` retains repository-secret value
   checks for repository secrets, and checks only the names of the protected
   `pypi` and `testpypi` environment secrets through `gh api`. It must not bind
   the preflight job to either environment or expose a secret value.
3. Active release operator documents use the same token names and correctly
   describe `CARGO_REGISTRY_TOKEN` as a repository secret.
4. `CHANGELOG.md` has a dated 1.4.2 entry for Scoop and the hermetic
   fixture-discovery fix; `release/RELEASE-NOTES-TEMPLATE.md` lists both
   `sc-sha` distributions and Scoop.
5. `scripts/tests/test_release_artifacts.py` asserts the active workflow
   secret contract and the metadata-only environment-secret check.

## Exact Targets

- `.github/workflows/pypi-publish.yml`
- `.github/workflows/release.yml`
- `.github/workflows/release-preflight.yml`
- `scripts/tests/test_release_artifacts.py`
- `RELEASING.md`, `docs/publishing.md`, `docs/publishing-agent.md`
- `CHANGELOG.md`, `release/RELEASE-NOTES-TEMPLATE.md`

## Required Validation

- `python3 -m pytest scripts/tests/test_release_artifacts.py`
- `cargo fmt --all --check`
- `cargo test --workspace` with the CI-pinned sc-lint runtime materialized
- `git diff --check`

## Out of Scope

- Version alignment from PR #498.
- Dispatching, tagging, or publishing 1.4.2.
- Wiring `RELEASE-NOTES-TEMPLATE.md` into automation.
- Release-rule content-parity policy and the #499 develop merge; publisher
  owns that mechanical merge separately.

## Acceptance Criteria

1. No active PyPI upload workflow references `PYPI_TOKEN` or
   `TEST_PYPI_TOKEN`; TestPyPI and production uploads remain separately
   conditional and execute in their corresponding protected environment.
2. Release preflight can detect absent PyPI environment-secret names without
   requesting a protected-environment approval or printing a token value.
3. All active operator documentation agrees on the credential names and
   locations.
4. The changelog and release-notes package table accurately describe 1.4.2's
   released artifacts.
5. The required validation passes, including the CI-provisioned workspace
   suite.

## References

- #497: PyPI credential wiring.
- #500: `RELEASING.md` credential documentation.
- #501: 1.4.2 changelog entry.
- #502: release-notes package-table drift.
