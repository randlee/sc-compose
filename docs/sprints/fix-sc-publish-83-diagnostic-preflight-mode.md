---
status: assigned
branch: fix/83-diagnostic-preflight-mode
worktree: /Users/randlee/Documents/github/sc-publish-worktrees/fix-83-diagnostic-preflight-mode
---

# FIX-SC-PUBLISH-83: diagnostic-vs-authorization preflight mode

## Source

comp's assessment of the v1.6.1 readiness-preflight FAILED report
(`/Users/randlee/.atm/.config/atm/share/sc-compose/v1.6.1-readiness-preflight-report.json`),
filed upstream as `sc-publish#83` (https://github.com/randlee/sc-publish/issues/83).

Note: this task lives in the **sc-publish** repo, not sc-compose. sc-publish
has no `docs/sprints/` convention of its own — this doc is tracked here in
sc-compose (the orchestrating repo) per comp's own issue filing, and stands in
as the authoritative sprint doc for this assignment.

## Problem

The current `release-preflight.yml` / gate-script package-checks step
hard-fails when a workspace-internal crate dependency (e.g.
`sc-composer-beads = "^1.6.1"`) cannot yet resolve against crates.io, because
the dependency's next version hasn't published yet. This is expected and
routine for any multi-crate release where one crate depends on a sibling
crate's about-to-be-published version — it is the same "not yet published"
situation already handled as non-blocking for first-time publishes, but the
gate currently does not extend that sibling-skip treatment to an
already-published crate awaiting its *next* version.

Per comp's issue #83, the fix scope is:
- a diagnostic-vs-authorization preflight mode distinction (so a readiness
  check run against a not-yet-authorized release state can still report full
  sanitized diagnostics without hard-failing on expected sibling-version gaps)
- publisher dispatch despite invalid release state (report, don't block)
- per-probe aggregation fixes, including registry/liveness loop handling
- a complete sanitized receipt covering every manifest channel
- regression/eval coverage for this case
- link the remaining agent-routing gap to sc-publish#69

## Required Fix

- Read `sc-publish#83` in full (https://github.com/randlee/sc-publish/issues/83)
  and treat it as the authoritative scope for this task; if this summary and
  the issue differ, the issue wins and the mismatch must be reported.
- Extend the package-checks gate's existing first-publish sibling-skip logic
  to also cover an already-published sibling crate awaiting its next version,
  without weakening any other gate check.
- Implement the diagnostic-vs-authorization preflight mode distinction
  described in the issue.
- Add regression/eval coverage exercising this exact case (an internal
  workspace crate dependency requirement one version ahead of what's live on
  crates.io).

## Acceptance Criteria

- A readiness preflight run against a release where an internal sibling crate
  requires its own next (unpublished) version no longer hard-fails solely on
  that unresolved version — it reports the condition as a non-blocking,
  expected finding.
- All other gate checks (real missing-tag provenance failures, genuine
  unresolvable external dependencies, credential/liveness checks, etc.) are
  unaffected and still hard-fail correctly.
- New regression/eval coverage passes.
- Repo's standard validation suite passes (fmt/lint/tests per sc-publish's own
  CI gates).

## References

- https://github.com/randlee/sc-publish/issues/83
- https://github.com/randlee/sc-publish/issues/69
- `/Users/randlee/.atm/.config/atm/share/sc-compose/v1.6.1-readiness-preflight-report.json`
