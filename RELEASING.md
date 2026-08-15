# Releasing sc-compose

Step-by-step release process for `sc-sha`, `sc-composer`, `sc-compose`,
`bindings/sc-sha-python`, and `bindings/python` from this repo.

## Overview

This repo publishes to six channels:

| Channel | Package | Method |
|---------|---------|--------|
| crates.io | `sc-sha` | `cargo publish` |
| crates.io | `sc-composer` | `cargo publish` |
| crates.io | `sc-compose` | `cargo publish` |
| PyPI | `sc-sha`, `sc-compose` | `maturin publish` |
| Homebrew | `sc-compose` | GitHub Actions → `randlee/homebrew-tap` |
| Winget | `sc-compose` | GitHub Actions → `microsoft/winget-pkgs` |
| Scoop | `sc-compose` | GitHub Actions → `randlee/scoop-bucket` |
| GitHub Releases | `sc-compose` | CI workflow attachment |

## Versioning

- Single workspace version for all crates in this repo.
- `bindings/sc-sha-python/pyproject.toml` and `bindings/python/pyproject.toml`
  must sync from the workspace version before wheel builds. The release
  workflow enforces this via `verify-python-version`.
- Target version must be strictly higher than the last published version on
  crates.io for these crate names.

## Pre-Release Checklist

### Version Alignment

- [ ] Workspace `Cargo.toml` `[workspace.package] version` reflects the target version
- [ ] `crates/sc-sha/Cargo.toml` matches or inherits workspace version
- [ ] `crates/sc-composer/Cargo.toml` matches or inherits workspace version
- [ ] `crates/sc-compose/Cargo.toml` matches or inherits workspace version
- [ ] `release/publish-artifacts.toml` lists all publishable crates with correct paths
- [ ] `bindings/sc-sha-python/pyproject.toml` version matches workspace version
- [ ] `bindings/python/pyproject.toml` version matches workspace version

### Quality Gates

- [ ] `cargo test --workspace` — zero failures
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` — clean
- [ ] `cargo fmt --all --check` — clean
- [ ] `just lint && just test && just smoke` — all pass
- [ ] `just reports && just reports-verify` — report pipeline intact
- [ ] `observability-health` and `sc-observability` shutdown behavior covered by tests
- [ ] `ERR_*` failure-mode matrix exercised by tests
- [ ] `--json` commands verified to keep stdout machine-readable
- [ ] Standalone boundary verification: no forbidden ATM references in source
- [ ] `quality-mgr` full QA pass
- [ ] `team-lead` final design review

### crates.io Ownership

- [ ] `cargo owner --list sc-sha` — confirm expected owners
- [ ] `cargo owner --list sc-composer` — confirm expected owners
- [ ] `cargo owner --list sc-compose` — confirm expected owners
- [ ] `CARGO_REGISTRY_TOKEN` configured in GitHub Actions `crates-io` environment
- [ ] Token has publish permission for `sc-sha`, `sc-composer`, and `sc-compose`

### Homebrew, Winget, and Scoop

- [ ] `HOMEBREW_TAP_TOKEN` configured in repo secrets
- [ ] `WINGET_GITHUB_TOKEN` configured in repo secrets with permission to open
  Winget submissions in `microsoft/winget-pkgs`; every release dispatches the
  automated `winget-publish.yml` workflow
- [ ] `SCOOP_BUCKET_TOKEN` configured in repo secrets with write access to
  `randlee/scoop-bucket`

### PyPI

- [ ] `PYPI_API_TOKEN` and `TEST_PYPI_API_TOKEN` configured in the protected
  GitHub Actions `pypi` and `testpypi` environments, respectively
- [ ] Run one staged TestPyPI or `workflow_dispatch` rehearsal before treating
  the Python release channel as production-closed:
  - [ ] Both packages build wheels on all three platforms
  - [ ] Exactly one sdist is produced for each package
  - [ ] PyPI upload paths succeed for both packages
  - [ ] GitHub Release attachment set includes both packages' wheels + sdists

### Release Preflight

- [ ] Trigger `.github/workflows/release-preflight.yml` (manual via
  `gh workflow run release-preflight.yml`)
- [ ] Preflight must PASS: version alignment, manifest completeness, workspace
  consistency

## Publish Order (MANDATORY)

Publish in this exact order. Deviating will break the dependency graph.

1. **`sc-sha`** — publish first
   - `cargo publish -p sc-sha`
   - Wait ≥30 seconds for crates.io index propagation
2. **`sc-composer`** — publish after `sc-sha`
   - `cargo publish -p sc-composer`
   - Wait ≥30 seconds for crates.io index propagation
3. **`sc-compose`** — publish after its library dependencies
   - `cargo publish -p sc-compose`

The manually dispatched `.github/workflows/release.yml` workflow creates the
production tag in its `gate-and-tag` job, then enforces this order
automatically.

## Post-Release Channel Dispatch

After the root `Release` workflow has created the immutable GitHub Release,
dispatch the independent, retry-safe channel workflows with the same `v<version>`
tag:

- `.github/workflows/pypi-publish.yml` uploads the published Python artifacts.
- `.github/workflows/homebrew-publish.yml` updates `randlee/homebrew-tap`.
- `.github/workflows/winget-publish.yml` submits the Windows installer.
- `.github/workflows/scoop-publish.yml` updates the Scoop bucket manifest in
  `randlee/scoop-bucket`.

If one channel fails, re-dispatch only that channel workflow. Do not recreate
the tag or rerun the root release workflow.

## Post-Publish Verification

- [ ] Verify `sc-sha` visible on crates.io at expected version
- [ ] Verify `sc-composer` visible on crates.io at expected version
- [ ] Verify `sc-compose` visible on crates.io at expected version
- [ ] `cargo add sc-sha@<version>` in a scratch workspace — resolves cleanly
- [ ] `cargo add sc-composer@<version>` in a scratch workspace — resolves cleanly
- [ ] `cargo install sc-compose@<version>` — binary installs cleanly
- [ ] Verify GitHub Release archives include `share/sc-compose/examples/`
- [ ] Verify Homebrew formula update completed in `randlee/homebrew-tap`
- [ ] Verify `winget` submission/update dispatched successfully
- [ ] Verify Scoop manifest update completed in `randlee/scoop-bucket` and
      `scoop install sc-compose` succeeds
- [ ] Verify PyPI: `pip install sc-sha==<version> sc-compose==<version>` on all
      three platforms
- [ ] Confirm the production `gate-and-tag` job created `v<version>` at
  `origin/main`
- [ ] Confirm the release workflow created the GitHub Release for that tag
  with its generated release notes

## Distribution Channel Details

### crates.io

- Manifest: `release/publish-artifacts.toml`
- Preflight: `.github/workflows/release-preflight.yml`
- Release: `.github/workflows/release.yml`
- Gate script: `scripts/release_gate.sh`

### PyPI

- Pre-release rehearsals go to TestPyPI:
  ```bash
  pip install -i https://test.pypi.org/simple/ sc-sha sc-compose
  ```
- Production releases for `sc-sha` and `sc-compose` go to PyPI via the `pypi`
  protected environment
- `verify-python-version` gate enforces version sync for both Python packages
  before wheel builds

### Homebrew

- Formula auto-updated in `randlee/homebrew-tap` via release workflow
- Bundled examples installed to `$(brew --prefix)/share/sc-compose/examples/`

### Scoop

- Add the bucket, then install the package:
  ```powershell
  scoop bucket add randlee https://github.com/randlee/scoop-bucket
  scoop install sc-compose
  ```
- The retry-safe `.github/workflows/scoop-publish.yml` workflow updates the
  manifest from the immutable GitHub Release asset.

### GitHub Releases

- Archives include `bin/sc-compose` and `share/sc-compose/examples/...`
- Platform-specific archives: Linux, macOS, Windows

### Cargo Install Limitation

`cargo install` ships the binary only — it does NOT install bundled examples.
Users who install with Cargo must set `SC_COMPOSE_DATA_DIR` to access examples.

## Release Notes

Fill out `release/RELEASE-NOTES-TEMPLATE.md` with the actual release summary
before creating the GitHub release. The template covers:

- Summary of changes
- Included crates and versions
- Compatibility notes and migration guidance

## ATM Cutover

When the ATM workspace needs to switch from in-workspace path dependencies to
crates.io dependencies from this repo:

1. This repo must publish the target version of `sc-sha`
2. This repo must publish the target version of `sc-composer`
3. This repo must publish the target version of `sc-compose`
4. ATM must replace its in-workspace path dependencies with version pins

## Reference Documents

- [docs/publishing.md](docs/publishing.md) — publishing and version ownership
- [docs/release-checklist.md](docs/release-checklist.md) — previous checklist format
- [docs/release-tag-protection.md](docs/release-tag-protection.md) — tag protection rules
- [docs/publishing-agent.md](docs/publishing-agent.md) — operator guide
