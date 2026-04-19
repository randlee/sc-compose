# Publishing Agent Guide

This document is the operator playbook for the first standalone `1.0.x`
release line of `sc-compose`.

## Scope

The release surface is:

- crates.io:
  - `sc-composer`
  - `sc-compose`
- GitHub Releases:
  - `sc-compose` archives for Linux, macOS, and Windows
- Homebrew:
  - `randlee/homebrew-tap` formula `sc-compose.rb`
- `winget`:
  - package id `randlee.sc-compose`

## Hard Rules

- Release tags are created only by the release workflow.
- Never manually push `v*` tags from a local machine.
- `develop` must already be merged into `main` before release starts.
- Always run the `Release Preflight` workflow before the `Release` workflow.
- If any gate or prerequisite fails, stop and report the exact failure to
  `team-lead`.

## Required Secrets

- `CARGO_REGISTRY_TOKEN`
  - required for publishing both crates to crates.io
- `HOMEBREW_TAP_TOKEN`
  - required so the workflow can update `randlee/homebrew-tap`

`winget` automation uses the default workflow `GITHUB_TOKEN` and does not need
an extra repository secret.

## Standard Release Flow

1. Confirm the target version already exists in the root `Cargo.toml`.
2. Confirm `develop` is merged into `main`.
3. Run `Release Preflight` with:
   - `version=<X.Y.Z or vX.Y.Z>`
   - `run_by_agent=publisher`
4. Wait for preflight to pass.
5. Run the `Release` workflow with the same version input.
6. Monitor the workflow until completion.
7. Verify all channels:
   - crates.io: both crates published in order
   - GitHub Release: archives include `bin/sc-compose` and
     `share/sc-compose/examples`
   - Homebrew: `sc-compose.rb` updated in `randlee/homebrew-tap`
   - `winget`: submission dispatched successfully

## Manual Checks

- Verify crate owners:
  - `cargo owner --list sc-composer`
  - `cargo owner --list sc-compose`
- Verify the target version is unpublished before the workflow runs:
  - `python3 scripts/release_artifacts.py check-version-unpublished --manifest release/publish-artifacts.toml --version <X.Y.Z>`
- Verify package installs:
  - Homebrew and GitHub Release installs include bundled examples
  - `cargo install sc-compose --version <X.Y.Z>` installs the binary only

## Notes

- Bundled examples are guaranteed in Homebrew, `winget`, and GitHub Release
  installs.
- `cargo install` does not ship bundled examples because Cargo installs binary
  artifacts only.
- `SC_COMPOSE_DATA_DIR` can override the examples location for CI, custom
  installs, and `cargo install` users.
