# Publishing Agent Guide

This is the vendorable operator playbook for this repository's release kit.
The repository-specific release surface is declared only in
`release/publish-artifacts.toml`.

## Scope

The manifest declares all release inventory:

- crates.io crates and dependency-aware publish order;
- GitHub Release targets, archives, binaries, and bundled paths;
- Python wheels and source distributions;
- PyPI/TestPyPI repository names; and
- Homebrew, winget, and Scoop destination configuration.

To vendor the kit into another repository, copy the workflow files, composite
actions, helper scripts/templates, this guide, and the manifest; change only
the manifest's repository-specific values. In particular, retain
`.github/actions/extract-published-sc-compose/` with the Homebrew and Scoop
workflows. Do not fork workflow logic to add package names, target triples,
tap/bucket locations, or distribution inventories.

## Hard Rules

- Release tags are created only by the release workflow.
- Never manually push `v*` tags from a local machine.
- `develop` must already be merged into `main` before release starts.
- Always run the `Release Preflight` workflow before the `Release` workflow.
- If any gate or prerequisite fails, stop and report the exact failure to
  `team-lead`.

## Required Secrets

Release secrets are standardized GitHub Actions repository or environment
secrets across every repository that vendors this kit. They are already
provisioned by the release owner. The `publisher` agent must use the documented
secret references, but must not ask whether a token exists, request a token, or
attempt to inspect a token value.

- `CARGO_REGISTRY_TOKEN` — crates.io publication
- `PYPI_TOKEN` and `TEST_PYPI_TOKEN` — production and rehearsal Python uploads
- `HOMEBREW_TAP_TOKEN` — Homebrew tap update
- `WINGET_GITHUB_TOKEN` — winget submission
- `SCOOP_BUCKET_TOKEN` — Scoop bucket manifest update

Release Preflight fails before a release if a required secret is absent. It
also makes non-mutating GitHub API checks for the Homebrew, Scoop, and winget
tokens, which detects expired or revoked GitHub tokens without exposing them.
If Actions reports a missing or rejected secret, report the exact workflow
failure to `team-lead`; do not replace it with a locally supplied credential
or change the secret contract.

## Standard Release Flow

1. Confirm the target version already exists in the workspace metadata.
2. Confirm `develop` is merged into `main`.
3. Run `Release Preflight` with:
   - `version=<X.Y.Z or vX.Y.Z>`
   - `run_by_agent=publisher`
4. Wait for preflight to pass.
5. Run the `Release` workflow with the same version input.
6. Monitor the workflow until completion.
7. Verify every channel and artifact declared in the manifest.
8. If a downstream channel fails after the GitHub Release is published, run
   only that channel's dispatch workflow again with the same tag. Do not rerun
   the root `Release` workflow, recreate the tag, republish crates, or rebuild
   artifacts.

## Manual Checks

- Verify crate owners for every crate in the manifest.
- Verify the target version is unpublished before the workflow runs:
  - `python3 scripts/release_artifacts.py check-version-unpublished --manifest release/publish-artifacts.toml --version <X.Y.Z>`
- Verify each configured install path includes its manifest-declared binaries
  and bundled paths.
- Run a TestPyPI rehearsal when the manifest includes Python distributions;
  confirm the declared wheel/sdist set uploads successfully.

## Notes

- Treat the GitHub Release as the immutable source of truth for external
  channel workflows. They download and verify its declared assets rather than
  rebuilding them.
- The workflows are retry-safe: an already-updated Homebrew/Scoop manifest or
  an already-uploaded PyPI distribution is a successful no-op.
