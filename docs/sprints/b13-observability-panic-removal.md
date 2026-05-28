---
id: B13
title: Observability Panic Removal
status: complete
branch: feat/b13-observability-panic-removal
worktree: ../sc-compose-worktrees/feat/b13-observability-panic-removal
target: integrate/phase-B
---

# Sprint B13 — Observability Panic Removal

## Goal

- Remove runtime-variable `expect()` / `unwrap()` panic paths from production observability code.
- Preserve the shipped `sc-observability 1.2` logger lifecycle and health-report behavior while making failures explicit.
- Convert runtime logger-state and label-validation assumptions into typed, testable fallbacks without widening the CLI interface boundary.

## Hard Dependencies

- `integrate/phase-B` at the current merged Phase B tip.
- The existing `sc-observability 1.2.0` integration already merged in Sprint B9.
- [ADR-0001: Observability Health Interface Stability During Panic Removal](../adrs/0001-observability-health-interface-stability.md)
  fixes the B13/B14 boundary: B13 removes runtime-variable panic paths without
  making `main.rs` or `CliObserver::new()` fallible.

## Exact Targets

- `crates/sc-compose/src/observability.rs`
- `crates/sc-compose/src/observer_impl.rs`
- `docs/adrs/README.md`
- `docs/adrs/0001-observability-health-interface-stability.md`

## Deliverables

- `health_json_value()` no longer panics on serialization and instead degrades to a fallback JSON payload.
- `CliObserver::health()` no longer relies on `expect(...)` over internal logger state, but it remains an infallible CLI-facing interface.
- Static target/action/outcome normalization no longer panics in production execution paths.
- The B13/B14 observability interface boundary is documented in an ADR so runtime panic removal does not silently expand into CLI-surface redesign.
- Regression tests cover degraded and fallback observability paths introduced by the refactor.

## Required Work

- Replace `serde_json::to_value(...).expect(...)` with a fallback-producing path in `observability.rs` while keeping `health_json_value()` infallible at the CLI boundary.
- Refactor `CliObserver` state access so `health()` remains infallible but no longer depends on `expect(...)` over retained logger state.
- Replace the dynamic target/action/outcome normalization helpers with fallback-producing logic so runtime label errors degrade cleanly inside `emit_log()`.
- Keep `CliObserver::new()` and the crate-owned schema/service constant guards out of scope for B13 so this sprint does not pull `main.rs` into its Exact Targets.
- Add tests that prove observability degradation is reported cleanly instead of aborting the process.

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
pub fn health(&self) -> LoggingHealthReport;

fn health_json_value(health: &LoggingHealthReport) -> Value;

fn target_category(value: &str) -> Result<TargetCategory, CommandError>;
fn action_name(value: &str) -> Result<ActionName, CommandError>;
fn outcome_label(value: &str) -> Result<OutcomeLabel, CommandError>;

fn emit_log(
    &self,
    level: Level,
    target: &str,
    action: &str,
    message: impl Into<String>,
    outcome: Option<&str>,
    fields: Map<String, Value>,
);
```

## This Sprint Does Not Close

- No new observability sinks or protocols.
- No event-taxonomy redesign.
- No ATM-specific runtime behavior.
- No change to `main.rs` or the CLI-owned `run_observability_health(...)`
  boundary.
- No redesign of `CliObserver::new()` into a fallible constructor.

## Acceptance Criteria

- No runtime-variable panic path remains in the listed production files.
- `health()` remains infallible for CLI callers while no longer panicking on missing logger state.
- Observability target/action/outcome normalization errors degrade gracefully inside event emission.
- The sprint docs include an ADR-backed decision that keeps `main.rs` and `CliObserver::new()` out of scope for B13.
- `cargo clippy --all-targets --all-features -- -D warnings` passes on the implementation branch.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
