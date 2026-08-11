# ADR-0018: sc-sha Hash Ownership and Boundary

## Status

Accepted

## Context

`crates/sc-composer/src/template_hash.rs` currently owns the SHA-256 hashing
logic used to identify rendered template content and, per PR #358, whole
compositions. Two other consumers need the same hash contract without
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
- `CLAUDE.md`'s Boundary Rules gain two new rules (proposed text below) making
  both new crates' dependency and dependent constraints explicit, machine-
  checked via `boundaries/sc-sha/shared-library.toml` and
  `boundaries/sc-sha-python/python-adapter.toml` (`sc-boundary` allowlists) in
  addition to this ADR's prose rationale.
- No source under `crates/sc-sha/` or `bindings/sc-sha-python/` is authored or
  staged until this ADR and the `CLAUDE.md` amendment are both accepted by
  team-lead. Full implementation detail (public API signatures, manifest
  encoding, error taxonomy, migration steps) lives in
  [`docs/phase-M/phase-M-plan.md`](../phase-M/phase-M-plan.md) and its sprint
  docs; this ADR records the ownership/boundary decision, not the
  implementation plan.

### Proposed `CLAUDE.md` amendment

Append to the existing Boundary Rules section (currently numbered 1-7):

```diff
 6. Do not read `ATM_HOME`.
 7. Any ATM integration belongs in ATM adapters, not in this repo.
+8. `sc-sha` is a pure computation crate. It may depend only on the approved
+   hashing/encoding implementation dependencies, and may be depended on only
+   by `sc-composer` and `bindings/sc-sha-python`. It must not depend on
+   `sc-compose`, `sc-composer`, MiniJinja, filesystem/CLI libraries, ATM
+   crates, PyO3, or maturin, and must not implement resolver, path-policy,
+   cycle-detection, or depth-limiting behavior.
+9. `bindings/sc-sha-python` is a Python-facing adapter for `sc-sha` only. It
+   may depend on published `sc-sha` plus PyO3/maturin packaging dependencies
+   only. It must not depend on `sc-compose`, `sc-composer`, ATM-specific
+   crates, or read `ATM_HOME`, and it must delegate both public operations to
+   `sc-sha` without a Python-only algorithm.
```

This amendment does not take effect, and no source under `crates/sc-sha/` or
`bindings/sc-sha-python/` may be authored, until team-lead accepts this ADR.

## Consequences

- `sc-composer` stays a pure rendering library (Boundary Rule 1 intact); it no
  longer owns the only implementation of the hash contract, so
  `synaptic-canvas-dolt` and any future non-Rust consumer can depend on
  `sc-sha` directly without pulling in MiniJinja or the resolver.
- Two additional crates must be kept in sync via the workspace and via
  `boundaries/sc-sha/shared-library.toml` /
  `boundaries/sc-sha-python/python-adapter.toml` (`sc-boundary`-enforced
  allowlists) rather than review discipline alone — an unlisted dependency
  edge fails CI instead of silently landing.
- PR #358's in-progress hash work in `sc-composer` must be migrated to call
  `sc-sha` rather than reimplement it once M.1/M.2 land; until this ADR is
  accepted, PR #358 continues on its existing `template_hash.rs`
  implementation unchanged.
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
