# Release Checklist

Use this checklist before every crates.io release of `sc-composer` and `sc-compose`.

## Pre-Release: Version Alignment

- [ ] Workspace `Cargo.toml` `[workspace.package] version` reflects the target release version
- [ ] `crates/sc-composer/Cargo.toml` inherits or matches workspace version
- [ ] `crates/sc-compose/Cargo.toml` inherits or matches workspace version
- [ ] `release/publish-artifacts.toml` lists both crates with correct `cargo_toml` paths
- [ ] Target release version is strictly higher than the last version published from the
      `agent-team-mail` workspace for these crate names

## Pre-Release: Quality Gates

- [ ] `cargo test --workspace` passes with zero failures on the release tag commit
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes
- [ ] The full `1.1.0` promoted surface is cleared on the release branch:
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

- [ ] Verify crate owners for `sc-composer` on crates.io:
  - run `cargo owner --list sc-composer` and confirm expected owners
- [ ] Verify crate owners for `sc-compose` on crates.io:
  - run `cargo owner --list sc-compose` and confirm expected owners
- [ ] Confirm that the publish token (CARGO_REGISTRY_TOKEN) is configured in GitHub
      Actions secrets for the `release` environment
- [ ] Confirm the token has permission to publish both `sc-composer` and `sc-compose`
- [ ] Confirm `HOMEBREW_TAP_TOKEN` is configured in repo secrets before running the
      release workflow

## Pre-Release: Release Preflight

- [ ] Run `.github/workflows/release-preflight.yml` (or trigger it manually via `gh workflow run`)
- [ ] Preflight must PASS: version alignment, manifest completeness, workspace consistency

## Publish Order (MANDATORY)

Publish crates in this exact order. Do NOT publish `sc-compose` before `sc-composer` resolves
on crates.io, or the dependency graph will be broken.

1. **`sc-composer`** — publish first (`publish_order = 1`)
   - `cargo publish -p sc-composer`
   - Wait at least 30 seconds for crates.io index propagation (`wait_after_publish_seconds = 30`)
2. **`sc-compose`** — publish second (`publish_order = 2`)
   - `cargo publish -p sc-compose`

The `.github/workflows/release.yml` workflow enforces this order automatically when
triggered by a release tag.

## Post-Publish Verification

- [ ] Verify `sc-composer` is visible on crates.io at the expected version
- [ ] Verify `sc-compose` is visible on crates.io at the expected version
- [ ] Run `cargo add sc-composer@<version>` in a scratch workspace to confirm the crate resolves
- [ ] Run `cargo install sc-compose@<version>` to confirm the binary installs cleanly
- [ ] Verify the GitHub Release archives include `share/sc-compose/examples/`
- [ ] Verify the Homebrew formula update completed in `randlee/homebrew-tap`
- [ ] Verify the `winget` submission/update was dispatched successfully
- [ ] Update `release/RELEASE-NOTES-TEMPLATE.md` with the actual release summary
- [ ] Tag the release commit: `git tag v<version> && git push origin v<version>`
- [ ] Create a GitHub release pointing at the tag with the filled-in release notes

## Release Authorization

- [ ] The `1.1.0` promoted surface is cleared on the release branch:
  - HTML-report line remains release-ready
  - Phase A and Phase B reporting runtime remains release-ready
  - publish-manifest handoff remains release-ready
  - `sc-observability 1.2.0` observability behavior remains release-ready
- [ ] standalone boundary verification passes with no forbidden ATM references in source
- [ ] downstream cutover notes are published alongside the release notes
