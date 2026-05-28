# ADR-0001: Observability Health Interface Stability During Panic Removal

## Status

Accepted

## Context

Phase B cleanup Sprint B13 removes runtime-reachable panic paths from
observability code on `integrate/phase-B`. The current shipped CLI entrypoint
calls `CliObserver::health()` and `health_json_value()` through `main.rs`
without any fallible interface handling. Making those interfaces fallible would
pull `main.rs` into B13 and couple B13 to the CLI extraction work assigned to
Sprint B14.

The production-readiness review distinguished two classes of panic guards:

- runtime-variable paths such as JSON serialization of health payloads and
  dynamic target/action/outcome label normalization
- startup-invariant guards over crate-owned constants such as schema version
  and service name construction

## Decision

- `CliObserver::health()` remains infallible in Sprint B13 and continues to
  return `LoggingHealthReport`.
- `health_json_value()` remains infallible at the interface level and degrades
  to a fallback JSON object instead of panicking if serialization fails.
- Dynamic target/action/outcome normalization may use internal `Result`-based
  helpers, but `emit_log()` owns the degradation policy and must emit a
  fallback event shape instead of propagating those failures across the CLI
  boundary.
- Sprint B13 closes runtime-variable panic paths only. Startup-invariant guards
  for crate-owned constants may remain infallible in B13 because changing
  `CliObserver::new()` to a fallible constructor would create a cross-sprint
  coupling with B14-owned CLI wiring.

## Consequences

- B13 can harden observability runtime behavior without changing `main.rs`.
- B14 retains ownership of CLI surface and constructor-boundary extraction
  work.
- Reviewers can distinguish runtime panic removal from startup invariant
  validation and verify B13 closure without inferring an unplanned CLI
  contract change.
