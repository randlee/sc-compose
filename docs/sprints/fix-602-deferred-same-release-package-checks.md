---
status: dev-complete
branch: fix/deferred-same-release-package-checks
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/deferred-same-release-package-checks
---

# FIX-602: defer same-release package checks instead of skipping verification

## Source

Follow-up to `docs/sprints/fix-600-manifest-aware-preflight-package-checks.md`
(PR #600, QA PASS, merged). Readiness preflight re-run against PR #598
(chore/v1.6.1-version-bump, head 808bdcd) after #600 landed found gate 2
(package-checks) still failing: #600's `cargo package --no-verify` skip
covered `sc-composer` and `sc-composer-beads` (crates that only *depend on* an
unpublished same-release sibling), but `cargo package` itself resolves a
declared registry dependency version against the live crates.io index before
any no-verify skip logic can apply — so `sc-compose`'s own package step still
hard-failed resolving `sc-composer-beads = "^1.6.1"` (only 1.6.0 published).
`--no-verify` does not prevent that resolution step; it only skips the build
verification that follows a successful `cargo package`.

## Required Fix (deliverables)

- Replace the `no_verify` sibling-skip mode with `deferred_same_release`:
  when a crate has an earlier manifest-declared same-release dependency that
  is not yet published, preflight must not invoke `cargo package` for that
  crate at all (since the command itself fails at dependency resolution,
  before `--no-verify` has any effect).
- Workspace/manifest/publish-order checks are still run for every crate
  regardless of this deferral.
- Candidate-tag/provenance policy is unchanged: a missing
  `release-candidate-vX.Y.Z` tag remains a legitimate blocking result.

## Acceptance Criteria

- `sc-compose`'s own package-check step no longer hard-fails when
  `sc-composer-beads` (or any earlier same-release dependency) has not yet
  published the required version.
- `sc-composer` / `sc-composer-beads` behavior from PR #600 is preserved or
  subsumed by the new `deferred_same_release` mode (no regression).
- `pytest .github/scripts/tests/test_release_artifacts.py` passes (comp
  reported 68 passed / 6 skipped).
- Publish-kit unit suite passes (comp reported 16 passed).
- `py_compile` and `git diff --check` pass.
- Re-running readiness preflight against this branch (or the v1.6.1 stack
  once rebased) shows gate 2 passing; gate 1 (release-candidate tag) remains
  the only expected-red gate pending an actual tag cut.

## References

- `docs/sprints/fix-600-manifest-aware-preflight-package-checks.md` (prior
  round, PR #600, QA PASS — incomplete fix this supersedes)
- https://github.com/randlee/sc-compose/pull/602
- readiness preflight re-check
  (`/Users/randlee/.atm/.config/atm/share/sc-compose/v1.6.1-readiness-preflight-report-2.json`)
  documenting the gate-2 gap this closes
- commit 847b907 on `fix/deferred-same-release-package-checks`
