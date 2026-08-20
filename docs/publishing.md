# Publishing and Version Ownership

## Purpose

This repo is the publishing source of truth for:
- `sc-sha`
- `sc-composer`
- `sc-compose`

## Versioning

- The repo uses a single workspace version.
- All published crates in this repo must share that version.
- Release workflows verify that the requested release version matches:
  - workspace version
  - each crate package version
- The Phase C Python release channel must also sync
  `bindings/sc-sha-python/pyproject.toml` and
  `bindings/python/pyproject.toml` from the workspace version immediately
  before wheel or sdist builds and then fail release if
  `verify-python-version` detects drift.

## Source of Truth

- Manifest: `release/publish-artifacts.toml`
- Preflight workflow: `.github/workflows/release-preflight.yml`
- Release workflow: `.github/workflows/release.yml`
- Release gate helper: `.github/scripts/release_artifacts.py` (installed from
  `plugins/sc-publish`)
- Release notes template: `release/RELEASE-NOTES-TEMPLATE.md`
- `winget` setup note: `docs/WINGET_SETUP.md`
- Operator guide: `docs/publishing-agent.md`

## Installed Data Layout

Bundled example templates are installed under the shared data root:

- Homebrew (macOS): `$(brew --prefix)/share/sc-compose/examples/`
- FHS-style Linux packages: `<prefix>/share/sc-compose/examples/`
- Other system installs: `<install-root>/share/sc-compose/examples/`

At runtime, `sc-compose` resolves bundled examples from:

1. `SC_COMPOSE_DATA_DIR/examples/` when `SC_COMPOSE_DATA_DIR` is set
2. install-relative `../share/sc-compose/examples/` next to the binary

Package builds must preserve that share layout so `sc-compose examples list`
and `sc-compose examples <name>` work without extra configuration.

GitHub Release archives and package-manager installs must ship both:

- `bin/sc-compose`
- `share/sc-compose/examples/...`

## User Template Root

User-managed template packs resolve from:

1. `SC_COMPOSE_TEMPLATE_DIR` when set
2. the platform user-data directory joined with `sc-compose/templates/`

Packaging guidance:

- Do not package user templates into the shared examples root.
- Do not set `SC_COMPOSE_TEMPLATE_DIR` in package wrappers by default.
- Document `SC_COMPOSE_DATA_DIR` as the override for CI, custom installs, and
  nonstandard packaging layouts.

## Cargo Install Limitation

`cargo install` publishes and installs the binary only. It does not install the
bundled examples directory. Bundled examples are guaranteed in:

- Homebrew installs
- `winget` installs
- GitHub Release archives

Users who install with Cargo can point `SC_COMPOSE_DATA_DIR` at a manual copy of
the examples root when they want `examples list` and `examples <name>`.

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

## Distribution Channels

The standalone release path covers:

- crates.io publication for `sc-sha`, `sc-composer`, and `sc-compose`
- GitHub Release archives for Linux, macOS, and Windows
- Homebrew formula updates in `randlee/homebrew-tap`
- `winget` publication for package id `randlee.sc-compose`
- PyPI publication for packages `sc-sha` and `sc-compose`

Python release-train rule:

- do not treat the Python release path as production-closed until a staged
  TestPyPI or `workflow_dispatch` rehearsal proves wheel build, single-sdist
  build, publish, and GitHub Release attachment behavior end-to-end

Release-operator verification for PyPI:

- run one staged TestPyPI or `workflow_dispatch` rehearsal before treating the
  Python release channel as production-closed
- confirm exactly one sdist is produced, all three wheel builds complete, the
  PyPI upload path succeeds, and the GitHub Release attachment set includes
  wheels plus the single sdist

The first `winget` release requires a one-time manual submission to
`microsoft/winget-pkgs`. Later releases use the automated workflow job.

## Report Publication Handoff

Generated report evidence uses one machine-readable handoff file:

- `sc-compose reports publish-manifest --root .`
- writes `reports/latest/publish-manifest.json`

That manifest lists:

- each publishable report present in the current latest artifact set
- each report artifact path
- the intended publish destination for each artifact
- the latest archive snapshot path for the report when one exists

`sc-compose` does not upload, copy, or host those artifacts. CI and wrapper
tooling consume the manifest and perform publication separately.

## Generated `sc-sha-go` module

The Go adapter is released as the versioned module
`github.com/randlee/sc-compose/bindings/sc-sha-go` using tags of
the form `bindings/sc-sha-go/v<version>`. Release CI stages one matching
static native library per advertised OS/architecture and runs an independent
temporary Go consumer module. Consumers must use the released bundle; they do
not use a Cargo checkout or a system Rust library.
