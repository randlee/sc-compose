# Publishing and Version Ownership

## Purpose

This repo becomes the publishing source of truth for:
- `sc-composer`
- `sc-compose`

These crates currently exist inside the `agent-team-mail` workspace. After
cutover, new releases of these crate names must come from this repo instead.

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
- Release notes template: `release/RELEASE-NOTES-TEMPLATE.md`
