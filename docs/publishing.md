# Publishing and Version Ownership

## Purpose

This repo is the publishing source of truth for the `sc-sha`, `sc-composer`,
and `sc-compose` release family:
- `sc-sha`
- `sc-composer`
- `sc-compose`

That family also includes the `sc-sha` and `sc-compose` Python distributions
and the `sc-sha-go` module.

## Versioning

- The repo uses a single workspace version.
- All published crates in this repo must share that version.
- Release workflows verify that the requested release version matches:
  - workspace version
  - each crate package version
- Python release builds synchronize `bindings/sc-sha-python/pyproject.toml`
  and `bindings/python/pyproject.toml` from the workspace version before wheel
  or sdist builds, then fail on version drift.

## Release Source Of Truth

- Consumer-owned release input: `release/sc-publish-install.json`
- Generated release plans: `release/publish-artifacts.toml` and
  `release/publish-channel-contracts.toml` (do not hand-edit)
- Canonical shared publishing package: `plugins/sc-publish`
- Preflight workflow: `.github/workflows/release-preflight.yml`
- Release workflow: `.github/workflows/release.yml`
- Release gate helper: `.github/scripts/release_artifacts.py` (installed from
  `plugins/sc-publish`)
- Release notes template: `release/RELEASE-NOTES-TEMPLATE.md`
- `winget` setup note: `docs/WINGET_SETUP.md`
- Operator guide: `docs/publishing-agent.md`

## Package Contents

GitHub Release archives and package-manager installs ship
`bin/sc-compose` plus `share/sc-compose/examples/...`. At runtime, bundled
examples resolve from `SC_COMPOSE_DATA_DIR/examples/` when set, otherwise from
the install-relative shared-data directory. User template packs instead use
`SC_COMPOSE_TEMPLATE_DIR` or the platform user-data directory; package wrappers
must not set that variable or place user templates in the shared examples root.

`cargo install` installs only the binary. Users who need bundled examples with
that installation method must point `SC_COMPOSE_DATA_DIR` at a copied examples
root.

## Release Secrets And Ownership Checks

### Configured credential fact

The following credentials already exist and are configured in this
repository's GitHub Actions secret locations. They are not local inputs and
agents and reviewers must not ask whether they exist, request their values, or
try to prove their presence from a local checkout.

- `CARGO_REGISTRY_TOKEN` — repository secret used for crates.io publication;
  the publish job runs in the `crates-io` environment.
- `HOMEBREW_TAP_TOKEN` — repository secret for `randlee/homebrew-tap` updates.
- `PYPI_API_TOKEN` — protected `pypi` environment secret for production Python
  uploads.
- `TEST_PYPI_API_TOKEN` — protected `testpypi` environment secret for Python
  rehearsal uploads.
- `WINGET_GITHUB_TOKEN` — repository secret for automated winget submission.
- `SCOOP_BUCKET_TOKEN` — repository secret for Scoop bucket-manifest updates.

`.github/workflows/release-preflight.yml` is the authoritative release-time
verification mechanism. It checks the manifest-declared repository-secret
bindings, protected-environment secret metadata, required environments, and
applicable credential liveness without exposing credential values. A local or
code-review environment cannot inspect GitHub Actions secrets; that boundary
is not evidence that a configured credential is absent. For a real release,
record the workflow's sanitized result instead of creating a manual
secret-existence blocker.

Manual verification steps:

- verify crate owners:
  - `cargo owner --list sc-sha`
  - `cargo owner --list sc-composer`
  - `cargo owner --list sc-compose`
- verify the target version is unpublished before tagging:
  - `python3 .github/scripts/release_artifacts.py check-version-unpublished --manifest release/publish-artifacts.toml --version <X.Y.Z>`

## Next-Release Outputs And Channels

| Output | Destination | Current release content |
| --- | --- | --- |
| Rust crates | crates.io | `sc-sha`, `sc-composer`, and `sc-compose`, in dependency order. |
| Python distributions | TestPyPI rehearsal, then PyPI | `sc-sha` and `sc-compose`; each ships one sdist and wheels for Linux, macOS, and Windows. |
| CLI archives | GitHub Release | `sc-compose` archives for Linux x86_64, macOS arm64 and x86_64, and Windows x86_64 (MSVC and GNU); each includes bundled examples. |
| Go module | `github.com/randlee/sc-compose/bindings/sc-sha-go` | Tags use `bindings/sc-sha-go/v<version>` and bundle a matching static CGo library for Linux/amd64, macOS/arm64, and Windows/amd64. |
| macOS/Linux package manager | `randlee/homebrew-tap` | Homebrew formula `sc-compose`. |
| Windows package manager | `randlee/scoop-bucket` | Scoop manifest `sc-compose.json`. |
| Windows package manager | `microsoft/winget-pkgs` | Winget package `randlee.sc-compose`. |

Run a staged TestPyPI or `workflow_dispatch` rehearsal before treating the
Python path as production-ready. It must prove wheel and single-sdist builds,
the upload, and GitHub Release attachment behavior. The first Winget release
requires a one-time manual submission; later releases use the workflow.

The released Go consumer bundle selects its matching library from
`native/<rust-target>/`; it does not require a Cargo checkout, `go generate`,
`LD_LIBRARY_PATH`, or a system Rust installation.
