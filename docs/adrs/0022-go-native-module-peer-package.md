# ADR-0022: Go Native Module Peer Package Ownership and Release Validation

## Status

Accepted (2026-08-30).

## Context

Phase Q replaced sc-compose's locally maintained release scripts with the
canonical `sc-publish` package. That cutover correctly removed an old local
`[go_native]` extension from the core publish-artifact manifest, but it also
removed the target-aware matrix, staging, and version-lockstep logic used by
the optional `sc-sha-go` module.

The generic release matrix cannot substitute for this logic: Go native-module
support is declared by the binding's `native/targets.toml` and may be a strict
subset of generic release targets. Restoring the commands in the vendored
core script would fork canonical package behavior and repeat the cutover
failure on the next vendor update.

## Decision

1. `sc-publish/plugins/go-native-module` is the canonical optional peer
   package for Go modules that bundle a target-specific native static archive.
   It is parallel to, and independent from, `plugins/sc-publish` and
   `plugins/uniffi-bindgen-go`.
2. The peer package owns its JSON installer schema, rendered
   `release/go-native-module.toml` schema, helper, and hermetic tests.
   Consumers install byte-identical assets from an immutable package version
   and release tag or merge SHA; they do not patch copied assets locally.
3. The v1 installer input has an expected `package_version` guard plus the
   consumer-specific facts `source`, `cargo_package`, and `artifact_prefix`.
   The rendered config contains only the consumer facts. The helper derives
   and validates module identity from `<source>/go.mod`, generated package
   path and native target data from `<source>/native/targets.toml`, and
   runner/archive data from core `[[release_targets]]`.
4. The peer helper exposes three commands: `target-matrix`, `stage`, and
   `verify-version-lockstep`. The last command is invoked after the core
   lockstep check in CI, preflight, release, and release-gate paths, so an
   optional binding cannot silently leave release-version validation.
5. Core `sc-publish` keeps its closed publish-artifact schema and generic
   release scripts. There is no `[go_native]` table in the core manifest and
   no Go-native command in `release_artifacts.py`.

## Consequences

- A new optional artifact family is independently versioned, testable, and
  vendorable without changing every consumer's core release manifest.
- Consumers must pin and record the upstream package provenance, and the
  installer must fail on schema/provenance/config errors rather than guess.
- The peer package bears responsibility for helper/config semantics; the
  consumer bears responsibility for its binding target contract and for
  invoking the peer lockstep gate in every release path.
- Adding a new binding target requires changing the binding contract first;
  the peer package must not infer unsupported architecture variants.

## Alternatives Rejected

- Restore the old commands to vendored `release_artifacts.py`: creates a
  permanent local fork and makes subsequent package synchronization unsafe.
- Add a generic `[go_native]` table to core `publish-artifacts.toml`: expands a
  deliberately closed all-consumer schema for an optional artifact family.
- Use the generic release matrix: schedules targets which may not have a
  compatible cgo loader or static archive.

## References

- [S.9 remediation plan](../plans/go-native-module-remediation.md)
- [Sprint S.10](../phase-S/sprint-s-10-sc-publish-go-native-module-package.md)
- [Sprint S.11](../phase-S/sprint-s-11-sc-compose-go-native-module-adoption.md)
- [ADR-0020: Generated Go Binding Strategy](0020-generated-go-binding-strategy.md)
