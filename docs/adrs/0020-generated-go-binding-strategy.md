# ADR-0020: Generated Go Binding Strategy

## Status

Proposed — requires team-lead and architecture review before a Go adapter is
implemented or a `CLAUDE.md` boundary amendment is committed.

## Context

`sc-sha` is the first Rust library in this repository that needs a Go-facing
API, but it will not be the last. `sc-dolt` and atm-core need the shared hash
contract now; future consumers may need a much larger `sc-composer` surface.

Maintaining handwritten C ABI declarations and CGo wrappers for each adapter
would create duplicate marshaling, ownership, error, and API-versioning logic.
That is tolerable for one function but not a sound shared-library strategy.

The relevant generator trade-off is not language preference:

| Interface need | Preferred generator |
| --- | --- |
| Ordinary synchronous APIs; records, enums, typed errors, and Go objects that implement Rust traits | UniFFI with `uniffi-bindgen-go` |
| Proven need for asynchronous Rust-to-Go calls or high-throughput shared-memory exchange | rust2go, under an adapter-specific decision gate |

Interoptopus is not selected for this repository's Go path: it does not offer
a supported Go generator. UniFFI's Go generator is third-party and currently
pre-1.0, so its version pairing and generated output need deliberate CI
verification. rust2go provides a different value proposition, but its shared
memory/callback model has stricter pointer-lifetime obligations; its own
documentation describes configurations that disable Go pointer checks. A
shared library must not require those safety checks to be disabled.

## Decision

### 1. Keep Rust domain libraries generator-free

`sc-sha`, `sc-composer`, and future Rust domain crates remain the sole owners
of domain behavior and public Rust APIs. They do not depend on UniFFI,
rust2go, CGo, Go tooling, or a foreign-language runtime.

Each foreign-language surface is a dedicated adapter, for example:

```text
crates/sc-sha                 # domain contract and implementation
bindings/sc-sha-go            # generated UniFFI Go adapter
bindings/sc-composer-go       # future, separately approved adapter
bindings/<product>-rust2go    # only when an async/shared-memory ADR approves it
```

No adapter may reimplement a hash, render, resolver, or error algorithm.

### 2. Use UniFFI as the default generated Go surface

New ordinary Go adapters use UniFFI and `uniffi-bindgen-go`. The adapter owns
the UniFFI interface definition, generated scaffolding, foreign-language
package metadata, and generator configuration. The domain crate owns neither
the generator nor generated code.

For every adapter, the implementation must:

- pin a compatible pair of UniFFI and `uniffi-bindgen-go` versions (the first
  approved pair is UniFFI `0.31.0` with `uniffi-bindgen-go`
  `v0.7.1+v0.31.0`);
- generate Go source from the committed interface definition, never by editing
  generated Go source by hand;
- commit generated source that is required to consume the released Go module;
- have CI regenerate it and fail on drift;
- publish typed Go records/enums/errors rather than an untyped JSON or
  `map[string]any` façade; and
- run cross-language conformance vectors against the same Rust-core values and
  stable error codes used by every other adapter.

UniFFI foreign traits are the preferred future mechanism where a Go object must
implement a Rust trait. New work uses UniFFI foreign-trait syntax rather than
the older callback-interface form, because UniFFI documents callback interfaces
as soft-deprecated.

### 3. Make rust2go a deliberate async/shared-memory opt-in

rust2go is permitted only in a separate adapter when the owning sprint shows
that a UniFFI-shaped interface cannot meet a real async callback or
high-throughput shared-memory requirement. Before such an adapter is started,
its ADR/sprint must document:

- the required direction(s) of asynchronous calls and callback lifecycle;
- the data types, ownership, cancellation, and drop behavior crossing the
  boundary;
- why copy/serialization overhead is material for the named workload;
- tests under normal Go pointer checking; and
- the exact ABI/module/version compatibility policy.

An adapter must not require callers, CI, or production services to set
`GODEBUG=cgocheck=0`, `GODEBUG=invalidptr=0`, or an equivalent setting that
weakens Go's FFI safety checks. If an intended rust2go interface cannot satisfy
that constraint, it is not approved for a shared library.

### 4. Allow both generators in one repository, but never in one adapter

A repository may contain a UniFFI adapter and one or more rust2go adapters.
They remain separate packages, module paths, generated artifacts, and release
targets. A single adapter must use exactly one generator; it must not mix
UniFFI and rust2go runtime types, callback registries, or memory ownership
models.

Where both adapters expose the same core operation, they must consume common
test vectors and produce the same successful values and stable error codes.
The Rust domain crate remains the only source of semantic behavior.

### 5. Apply the default to `sc-sha`; do not pre-authorize `sc-composer`

The initial `bindings/sc-sha-go` adapter uses the default UniFFI path. It
exports the existing two synchronous operations, `CalculateHash` and
`CalculateCompositionHash`, with typed values and errors generated from the
adapter contract. It does not add an artificial async API merely to exercise
rust2go.

Go bindings for `sc-composer` are a separate, uncommitted follow-on. Their
future plan must assess the actual API shape: use the UniFFI default for normal
rendering/value contracts, or seek a dedicated rust2go ADR only for named
async/shared-memory behavior.

### 6. Enforce the boundary mechanically

The `sc-sha` Go implementation sprint must amend ADR-0018, `CLAUDE.md`, and
the `sc-boundary` inventories before adapter code is authored. The amendment
must allow the new `bindings/sc-sha-go` adapter while preserving these rules:

- `sc-sha` remains a pure computation crate with no reverse dependency on any
  adapter;
- the Go adapter may depend on `sc-sha` and its approved generator/runtime
  dependencies only;
- neither adapter may depend on `sc-compose`, ATM code, filesystem/resolver
  policy, or another foreign-language adapter; and
- a negative boundary fixture proves a forbidden reverse or cross-adapter edge
  fails CI.

## Consequences

- Go bindings scale through a generator-owned interface definition rather than
  copied CGo glue.
- The first Go adapter has an explicit generator compatibility gate, so a
  third-party generator upgrade cannot silently change the released API.
- `sc-sha` remains small: UniFFI-related dependencies are isolated to its Go
  adapter rather than entering its core crate.
- Repositories can adopt rust2go later without making an async/shared-memory
  model mandatory for all Go consumers.
- A future large `sc-composer` Go binding can share the policy, generator
  workflow, CI drift checks, and conformance approach without being bundled
  into the `sc-sha` delivery.

## Rejected alternatives

### Handwritten C ABI and CGo wrappers as the standard

Rejected. They make every new API surface responsible for repeated ownership,
error, marshaling, and drift logic. Public Go bindings are generated; a
generator limitation is a compatibility/design problem to resolve explicitly,
not permission to introduce handwritten public CGo glue.

### One generator for every possible workload

Rejected. UniFFI and rust2go solve different problems. Requiring rust2go for
all adapters would import async/shared-memory lifetime complexity into simple
SDK calls; requiring UniFFI for every adapter could rule out a justified
high-throughput Rust↔Go async path.

### Implementing both generators inside one Go package

Rejected. It would obscure ABI ownership and make lifetime, error, and release
behavior impossible to reason about. A consumer selects an adapter package
explicitly.

## References

- [ADR-0018: sc-sha Hash Ownership and Boundary](0018-sc-sha-hash-ownership.md)
- [UniFFI foreign trait guidance](https://mozilla.github.io/uniffi-rs/next/proc_macro/traits.html)
- [NordSecurity `uniffi-bindgen-go`](https://github.com/NordSecurity/uniffi-bindgen-go)
- [rust2go](https://github.com/ihciah/rust2go)
