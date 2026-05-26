---
id: A9
title: sc-observability 1.1.0 Adoption
status: complete
branch: feat/sprint-A9
worktree: /Users/randlee/Documents/github/sc-compose-sprint-A9
---

# Sprint A9 — sc-observability 1.1.0 Adoption

## Goal

Adopt `sc-observability` `1.1.0` in `sc-compose`, verify logger and observer
compatibility, migrate deprecated `emit` usage to `log` / `try_log`, and make
one explicit release-line decision about retained-log policy for the local CLI
log path used by report-producing workflows.

Issue driver:

- GitHub issue `#57` in `sc-compose`

## Hard Dependencies

- [docs/project-plan.md](../project-plan.md)
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/phase-A/sprint-A8.md](./sprint-A8.md)

## Exact Targets

- `Cargo.toml`
- `crates/sc-compose/Cargo.toml`
- `crates/sc-compose/src/observability.rs`
- `crates/sc-compose/src/observer_impl.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/migration-notes.md`
- `docs/phase-A/phase-A-plan.md`
- `docs/phase-A/sprint-A9.md`

## Deliverables

- workspace dependency uplift to `sc-observability` `1.1.0`
- explicit compatibility verification for:
  - logger construction in `build_logger_for_root(...)`
  - `CliObserver::shutdown()`
  - `observability-health`
  - `--json` cleanliness after the dependency refresh
- one explicit migration decision for deprecated `emit` call sites:
  - issue `#57` anticipated a `try_log` / `log` migration, but the released
    `sc-observability` `1.1.0` API used by `sc-compose` still emits through
    `Logger::emit(...)`
  - this sprint therefore keeps direct `Logger::emit(...)` for CLI lifecycle
    and composition events rather than inventing a wrapper-only migration
- one explicit retained-log policy decision for the `.sc-compose` local log
  root:
  - enable `RetainedLogPolicy::default()` with documented upstream defaults:
    `64 MiB` active-log rotation, `10` retained rotated files, `7`-day max
    age, `60`-second maintenance cadence, and `5`-second shutdown join timeout
  - the logger itself owns rotation, pruning, and background maintenance
    behavior; `sc-compose` does not reimplement those duties
- one explicit decision on the `sc-observe` facade:
  - keep direct `sc-observability` use because `sc-compose` constructs the
    logger directly, manages sink registration directly, and does not benefit
    from an additional facade at this seam
- Windows rotation compatibility included in the validation target because
  `sc-compose` is a tiered cross-platform CLI with Windows release paths

## Explicit Code Samples

```rust
pub fn shutdown(&mut self) {
    if let Some(LoggerState::Running(logger)) = self.logger.take() {
        self.logger = logger.shutdown().ok().map(LoggerState::Stopped);
    }
}
```

```rust
let mut config = LoggerConfig::default_for(service_name, log_root);
config.enable_console_sink = false;
config.retained_log_policy = RetainedLogPolicy::default();
```

```rust
if let Some(LoggerState::Running(logger)) = &self.logger {
    let _ignored = logger.emit(event);
}
```

## This Sprint Does Not Close

- a redesign of the `sc-composer` observer contract
- OTLP rollout
- network publish behavior for rendered reports

## Acceptance Criteria

- the sprint identifies the exact `sc-compose` logger construction and
  shutdown seams that must compile unchanged or be minimally adapted for
  logger typestate
- the sprint identifies the exact deprecated `emit` call sites and makes one
  explicit `log` versus `try_log` decision for them
- the sprint keeps `sc-composer` free of direct `sc-observability` and
  `sc-observability-types` dependencies
- the sprint makes one explicit yes/no decision about retained-log policy for
  `.sc-compose` local logs
- when retained logging is enabled, the sprint states that the logger itself
  owns rotation/pruning/background maintenance based on configured settings
- the sprint makes one explicit yes/no decision about adopting `sc-observe`
- `docs/requirements.md`, `docs/architecture.md`, and `docs/migration-notes.md`
  remain aligned on the chosen `1.1.0` integration shape

## Required Validation

- `cargo fmt --all --check`
