# Migration and Cutover Notes

## Background

`sc-composer` and `sc-compose` were previously developed and published from inside the
`agent-team-mail` monorepo workspace. This repository (`sc-compose`) is the new standalone
home for both crates. All future development and releases happen here.

The last versions of these crates published from the `agent-team-mail` workspace are the
baseline. This repo's workspace version (currently `0.46.2`) is set above that baseline to
ensure crates.io version ordering is correct when the first standalone release occurs.

## What Changes for Downstream Consumers

### crates.io Consumers

Once the first release from this repo is published:

- Downstream crates that depend on `sc-composer` or `sc-compose` via crates.io will
  automatically resolve to the new source with no manifest change required, as long as
  their version constraint covers the new release.
- Consumers pinned to a pre-standalone version will need to update their version pin.

### ATM Workspace (agent-team-mail)

ATM currently consumes `sc-composer` and `sc-compose` as in-workspace path dependencies.
The cutover replaces those path dependencies with crates.io version pins.

Cutover steps for the ATM workspace maintainer:

1. Confirm the target standalone version is live on crates.io.
2. In `Cargo.toml`, replace path dependency entries for `sc-composer` and `sc-compose`
   with version-pinned crates.io entries:
   ```toml
   # Before (path dependency):
   sc-composer = { path = "../sc-composer" }

   # After (crates.io pin):
   sc-composer = "0.46.2"
   ```
3. Run `cargo update` to resolve the dependency graph.
4. Run `cargo test --workspace` to verify nothing broke.
5. Delete the in-workspace crate directories for `sc-composer` and `sc-compose` once
   all dependent crates resolve cleanly.

## Breaking Changes

This is a breaking-change release relative to the last ATM-published version. Consumers
should expect:

- **API surface changes**: The public API has been redesigned through sprints S2-S6.
  Type names, module paths, and function signatures may differ from the ATM-workspace versions.
  Review `docs/requirements.md` and `docs/architecture.md` for the authoritative API contract.
- **Error type redesign**: `ComposeError`, `ResolveError`, `IncludeError`, `ValidationError`,
  `RenderError`, and `ConfigError` have been restructured. Error code strings (`ERR_*`) are
  now stable; error variant names are not guaranteed to match the prior version.
- **Observer API**: The observer/sink trait surface is new in this release. ATM adapters
  must implement the new traits. See `docs/atm-adapter-notes.md` for the integration guide.
- **CLI flags**: Some CLI flags have been renamed or added. See `docs/requirements.md` FR-7
  for the complete current flag specification.

## Deferred Publish: Blocking Conditions

The first standalone crates.io release is **deferred** until both conditions are met:

1. **Downstream integration is complete**: at least one downstream consumer (ATM or another
   product) has been updated to use the new API and the integration has been verified in a
   non-production environment.
2. **Integration gate is cleared**: `qm-comp` and `arch-ctm` have signed off on the
   integration test results.

Do NOT publish before these conditions are met, even if the sprint exit gates are all passing.
The version number `0.46.2` is intentionally held until integration is ready.

## Post-Cutover Ownership

After the first standalone release:

- All future `sc-composer` and `sc-compose` releases come from this repo only.
- The `agent-team-mail` workspace no longer owns or publishes these crate names.
- crates.io ownership records should reflect this repo as the canonical source.
- See `docs/release-checklist.md` for the publish procedure.
