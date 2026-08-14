# Release Checklist

Use this checklist before every crates.io release of `sc-sha`, `sc-composer`, and `sc-compose`.

## Pre-Release: Version Alignment

- [ ] Workspace `Cargo.toml` `[workspace.package] version` reflects the target release version
- [ ] `crates/sc-sha/Cargo.toml` inherits or matches workspace version
- [ ] `crates/sc-composer/Cargo.toml` inherits or matches workspace version
- [ ] `crates/sc-compose/Cargo.toml` inherits or matches workspace version
- [ ] `release/publish-artifacts.toml` lists `sc-sha`, `sc-composer`, and `sc-compose`
      with correct `cargo_toml` paths and dependency-aware publish order
- [ ] `bindings/python/pyproject.toml` and `bindings/sc-sha-python/pyproject.toml`
      match the workspace version, and their explicit Cargo path-dependency pins do too
- [ ] Target release version is strictly higher than the last version published from the
      `agent-team-mail` workspace for these crate names

## Pre-Release: Quality Gates

- [ ] `cargo test --workspace` passes with zero failures on the release tag commit
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes
- [ ] The full promoted surface for the target release is cleared on the release branch:
  - HTML-report functionality and examples remain covered by tests
  - Phase A and Phase B reporting commands remain covered by tests:
    `reports init`, `reports smoke`, `reports finalize`, `reports render-spec`,
    `reports index`, `reports verify`, and `reports publish-manifest`
  - the checked-in `Justfile` proof path remains release-ready:
    `just lint`, `just test`, `just smoke`, `just state-diagrams`,
    `just sql-diagrams`, `just reports`, and `just reports-verify`
  - publish-manifest handoff remains verified through
    `reports/latest/publish-manifest.json`
  - `observability-health` and the shipped `sc-observability 1.2.0`
    queue-admission / shutdown behavior remain covered by tests
  - failure-mode matrix `ERR_*` codes exercised by tests
  - `--json` commands are verified to keep stdout machine-readable
  - `quality-mgr` full QA pass on the release branch
  - `team-lead` final design review complete

## Pre-Release: crates.io Ownership

- [ ] Verify crate owners for `sc-sha` on crates.io:
  - run `cargo owner --list sc-sha` and confirm expected owners
- [ ] Verify crate owners for `sc-composer` on crates.io:
  - run `cargo owner --list sc-composer` and confirm expected owners
- [ ] Verify crate owners for `sc-compose` on crates.io:
  - run `cargo owner --list sc-compose` and confirm expected owners
- [ ] Use the standard pre-provisioned GitHub Actions secret contract:
      `CARGO_REGISTRY_TOKEN`, `PYPI_TOKEN`, `TEST_PYPI_TOKEN`,
      `HOMEBREW_TAP_TOKEN`, `WINGET_GITHUB_TOKEN`, and `SCOOP_BUCKET_TOKEN`.
      The publisher does not inspect, request, or locally replace these secrets.

## Pre-Release: Release Preflight

- [ ] Run `.github/workflows/release-preflight.yml` (or trigger it manually via `gh workflow run`)
- [ ] Preflight must PASS: version alignment, manifest completeness, workspace consistency

## Publish Order (MANDATORY)

Publish crates in this exact order. Do NOT publish a dependent before its dependency resolves
on crates.io, or the dependency graph will be broken.

1. **`sc-sha`** — publish first (`publish_order = 1`)
   - `cargo publish -p sc-sha`
   - Wait at least 30 seconds for crates.io index propagation (`wait_after_publish_seconds = 30`)
2. **`sc-composer`** — publish second (`publish_order = 2`)
   - `cargo publish -p sc-composer`
   - Wait at least 30 seconds for crates.io index propagation (`wait_after_publish_seconds = 30`)
3. **`sc-compose`** — publish third (`publish_order = 3`)
   - `cargo publish -p sc-compose`

The `.github/workflows/release.yml` workflow enforces this order automatically
for a production `workflow_dispatch` release.

## Post-GitHub-Release Channel Dispatch

The root `Release` workflow creates the protected tag, publishes crates.io
packages, builds the authoritative artifacts, and creates the GitHub Release.
It deliberately does **not** publish production PyPI packages, update
Homebrew, or submit `winget`. After verifying the GitHub Release, dispatch all
three recovery-safe channel workflows with the same `v<version>` tag:

- [ ] Run `.github/workflows/pypi-publish.yml` with `tag=v<version>` and
      `target=production`.
  - It uploads only the six wheels and two source distributions attached to the
    published GitHub Release; it does not rebuild artifacts or create a tag.
- [ ] Run `.github/workflows/homebrew-publish.yml` with `tag=v<version>`.
  - It verifies all three Unix archives, renders the formula from the tagged
    source, validates Ruby syntax, and updates `randlee/homebrew-tap`.
- [ ] Run `.github/workflows/winget-publish.yml` with `tag=v<version>`.
  - It verifies the published Windows ZIP before submitting the package update
    with `WINGET_GITHUB_TOKEN`.

Each workflow may be safely re-dispatched for that release if its channel
fails. Do not rerun the root `Release` workflow to recover an external-channel
failure.

## Post-Publish Verification

- [ ] Verify `sc-sha` is visible on crates.io at the expected version
- [ ] Verify `sc-composer` is visible on crates.io at the expected version
- [ ] Verify `sc-compose` is visible on crates.io at the expected version
- [ ] Run `cargo add sc-composer@<version>` in a scratch workspace to confirm the crate resolves
- [ ] Run `cargo install sc-compose@<version>` to confirm the binary installs cleanly
- [ ] Verify the GitHub Release archives include `share/sc-compose/examples/`
- [ ] Verify the production `Publish PyPI` workflow uploaded `sc-compose` and
      `sc-sha` from the GitHub Release assets
- [ ] Verify the Homebrew formula update completed in `randlee/homebrew-tap`
- [ ] Verify the `winget` submission/update was dispatched successfully
- [ ] Update `release/RELEASE-NOTES-TEMPLATE.md` with the actual release summary
- [ ] Confirm the release workflow's `gate-and-tag` job created `v<version>` — do not tag manually; the tag branch is protected.
- [ ] Verify the root release workflow created a GitHub Release pointing at the
      protected tag with the release notes

## Release Authorization

- [ ] The promoted surface for the target release is cleared on the release branch:
  - HTML-report line remains release-ready
  - Phase A and Phase B reporting runtime remains release-ready
  - publish-manifest handoff remains release-ready
  - `sc-observability 1.2.0` observability behavior remains release-ready
- [ ] standalone boundary verification passes with no forbidden ATM references in source
- [ ] downstream cutover notes are published alongside the release notes
