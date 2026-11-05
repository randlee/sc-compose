# ADR-0025: Complete Platform Delivery and Python Stable ABI

## Status

Accepted user requirements; amended 2026-11-04 by the Phase V owner's direct
ruling: CPython 3.9+, Windows ARM64 3.11+. Implementation is pending.

## Decision

FR-25 governs all six OS/architecture targets and five artifact families.
The preferred packaging is one abi3-py39 family with requires-python >=3.9
and accurate tags. Windows ARM64 runtime support begins at 3.11; its lower
versions are explicitly inapplicable. Prove compatibility using real symbol
audits and installed behavior. Any platform-specific build/tag exception
must be explicit and must not raise the other platforms' floor.

Stable ABI constraints can prevent version-specific optimizations; do not
claim that this application has a penalty without measurements. V5 compares
representative binding workloads on the same interpreter/build baseline.
Prefer one family. A demonstrated material cost prompts owner review of
optimized wheel variants under the same package release where possible;
3.9 support stays mandatory. No flaky timing-based unit-test gate is added.

V6 rehearses all 30 artifact/platform builds and declared install channels.
V7 dispatches the qualifying matrix and supported preflight on the exact
final candidate after its last changes, binds run identity/version/SHA-256
to retained files and verifies those files again at publisher staging.
A V6 rehearsal never substitutes for the final-candidate run. Any later
candidate/version/byte change requires another qualifying run. No rebuild
after the gate inherits old evidence. Evidence remains available through the
publication preflight that consumes it; missing/expired evidence fails closed.

The canonical publish kit and optional Go peer remain upstream-owned under
ADR-0022; use their supported installer inputs and immutable releases. V4 owns the generic command-model exporter; V8 owns reusable external
docli/kit consumer adapters, V9 the Go-native adapter,
and V6 final command-reference refresh and artifact composition. FR-26
requires docli on every publish and stale-reference refusal. Recovery does
not perform public publication.

## References

- [PyO3 0.30.1: building and distribution](https://pyo3.rs/v0.30.1/building-and-distribution)
- [CPython: limited API scope and performance](https://docs.python.org/3/c-api/stable.html)
