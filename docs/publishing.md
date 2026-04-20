# Publishing and Version Ownership

## Purpose

This repo is the publishing source of truth for:
- `sc-composer`
- `sc-compose`

These crates previously existed inside the `agent-team-mail` workspace. New
releases of these crate names now come from this repo instead.

## Versioning

- The repo uses a single workspace version.
- All published crates in this repo must share that version.
- The initial standalone release must be strictly higher than the last version
  published from the ATM workspace for these crate names.
- Release workflows verify that the requested release version matches:
  - workspace version
  - each crate package version

## Replacement/Cutover Rule

Before the ATM workspace switches to crates.io dependencies from this repo:
1. This repo must publish the target version of `sc-composer`.
2. This repo must publish the target version of `sc-compose`.
3. ATM must then replace its in-workspace path dependencies with version pins.

## Source of Truth

- Manifest: `release/publish-artifacts.toml`
- Preflight workflow: `.github/workflows/release-preflight.yml`
- Release workflow: `.github/workflows/release.yml`
- Release gate script: `scripts/release_gate.sh`
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

Required secrets:

- `CARGO_REGISTRY_TOKEN`
  - must be configured in the GitHub Actions `crates-io` environment
  - must be able to publish both `sc-composer` and `sc-compose`
- `HOMEBREW_TAP_TOKEN`
  - must be configured in the repo secrets before Homebrew automation can
    update `randlee/homebrew-tap`

Manual verification steps:

- verify crate owners:
  - `cargo owner --list sc-composer`
  - `cargo owner --list sc-compose`
- verify the target version is unpublished before tagging:
  - `python3 scripts/release_artifacts.py check-version-unpublished --manifest release/publish-artifacts.toml --version <X.Y.Z>`

## Distribution Channels

The standalone release path covers:

- crates.io publication for `sc-composer` and `sc-compose`
- GitHub Release archives for Linux, macOS, and Windows
- Homebrew formula updates in `randlee/homebrew-tap`
- `winget` publication for package id `randlee.sc-compose`

The first `winget` release requires a one-time manual submission to
`microsoft/winget-pkgs`. Later releases use the automated workflow job.
