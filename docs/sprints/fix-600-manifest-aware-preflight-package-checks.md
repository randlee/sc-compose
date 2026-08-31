---
status: dev-complete
branch: fix/manifest-aware-preflight-package-checks
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/manifest-aware-preflight-package-checks
---

# FIX-600: vendor manifest-aware preflight package-check fix into sc-compose

## Source

sc-compose-side vendoring of the sc-publish#83 fix already QA-passed in
sc-publish PR #84 (see `docs/sprints/fix-sc-publish-83-diagnostic-preflight-mode.md`,
which remains authoritative for the underlying bug and fix rationale). This
task is the bug fix itself, not a new feature — scope is a straight port of
six specific assets, not a full sc-publish reinstall.

## Required Fix (deliverables)

- Manifest-aware package-check-plan replaces the output-grep sibling
  workaround: crates directly depending on an earlier `publish=true` manifest
  crate use `cargo package --no-verify` instead of full verification.
- Real sc-compose manifest plan honored: `sc-sha` verify; `sc-composer` waits
  on `sc-sha`; `sc-composer-beads` waits on `sc-composer` (recognized at
  publish order 3); `sc-compose` waits on `sc-composer` + `sc-composer-beads`.
- Candidate-tag/provenance policy is explicitly unchanged: a missing
  `release-candidate-vX.Y.Z` tag remains a legitimate blocking result.

## Acceptance Criteria

- Only the six b685b6c assets from PR #84 are vendored — no unrelated
  sc-publish reinstall or scope expansion.
- Candidate-tag/provenance blocking behavior is unaffected.
- `pytest .github/scripts/tests/test_release_artifacts.py` passes (comp
  reported 68 passed / 6 skipped).
- Package `unittest` suite passes (comp reported 16 passed).
- `py_compile` and `git diff --check` pass.

## References

- `docs/sprints/fix-sc-publish-83-diagnostic-preflight-mode.md` (authoritative
  source fix, sc-publish#83, sc-publish PR #84 — QA PASS 4/4)
- https://github.com/randlee/sc-compose/pull/600
- commit 1d4d16a28259498dbab5871ac54792be4016eba5
