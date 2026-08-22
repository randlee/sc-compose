---
status: assigned
branch: fix/release-yml-build-ref-literal-sha
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/release-yml-build-ref-literal-sha
pr: 549
---

# FIX-549-ARCH-001-002: release_sha output redundancy + registry-liveness duplication

## Source

quality-mgr revised verdict for SCQA-549 (FAIL, issued per direct user
instruction), commit `581a8ee32320d6a55cec07f523ba356784fb1f04`,
PR comment https://github.com/randlee/sc-compose/pull/549#issuecomment-5373197273.
This FAIL blocks PR #549's merge on exactly these two findings; the PR's own
7/7 deliverables (build_ref literal-string regression fix) need no rework.

## Portability constraint (binding on both findings)

`.github/scripts/`, `.github/workflows/release*.yml`, and
`plugins/sc-publish/` are vendored as-is into roughly 20 other repos. Any
consolidation here must stay generic:
- no hardcoded repo/org name, crate list, or workspace-layout assumption in
  script or workflow logic — repo-specific values come from the manifest
  (`release/publish-artifacts.toml`, `publish-channel-contracts.toml`) or a
  workflow input, never a literal in the shared script
- the shared surface must degrade to "not applicable" rather than fail when a
  manifest omits a channel/crate
- root and `plugins/sc-publish/` copies must remain byte-identical mirrors

## Finding ARCH-001 (Blocking)

File/Line: `.github/scripts/release_gate.sh:40` (+ `plugins/sc-publish` mirror,
same line); consumed redundantly at `.github/workflows/release.yml:111`
(+ mirror).

`release_gate.sh` computes `release_sha="$(git rev-parse "$RELEASE_REF")"`
internally in final mode but never writes it to `$GITHUB_OUTPUT` (zero
`GITHUB_OUTPUT` references anywhere in the script). The workflow's very next
step redundantly re-derives the identical value via its own separate
`main_sha="$(git rev-parse origin/main)"` call.

**Required fix**: add a `release_sha` output to `release_gate.sh`; change
`release.yml`'s `gate-and-tag` job to consume it instead of independently
re-running `git rev-parse origin/main`. Apply identically to the
`plugins/sc-publish` mirror.

## Finding ARCH-002 (Blocking)

File/Line: `.github/workflows/release.yml:265-298` (`crate_exists`/
`publish_if_missing`) is byte-for-byte identical to
`.github/workflows/crates-publish.yml:77-110`;
`.github/workflows/release-preflight.yml:332-341` (`status_code()`)
independently reimplements the same curl+200/404 pattern a third time. All 3
duplicated again in `plugins/sc-publish` mirrors (6 sites total).

**Required fix**: add a `release_artifacts.py` subcommand (e.g.
`registry-status`) performing the curl-and-interpret step; have
`release.yml`, `crates-publish.yml`, and `release-preflight.yml` (+ mirrors)
call it instead of maintaining 3 independent bash implementations.

## Non-blocking note (not required to close this FAIL)

simplification-reviewer: the reuse/create/rehearsal branch structure in
`gate-and-tag` remains a latent duplication-risk source beyond what PR #549
already collapsed. Comp may evaluate simplifying it in the same pass, but it
is not required for this FAIL to close.

## Acceptance Criteria

- `release_gate.sh` (root + plugin mirror) emits `release_sha` via
  `$GITHUB_OUTPUT` in final mode.
- `release.yml`'s `gate-and-tag` job (root + plugin mirror) consumes that
  output instead of re-deriving `git rev-parse origin/main`.
- One `release_artifacts.py registry-status` (or equivalent) subcommand
  implements the curl+200/404 registry-liveness check, parameterized by
  registry/package — no repo-specific literals.
- `release.yml`, `crates-publish.yml`, `release-preflight.yml` (root + plugin
  mirrors) all call the shared subcommand; the 3 independent bash
  implementations are removed.
- Root and `plugins/sc-publish` mirrors remain byte-identical.
- Regression tests cover both the `release_sha` output/consumption path and
  the consolidated registry-status subcommand (existing + new registry
  states: published, not-found, transient-error).
- Full validation sweep (cargo fmt/clippy/test, workflow YAML lint) passes.
