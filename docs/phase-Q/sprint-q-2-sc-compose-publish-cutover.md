---
id: Q.2
title: sc-compose install and publish cutover
status: in_progress
branch: sprint/q-2-sc-compose-publish-cutover
worktree: ../sc-compose-worktrees/sprint/q-2-sc-compose-publish-cutover
target: sc-compose develop
depends_on: Q.1 merged sc-publish commit
parallel_with: unrelated work outside release assets and publishing workflows
---

# Sprint Q.2 — sc-compose Install and Publish Cutover

## Scope

Install the reviewed sc-publish package into sc-compose and remove the
independently maintained publishing implementation. The consumer retains a
complete JSON input manifest and unrelated CI/Pages workflows only.

## Exact targets

- `release/sc-publish-install.json`
- generated `release/publish-artifacts.toml`
- generated `release/publish-channel-contracts.toml`
- installed `.claude/`, `.cursor/`, `.github/actions/`, `.github/scripts/`,
  `.github/workflows/`, and package-owned `release/` assets
- consumer-owned `.github/workflows/ci.yml` and `.github/workflows/pages.yml`

## Deliverables

1. Create the complete sc-compose JSON input from the reviewed release surface:
   all crates/order, release targets, binaries, Python distributions, Go
   native metadata, and all four channel declarations.
2. Install the exact Q.1 sc-publish commit into a clean sc-compose worktree.
3. Overwrite package-owned publishing agents, skills, scripts, actions,
   workflows, and templates; preserve unrelated CI, Pages, release notes, and
   runtime files.
4. Prove generated manifests validate against Cargo/package metadata and that
   the installed workflows reference existing local assets.
5. Run Release Preflight on a rehearsal version, then run the test-PyPI
   rehearsal. Do not publish production artifacts in this sprint.

## Acceptance criteria

- [x] `release/sc-publish-install.json` is complete and reviewed; no target or
      channel is inferred by the installer.
- [x] The installer overwrites exactly the intended shared publishing assets
      and generates the manifests.
- [x] A second dry run is clean with exit code zero.
- [x] `validate-manifest`, publish-order, version-lockstep, package checks, and
      workflow-local-file checks pass.
- [x] Installed preflight emits complete per-channel results and correctly
      distinguishes passed, failed, and blocked states.
- [x] A failed channel can be retried without rebuilding or replaying passed
      channels; a blocked channel is not retried until missing evidence exists.
- [x] Test-PyPI rehearsal completes or records an explicit external-service
      failure without production publication.
- [x] No production tag or publish occurs until exact-main final preflight and
      explicit named-publisher authorization are present.

## Required validation

```text
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
python3 .github/scripts/release_artifacts.py validate-manifest \
  --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
git diff --check
```

Then run the installed GitHub Actions Release Preflight and test-PyPI
rehearsal, recording workflow URLs and per-channel receipts in the sprint
handoff. Local success is not sufficient to close the sprint.

## Validation evidence

- Installed package source: sc-publish PR #28 merge `2699782`, with the
  sibling UniFFI layout from PR #29 head `d2655d8`.
- The first dry-run reported expected drift, installation returned zero, and
  the second dry-run returned zero with no diff.
- Manifest, publish-order, version-lockstep, Python package version, workflow
  asset, and vendored package checks passed locally (`46 passed, 7 skipped,
  3 subtests`).
- `py_compile` and `git diff --check` passed.
- Production publication was not attempted. Credential-bearing GitHub
  Actions preflight and Test-PyPI rehearsal require the pushed branch and
  repository secrets; the latest rehearsal runs were dispatched after the
  follow-up fixes: preflight run `32328716144` and Test-PyPI run
  `32328718083`. Test-PyPI stopped at the expected main/develop release gate.
- Follow-up validation fixed the Homebrew template to consume the canonical
  manifest `binaries` list and removed the unreachable legacy install branches;
  it also made empty per-channel JSON outcomes fail closed to `{}` before
  `jq --argjson` processing.
- The canonical package source is retained at `plugins/sc-publish`; its
  sibling UniFFI package is supplied by sc-publish PR #29 (`d2655d8`).
- A non-publishing mixed-state rehearsal was run from this branch with
  `channel_state_rehearsal=mixed-channel-states`: run
  `32331918843` ([workflow run](https://github.com/randlee/sc-compose/actions/runs/32331918843)).
  Its logged structured result contains `crates_io=failed`,
  `github_release/homebrew/winget/scoop=passed`, and `pypi=blocked`; the
  expected overall workflow failure prevented publication.

## Handoff and fix routing

Send team-lead the parent commit, exact package commit, manifest path, dry-run
proof, workflow run URLs, and channel receipts. Team-lead opens the PR to
develop and routes it to quality-mgr. Any remaining sc-lint or migration
finding gets an independent fix worktree from this sprint commit, grouped by
issue class, then follows the normal QA-approved merge and revalidation path.
