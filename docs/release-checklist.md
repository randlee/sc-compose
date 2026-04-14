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
- [ ] Sprint 4 exit gate is fully cleared:
  - all FR-1 through FR-11 requirements implemented and covered by tests
  - failure-mode matrix ERR_* codes exercised by tests
  - full end-to-end smoke test passes, including `observability-health`
  - `--json` commands are verified to keep stdout machine-readable
  - `qm-comp` full QA pass on the release branch
  - `arch-ctm` final design review complete

## Pre-Release: crates.io Ownership

- [ ] Verify crate owners for `sc-composer` on crates.io:
  - run `cargo owner --list sc-composer` and confirm expected owners
- [ ] Verify crate owners for `sc-compose` on crates.io:
  - run `cargo owner --list sc-compose` and confirm expected owners
- [ ] Confirm that the publish token (CARGO_REGISTRY_TOKEN) is configured in GitHub
      Actions secrets for the `release` environment
- [ ] Confirm the token has permission to publish both `sc-composer` and `sc-compose`

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
- [ ] Update `release/RELEASE-NOTES-TEMPLATE.md` with the actual release summary
- [ ] Tag the release commit: `git tag v<version> && git push origin v<version>`
- [ ] Create a GitHub release pointing at the tag with the filled-in release notes

## Deferred: First Standalone Release

The initial standalone release from this repo is deferred until downstream product integration
is complete. See `docs/migration-notes.md` for the cutover plan and blocking conditions.

Do NOT publish until the downstream integration gate is cleared.
