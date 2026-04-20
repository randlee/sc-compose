# Migration and Cutover Notes

## Background

`sc-composer` and `sc-compose` were previously developed and published from inside the
`agent-team-mail` monorepo workspace. This repository (`sc-compose`) is the new standalone
home for both crates. All future development and releases happen here.

The last versions of these crates published from the `agent-team-mail` workspace are the
baseline. This repo's standalone release version is `1.0.0`, which stays above that
baseline so crates.io version ordering remains correct through the source-of-truth cutover.

## What Changes for Downstream Consumers

### crates.io Consumers

With the first standalone `1.0.0` release from this repo:

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
   sc-composer = "1.0.0"
   ```
3. Run `cargo update` to resolve the dependency graph.
4. Run `cargo test --workspace` to verify nothing broke.
5. Delete the in-workspace crate directories for `sc-composer` and `sc-compose` once
   all dependent crates resolve cleanly.

## Breaking Changes

This is a breaking-change release relative to the last ATM-published version. Consumers
should expect:

- **API surface changes**: The public API has been redesigned across the current
  four-sprint release plan.
  Type names, module paths, and function signatures may differ from the ATM-workspace versions.
  Review `docs/requirements.md` and `docs/architecture.md` for the authoritative API contract.
- **Error type redesign**: `ComposeError`, `ResolveError`, `IncludeError`, `ValidationError`,
  `RenderError`, and `ConfigError` have been restructured. Error code strings (`ERR_*`) are
  now stable; error variant names are not guaranteed to match the prior version.
- **Observer API**: The observer/sink trait surface is new in this release. ATM adapters
  must implement the new traits. See `docs/atm-adapter-notes.md` for the integration guide.
- **Logging integration**: `sc-compose` now owns concrete structured logging through
  `sc-observability`. The CLI creates the logger, keeps file logging enabled for every
  command, suppresses the console sink whenever `--json` is active, and exposes
  `observability-health` for process-local sink/query health inspection.
- **Template whitespace behavior**: `trim_blocks` and `lstrip_blocks` are enabled by
  default. Block tags now strip the trailing newline after the block and the leading
  indentation before the next rendered content. Templates that need the previous
  whitespace-preserving behavior must opt out with the Jinja `+` modifier, for
  example `{%+ if condition %}`.
- **Binary allocator**: `sc-compose` now installs `mimalloc` as the global allocator.
  This changes the standalone binary's allocation profile without changing the
  `sc-composer` library API.
- **CLI flags**: Some CLI flags have been renamed or added. See `docs/requirements.md` FR-7
  for the complete current flag specification.

## Observability Cutover Notes

Downstream consumers that embed `sc-composer` keep using the local observer hooks.
They do not need to adopt `sc-observability` unless they want the same structured
logging behavior as the CLI.

Downstream consumers that shell out to `sc-compose` should expect:

- a new `observability-health` command for process-local logger health,
- structured JSON command output to remain clean when `--json` is active because the
  console sink is disabled in that mode,
- `observability-health --json` to serialize `logging.query` as `null` whenever
  query/follow is unavailable in the process-local logger,
- file-backed logging under `SC_LOG_ROOT` when the environment variable is set, or
  `.sc-compose/logs/` under the current working directory otherwise.
- graceful shutdown to flush logger sinks before process exit while recording sink
  degradation in health counters instead of aborting command completion.

## Release And Cutover Order

The first standalone crates.io release for this repo is `1.0.0`.

Recommended downstream cutover order:

1. Publish `sc-composer` and `sc-compose` version `1.0.0` from this repo.
2. Verify crates.io resolution and installation using the release checklist.
3. Update downstream consumers such as ATM to the published versions.
4. Run downstream integration validation after the published release is live.

## Post-Cutover Ownership

After the first standalone release:

- All future `sc-composer` and `sc-compose` releases come from this repo only.
- The `agent-team-mail` workspace no longer owns or publishes these crate names.
- crates.io ownership records should reflect this repo as the canonical source.
- See `docs/release-checklist.md` for the publish procedure.
