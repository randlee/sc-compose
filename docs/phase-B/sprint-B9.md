---
id: B9
title: sc-observability 1.2 Adoption
status: blocked
branch: plan/phase-B
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/phase-B
---

# Sprint B9 — sc-observability 1.2 Adoption

## Goal

Adopt `sc-observability` `1.2` in `sc-compose` after that release exists,
verify logger and observer compatibility against the report-generation
runtime, and make one explicit release-line decision about any new logging
surface required by the upgraded API.

## Blocked On

- upstream release of `sc-observability` `1.2`

This sprint is intentionally blocked until that release is available. The plan
exists now so the dependency uplift, validation scope, and exit criteria are
already explicit before implementation starts.

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
- one compatibility pass for:
  - report-generation lifecycle logging
  - retained-log maintenance
  - shutdown behavior after multi-report runs
  - text and JSON health output
- one explicit migration note describing any API changes from `1.1.0` to `1.2`
- one validation pass proving report-generation paths do not regress under the
  new logger surface

## Explicit Code Samples

```rust
pub fn shutdown(&mut self) -> Result<(), ShutdownError> {
    // update to the final 1.2 logger typestate or shutdown API once released
}
```

## This Sprint Does Not Close

- upload implementation
- network publish behavior for rendered reports

## Acceptance Criteria

- the sprint remains blocked until `sc-observability` `1.2` is released
- once unblocked, the exact `sc-compose` logger construction and shutdown
  seams required by the report runtime are identified and tested before the
  dependency uplift lands
- the retained-log and health-output behavior stay aligned with the chosen
  `1.2` API
- `docs/requirements.md`, `docs/architecture.md`, and `docs/migration-notes.md`
  remain aligned on the selected `1.2` integration shape

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
