---
id: B13
title: Observability Panic Removal
status: planned
branch: feat/b13-observability-panic-removal
worktree: ../sc-compose-worktrees/feat/b13-observability-panic-removal
target: integrate/phase-B
---

# Sprint B13 — Observability Panic Removal

## Goal

- Remove `expect()` / `unwrap()` panic paths from production observability code.
- Preserve the shipped `sc-observability 1.2` logger lifecycle and health-report behavior while making failures explicit.
- Convert invariant assumptions around logger state and label validation into typed, testable fallbacks.
## Hard Dependencies

- `integrate/phase-B` at the current merged Phase B tip.
- The existing `sc-observability 1.2.0` integration already merged in Sprint B9.
## Exact Targets

- `crates/sc-compose/src/observability.rs`
- `crates/sc-compose/src/observer_impl.rs`
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `health_json_value()` no longer panics on serialization.
- `CliObserver::health()` no longer relies on `expect(...)` over internal logger state.
- Static target/action/outcome/service/schema normalization no longer panics in production execution paths.
- Regression tests cover degraded and fallback observability paths introduced by the refactor.
## Required Work

- Replace `serde_json::to_value(...).expect(...)` with an error-aware or fallback-producing path in `observability.rs`.
- Refactor `CliObserver` state access so health and shutdown behavior are expressed through `Result`-returning helpers rather than implicit panic invariants.
- Remove panic-based newtype helpers for service name, schema version, target, action, and outcome labels from production code paths.
- Add tests that prove observability degradation is reported cleanly instead of aborting the process.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.


```rust
pub fn health(&self) -> Result<LoggingHealthReport, CommandError>;

fn health_json_value(health: &LoggingHealthReport) -> Result<Value, CommandError>;
```

## This Sprint Does Not Close

- No new observability sinks or protocols.
- No event-taxonomy redesign.
- No ATM-specific runtime behavior.
## Acceptance Criteria

- No `expect()` or `unwrap()` remains in the listed production files.
- `health()` returns an explicit `Result`-based state instead of panicking on missing logger state.
- Observability label and schema normalization errors degrade gracefully.
- `cargo clippy --all-targets --all-features -- -D warnings` passes on the implementation branch.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
