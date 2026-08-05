# ADR-0007: Verify Library-and-CLI Boundary

## Status

Accepted

## Context

Phase D adds a drift-check capability. The prototype reference implementation
in `prototype/multipass/verify.py` already separates the core verification
behavior from CLI reporting:

- parse and render template through all passes
- read deployed file
- diff rendered output against deployed content
- return a structured result

The Rust architecture already enforces a library/CLI boundary, so verify must
fit that split cleanly.

## Decision

- `sc-composer` owns the reusable verify library entry point and structured
  `VerifyResult`.
- `sc-compose` owns CLI argument parsing, stderr/stdout behavior, quiet mode,
  and exit-code mapping at the command boundary.
- Library APIs return crate-owned composition errors; CLI command functions in
  `sc-compose` use one consistent CLI-crate error surface.

## Consequences

- Embedded callers can reuse drift-check behavior without depending on CLI
  code.
- CLI UX decisions remain isolated to `sc-compose`.
- The Phase D plan can inventory verify-specific failure modes separately from
  CLI presentation concerns.
