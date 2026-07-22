# Releasing sc-compose

Step-by-step release process for `sc-composer`, `sc-compose`, and
`bindings/python` from this repo.

## Overview

This repo publishes to five channels:

| Channel | Package | Method |
|---------|---------|--------|
| crates.io | `sc-composer` | `cargo publish` |
| crates.io | `sc-compose` | `cargo publish` |
| PyPI | `sc-compose` | `maturin publish` |
| Homebrew | `sc-compose` | GitHub Actions → `randlee/homebrew-tap` |
| Winget | `sc-compose` | GitHub Actions → `microsoft/winget-pkgs` |
| GitHub Releases | `sc-compose` | CI workflow attachment |

## Versioning

- Single workspace version for all crates in this repo.
- `bindings/python/pyproject.toml` must sync from the workspace version before
  wheel builds. The release workflow enforces this via `verify-python-version`.
- Target version must be strictly higher than the last published version on
  crates.io for these crate names.

## Pre-Release Checklist

### Version Alignment

- [ ] Workspace `Cargo.toml` `[workspace.package] version` reflects the target version
- [ ] `crates/sc-composer/Cargo.toml` matches or inherits workspace version
- [ ] `crates/sc-compose/Cargo.toml` matches or inherits workspace version
- [ ] `release/publish-artifacts.toml` lists both crates with correct paths
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

- [ ] `cargo owner --list sc-composer` — confirm expected owners
- [ ] `cargo owner --list sc-compose` — confirm expected owners
- [ ] `CARGO_REGISTRY_TOKEN` configured in GitHub Actions `crates-io` environment
- [ ] Token has publish permission for both `sc-composer` and `sc-compose`

### Homebrew and Winget

- [ ] `HOMEBREW_TAP_TOKEN` configured in repo secrets
- [ ] First `winget` release requires one-time manual submission to
  `microsoft/winget-pkgs`; later releases use the automated workflow job

### PyPI

- [ ] `PYPI_API_TOKEN` configured in GitHub Actions `pypi` environment
- [ ] Run one staged TestPyPI or `workflow_dispatch` rehearsal before treating
  the Python release channel as production-closed:
  - [ ] Wheel build succeeds on all three platforms
  - [ ] Exactly one sdist produced
  - [ ] PyPI upload path succeeds
  - [ ] GitHub Release attachment set includes wheels + sdist

### Release Preflight

- [ ] Trigger `.github/workflows/release-preflight.yml` (manual via
  `gh workflow run release-preflight.yml`)
- [ ] Preflight must PASS: version alignment, manifest completeness, workspace
  consistency

## Publish Order (MANDATORY)

Publish in this exact order. Deviating will break the dependency graph.

1. **`sc-composer`** — publish first
   - `cargo publish -p sc-composer`
   - Wait ≥30 seconds for crates.io index propagation
2. **`sc-compose`** — publish second
   - `cargo publish -p sc-compose`

The `.github/workflows/release.yml` workflow enforces this order automatically
when triggered by a release tag.

## Post-Publish Verification

- [ ] Verify `sc-composer` visible on crates.io at expected version
- [ ] Verify `sc-compose` visible on crates.io at expected version
- [ ] `cargo add sc-composer@<version>` in a scratch workspace — resolves cleanly
- [ ] `cargo install sc-compose@<version>` — binary installs cleanly
- [ ] Verify GitHub Release archives include `share/sc-compose/examples/`
- [ ] Verify Homebrew formula update completed in `randlee/homebrew-tap`
- [ ] Verify `winget` submission/update dispatched successfully
- [ ] Verify PyPI: `pip install sc-compose==<version>` on all three platforms
- [ ] Tag the release commit: `git tag v<version> && git push origin v<version>`
- [ ] Create a GitHub release pointing at the tag with filled-in release notes
  from `release/RELEASE-NOTES-TEMPLATE.md`

## Distribution Channel Details

### crates.io

- Manifest: `release/publish-artifacts.toml`
- Preflight: `.github/workflows/release-preflight.yml`
- Release: `.github/workflows/release.yml`
- Gate script: `scripts/release_gate.sh`

### PyPI

- Pre-release rehearsals go to TestPyPI:
  ```bash
  pip install -i https://test.pypi.org/simple/ sc-compose
  ```
- Production releases go to PyPI via the `pypi` protected environment
- `verify-python-version` gate enforces version sync before wheel builds

### Homebrew

- Formula auto-updated in `randlee/homebrew-tap` via release workflow
- Bundled examples installed to `$(brew --prefix)/share/sc-compose/examples/`

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

1. This repo must publish the target version of `sc-composer`
2. This repo must publish the target version of `sc-compose`
3. ATM must replace its in-workspace path dependencies with version pins

## Reference Documents

- [docs/publishing.md](docs/publishing.md) — publishing and version ownership
- [docs/release-checklist.md](docs/release-checklist.md) — previous checklist format
- [docs/release-tag-protection.md](docs/release-tag-protection.md) — tag protection rules
- [docs/publishing-agent.md](docs/publishing-agent.md) — operator guide
