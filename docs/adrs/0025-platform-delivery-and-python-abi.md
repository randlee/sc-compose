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

V6 implements the full matrix and validates the real pipeline with targeted
producer/install/staging checks. V7 runs the qualifying full matrix and
supported preflight after the last final-candidate change. No extra full
rehearsal is mandatory solely because work passes from V6 to V7. A valid run
may be reused when candidate SHA, version and retained bytes are identical;
a change requires a new qualifying run. Bind trusted run identity/version
and artifact SHA-256 to unchanged publisher-staged files. No post-gate rebuild
inherits old evidence; retain evidence through its publication consumer,
and fail closed if it is missing or expired.

The canonical publish kit and optional Go peer remain upstream-owned under
ADR-0022; use their supported installer inputs and immutable releases. V4 owns the generic command-model exporter; V8 owns reusable external
docli/kit consumer adapters, V9 the Go-native adapter,
and V6 final command-reference refresh and artifact composition. FR-26
requires docli as the declared consumer-preflight check run by Release Preflight
through the existing sc-publish extension, with stale-reference refusal;
individual publish workflows are not mechanically gated. Recovery does not
perform public publication.

## References

- [PyO3 0.30.1: building and distribution](https://pyo3.rs/v0.30.1/building-and-distribution)
- [CPython: limited API scope and performance](https://docs.python.org/3/c-api/stable.html)

V8 alone owns the docli preflight_checks registration run by Release Preflight
and its real current/stale consumer-preflight tests; V6 consumes the interface on final
references. Full integration runs, including Windows amd64 and manylinux,
are manual-only; routine native Windows unit checks remain normal CI.
FR-28 names V7 as owner of 1.8.0 version preparation, including manifests
and lockstep checks before the qualifying run. Production publishing is after
Phase V. FR-29/30 govern deterministic fixtures and cargo-xwin cross-checks.
