---
id: B9
title: sc-observability 1.2 Adoption
status: complete
branch: feat/sprint-B9
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/feat/sprint-B9
---

# Sprint B9 — sc-observability 1.2 Adoption

## Goal

Adopt `sc-observability` `1.2` in `sc-compose` after that release exists,
verify logger and observer compatibility against the report-generation
runtime, and migrate `sc-compose` to the locked `1.2.0` logging and shutdown
surface now defined in `../sc-observability`.

## Release Status

`sc-observability` `1.2.0` is now published on crates.io, so this sprint is no
longer blocked on upstream release availability.

## Hard Dependencies

- [docs/phase-A/sprint-A9.md](../phase-A/sprint-A9.md)
- [docs/phase-B/sprint-B8.md](./sprint-B8.md)

## Exact Targets

- `Cargo.toml`
- `Cargo.lock`
- `crates/sc-compose/Cargo.toml`
- `crates/sc-compose/src/observability.rs`
- `crates/sc-compose/src/observer_impl.rs`
- `crates/sc-compose/src/main.rs`
- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `docs/migration-notes.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/phase-B/sprint-B9.md`

## Deliverables

- workspace dependency uplift to `sc-observability` `1.2`
- one explicit migration of direct logger call sites away from deprecated
  `Logger::emit(...)` toward the locked `1.2.0` surface:
  - `Logger::log(...)` for blocking queue admission
  - `Logger::try_log(...)` for non-blocking queue admission where that
    behavior is required
- one compatibility pass for:
  - report-generation lifecycle logging
  - retained-log maintenance
  - shutdown behavior after multi-report runs
  - text and JSON health output
- one explicit shutdown adaptation pass for the locked `1.2.0` typestate:
  - `Logger::shutdown(self) -> Logger<Stopped>`
  - post-shutdown health inspection remains available through
    `Logger<Stopped>`
- one explicit migration note describing the locked `1.2.0` behavior changes:
  - `emit()` is deprecated but remains a compatibility path during the
    deprecation window
  - queue admission and sink durability are distinct; durability-sensitive
    paths must use `flush()` or `shutdown()`
  - shutdown does not return until the writer thread has definitively joined
- one validation pass proving report-generation paths do not regress under the
  new logger surface

## Explicit Code Samples

```rust
pub fn shutdown(&mut self) {
    if let Some(LoggerState::Running(logger)) = self.logger.take() {
        let stopped = logger.shutdown();
        self.logger = Some(LoggerState::Stopped(stopped));
    }
}
```

```rust
logger.log(event)?;
// or
logger.try_log(event)?;
```

```rust
logger.flush()?;
let stopped = logger.shutdown();
let health = stopped.health();
```

## API Reference

The `sc-observability` `1.2.0` public API surface is available on crates.io.
The implementation shape for this sprint is also confirmed against the local
repository at `../sc-observability`.

Key documents for implementation:

- `CHANGELOG.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `crates/sc-observability/src/runtime.rs`

The expected `1.2.0` surface is now both locally reviewable and published on
crates.io, so this local path is only a review/reference aid rather than an
implementation blocker.

## This Sprint Does Not Close

- upload implementation
- network publish behavior for rendered reports
- any redesign of `sc-compose` observability beyond what is required to adopt
  the locked `1.2.0` public API

## Acceptance Criteria

- the sprint uses the published `sc-observability` `1.2.0` crates plus the
  local `../sc-observability` repo state as the implementation source of truth
  for the breaking-surface review
- `sc-observability` `1.2` is resolved from crates.io (published release)
- the exact `sc-compose` logger construction, direct logging, and shutdown
  seams required by the report runtime are identified and tested before the
  dependency uplift lands
- direct `Logger::emit(...)` use in `sc-compose` is either migrated to
  `log()` / `try_log()` or retained only with an explicit compatibility
  rationale documented in `docs/migration-notes.md`
- the sprint documents and tests the queue-admission versus durability split so
  durability-sensitive paths rely on `flush()` or `shutdown()`
- the retained-log and health-output behavior stay aligned with the chosen
  `1.2` API
- `docs/requirements.md`, `docs/architecture.md`, and `docs/migration-notes.md`
  remain aligned on the selected `1.2` integration shape

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
