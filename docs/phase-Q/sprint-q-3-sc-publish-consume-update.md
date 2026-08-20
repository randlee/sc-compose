---
id: Q.3
title: consume sc-publish develop update
status: in_progress
branch: sprint/q-3-sc-publish-consume-update
worktree: ../sc-compose-worktrees/sprint/q-3-sc-publish-consume-update
target: sc-compose develop
depends_on: Q.2 merged (sc-compose develop 5ab6da0); sc-publish PR #38 merged (sc-publish develop ce85b4d)
parallel_with: unrelated work outside release assets and publishing workflows
---

# Sprint Q.3 — Consume `sc-publish` develop Update

## Scope

sc-compose is a **consumer** of the `sc-publish` package, not its owner. This
sprint re-installs the current `sc-publish` develop commit (`ce85b4d`) into
sc-compose, replacing the Q.2-era vendored copy, and verifies sc-compose's own
install/test/CI surface still passes against the update.

This sprint does **not** modify `sc-publish`'s internal workflow, probe, or
install logic. The three known residual defects in that logic — pypi
`build_system` branching, GH Release probe fail-open on transient errors,
winget probe fail-open on transient errors — are `sc-publish`'s own bugs,
already filed there as sc-publish#39, sc-publish#40, sc-publish#41. They are
explicitly out of scope for this sprint; do not attempt to fix them here.
The empty optional channel-output handling defect is likewise tracked upstream
as [sc-publish#43](https://github.com/randlee/sc-publish/issues/43).

## Exact targets

- `plugins/sc-publish/` (full re-vendor from sc-publish develop `ce85b4d`)
- `release/sc-publish-install.json` (update only if the new package version
  requires new/changed manifest fields; do not otherwise touch)
- generated `release/publish-artifacts.toml`
- generated `release/publish-channel-contracts.toml`
- installed `.claude/`, `.cursor/`, `.github/actions/`, `.github/scripts/`,
  `.github/workflows/`, and package-owned `release/` assets

## Deliverables

1. Re-run the installer against the current `sc-publish` develop commit
   (`ce85b4d`) using the existing `release/sc-publish-install.json`, updating
   only what the new package version requires.
2. Confirm the resulting `plugins/sc-publish/` tree matches upstream
   `sc-publish` develop byte-for-byte (diff against the sc-publish repo
   checkout, not just "installer exited zero").
3. Prove generated manifests still validate against Cargo/package metadata
   and that installed workflows reference existing local assets.
4. Run sc-compose's full test suite and Release Preflight rehearsal against
   the updated install; confirm no regression from the Q.2 baseline.
5. Confirm sc-compose's own test run uses the pinned bootstrap venv
   (`sc_compose` bootstrap script), not a stale ambient `sc-compose` install
   — this was flagged as a reproducibility risk in the PR #38 independent
   review.

## Acceptance criteria

- [x] `plugins/sc-publish/` matches sc-publish develop `ce85b4d` exactly
      (no drift, no local patches).
- [x] A second installer dry run is clean with exit code zero.
- [x] `validate-manifest`, publish-order, version-lockstep, package checks,
      and workflow-local-file checks pass.
- [ ] Release Preflight rehearsal run is clean (or records an explicit,
      expected external-service stop) on the updated install.
- [x] sc-compose's own test run is confirmed to use the pinned bootstrap venv.
- [x] No sc-publish internal logic (probes, workflows, install.py) is
      modified by this sprint's diff.

## Required validation

```text
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json .
python3 plugins/sc-publish/install.py --input release/sc-publish-install.json --dry-run .
python3 .github/scripts/release_artifacts.py validate-manifest \
  --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml
diff -rq plugins/sc-publish/ <sc-publish-repo-checkout>/plugins/sc-publish/
git diff --check
python3 -m pytest -q
```

`python3 -m pytest -q` must be run through the pinned bootstrap venv used by
Sprint Q.2 (`sc-compose==1.4.1`), not a stale ambient `sc-compose` install —
record which venv path was used in the sprint handoff.

Then run the installed GitHub Actions Release Preflight, recording the
workflow run URL in the sprint handoff. Local success is not sufficient to
close the sprint.

## Validation evidence

- The package was re-vendored from `sc-publish` develop `ce85b4d`; tracked-file
  parity is exact (the upstream checkout has only an untracked `.github/templates`
  directory beyond the committed package tree).
- Installer dry-run, install, and second dry-run all completed cleanly; the
  second dry-run returned zero.
- Manifest, publish-order, and version-lockstep validation passed. Package
  script tests passed (`63 passed, 7 skipped, 3 subtests`).
- The pinned bootstrap environment was `/private/tmp/sc-compose-q3-venv-1.4.1/bin/python`
  with `sc-compose==1.4.1`. The full suite reached 125 passed tests but has
  four collection/test failures because the upstream `ce85b4d` template does
  not render the caller-declared `go_native` table into
  `release/publish-artifacts.toml`. This is an upstream sc-publish defect and
  is not patched in this consumer sprint.
- Release Preflight run `32340747396`
  ([workflow run](https://github.com/randlee/sc-compose/actions/runs/32340747396))
  completed with the expected external credential failures plus the
  `channel-results` defect: it fails closed on invalid empty `jq --argjson`
  input. The earlier Homebrew `binary_paths`/`binaries` test-fixture mismatch
  has been fixed in this consumer repository.
  No production publication occurred.
- Post-fix `SC_LINT_SOURCE_ROOT=/Users/randlee/Documents/github/sc-lint cargo
  test --workspace` passed. Without that explicit local source root, the two
  `sc_lint_identity_literals` tests reproduce the known bootstrap/environment
  failure on `develop`; this is pre-existing and not a Q3 regression.
- The post-fix pinned-venv command was
  `/private/tmp/sc-compose-q3-venv-1.4.1/bin/python -m pytest -q`. It recorded
  `125 passed, 4 failed`; the four failures are only the `go_native` manifest
  omission tracked by sc-publish#42.
- After reverting the local consumer workflow patch, the installer dry run is
  clean and `.github/workflows/release-preflight.yml` again exactly matches
  the vendored workflow. The underlying empty-channel JSON defect is now
  tracked by sc-publish#43 and must be fixed and re-vendored upstream before
  this sprint can satisfy the fresh-clean-preflight criterion.

## Q3 follow-up blocker resolutions

- **Q3-BLOCK-01 (`go_native`)** — confirmed as an upstream `sc-publish`
  rendering defect: the install manifest declares `go_native`, but the
  upstream `publish-artifacts.toml.j2` does not render that table. This is
  tracked by [sc-publish#42](https://github.com/randlee/sc-publish/issues/42);
  no consumer workaround was added, and the vendored package remains exact.
- **Q3-BLOCK-02 (Homebrew fixture)** — fixed the sc-compose test fixture to
  use the current upstream `binary_paths` contract rather than the obsolete
  `binaries` key. The upstream template was not modified.
- **Q3-BLOCK-03 (empty channel JSON)** — confirmed as an upstream package
  workflow defect and filed as [sc-publish#43](https://github.com/randlee/sc-publish/issues/43).
  The local consumer patch was reverted so installer dry-run remains clean;
  Q3 awaits an upstream fix and re-vendor before its fresh-preflight
  acceptance criterion can close.

The Release Preflight acceptance criterion's "explicit, expected
external-service stop" means: the workflow run halts at a specific,
independently verifiable step — the Test-PyPI/production gate that requires
human-authorized publication — with that step's logs showing an expected
stop reason (e.g. "no publisher authorization present"), not a silent
timeout, an unrelated credential error, or a step that simply never ran. Any
other non-clean outcome is a failure of this criterion, not a pass.

## Handoff and fix routing

Send team-lead the sc-publish source commit, dry-run proof, upstream-diff
proof, manifest validation output, and Release Preflight run URL. Team-lead
opens the PR to develop and routes it to quality-mgr. Any finding that is
actually an sc-publish defect (not an install/consumption problem) gets filed
as an sc-publish issue, not fixed in this sprint's worktree.
