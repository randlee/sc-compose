# ADR-0018: sc-sha Hash Ownership and Boundary

## Status

Accepted

## Context

Before Phase M, `crates/sc-composer/src/template_hash.rs` owned the SHA-256
hashing logic used to identify rendered template content and, per PR #358,
whole compositions. Two other consumers need the same hash contract without
inheriting sc-compose's rendering, resolver, or ATM-adjacent surface:

- A Python-facing binding (`bindings/sc-sha-python`), so non-Rust callers can
  verify content identity without a full `sc-composer` dependency.
- The sibling `synaptic-canvas-dolt` repo, which needs the same hash contract
  for its own content-addressed storage and cannot depend on `sc-compose` or
  `sc-composer` (a rendering library, not a hashing library) to get it.

Leaving the hash implementation inside `sc-composer` forces every consumer to
either duplicate the algorithm (risking silent drift between copies) or pull
in MiniJinja, the resolver, and the rest of the rendering surface just to
compute a digest. `sc-composer` must also remain a pure library per
`CLAUDE.md` Boundary Rule 1 — it should not become the shared dependency of
unrelated hashing consumers outside this repo's own CLI/bindings.

## Decision

- Extract the hashing logic into a new standalone crate, `sc-sha`, added to
  the workspace at `crates/sc-sha/`. `sc-sha` is a pure computation crate: no
  filesystem access, no template syntax, no CLI behavior, no ATM crates.
- `sc-sha` exposes exactly two public operations:
  - `calculate_hash` — hashes a single piece of content into a
    `TemplateSha256` (strict UTF-8, LF-normalized).
  - `calculate_composition_hash` — hashes a caller-supplied, already-resolved,
    deduplicated manifest of `CanonicalSource`-tagged nodes into a
    `CompositionSha256`, via an injective, versioned, length-delimited
    encoding. This is the sole structural-validation gate for the manifest;
    `sc-sha` has no resolver, graph-discovery, cycle-detection, or path-policy
    behavior — those stay owned by the caller (`sc-composer`/sc-compose or, for
    the dolt use case, `synaptic-canvas-dolt` itself).
- `sc-composer` becomes a consumer of `sc-sha`, not the hash owner. It
  continues to perform filesystem canonicalization, confinement, and resolver
  work, then calls `sc-sha`'s public API with an already-canonical manifest.
  The dependency direction is `bindings/python -> sc-composer -> sc-sha`;
  `sc-sha` has no reverse dependency on `sc-composer`, `sc-compose`, or any
  binding.
- A separate `bindings/sc-sha-python` adapter (maturin/PyO3) exposes the same
  two operations to Python callers without re-implementing the algorithm and
  without acquiring a `sc-compose`/`sc-composer`/ATM dependency.
- A separate `bindings/sc-sha-go` adapter exposes the same two operations to
  Go callers. Per ADR-0020, it is a generated UniFFI adapter; it is not a
  handwritten CGo implementation and it does not introduce an async/shared-
  memory API. The adapter may depend on `sc-sha` and its approved pinned
  generator/runtime dependencies only.
- `CLAUDE.md`'s Boundary Rules make all three adapters' dependency and
  dependent constraints explicit, machine-checked via
  `boundaries/sc-sha/shared-library.toml`,
  `boundaries/sc-sha-python/python-adapter.toml`, and the forthcoming
  `boundaries/sc-sha-go/go-adapter.toml` (`sc-boundary` allowlists), in
  addition to this ADR's prose rationale.
- Phase M.1 and M.2 implemented this accepted extraction under
  `crates/sc-sha/` and `bindings/sc-sha-python/`; the public API signatures,
  manifest encoding, error taxonomy, and migration details remain documented
  in [`docs/phase-M/phase-M-plan.md`](../phase-M/phase-M-plan.md) and its sprint
  docs. This ADR records the ownership/boundary decision, not the
  implementation plan.

### `CLAUDE.md` amendment

This ADR adds Boundary Rules 8–10 to `CLAUDE.md`, stated there in short form
with a pointer back to this ADR for rationale. Rule 10 records the generated
Go adapter direction adopted by ADR-0020. See `CLAUDE.md`'s Boundary Rules
section for the accepted text.

## Consequences

- `sc-composer` stays a pure rendering library (Boundary Rule 1 intact); it no
  longer owns the only implementation of the hash contract, so
  `synaptic-canvas-dolt` and any future non-Rust consumer can depend on
  `sc-sha` directly without pulling in MiniJinja or the resolver.
- The three workspace packages (`sc-sha` plus its Python and Go adapters) must
  be kept in sync via the workspace and via
  `boundaries/sc-sha/shared-library.toml` /
  `boundaries/sc-sha-python/python-adapter.toml` /
  `boundaries/sc-sha-go/go-adapter.toml` (`sc-boundary`-enforced allowlists)
  rather than review discipline alone — an unlisted dependency edge fails CI
  instead of silently landing.
- PR #358's hash work was migrated during M.1/M.2; `sc-composer` now calls
  `sc-sha` rather than carrying a duplicate `template_hash.rs` implementation.
- `sc-sha`'s API surface is deliberately minimal (two functions); any future
  hash-domain addition (e.g. a raw-byte variant) requires a new ADR rather
  than a silent extension, consistent with the "no duplicate implementation"
  constraint carried throughout the phase-M plan.

## References

- [Phase M plan](../phase-M/phase-M-plan.md)
- [Sprint M.1 — sc-sha Core Crate](../phase-M/sprint-m-1-sc-sha-core-crate.md)
- [Sprint M.2 — sc-compose Integration](../phase-M/sprint-m-2-sc-compose-integration.md)
- PR #358 (original `crates/sc-composer/src/template_hash.rs` implementation)
- [ADR-0016: sc-lint Integration Boundary](0016-sc-lint-integration-boundary.md)
- [ADR-0017: sc-lint Runner Allowlist and Reporting](0017-sc-lint-runner-allowlist-and-reporting.md)
- [ADR-0020: Generated Go Binding Strategy](0020-generated-go-binding-strategy.md)
