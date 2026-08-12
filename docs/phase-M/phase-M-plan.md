---
phase: M
title: sc-sha Extraction and Recursive Template Composition Fingerprints
status: planned
target: integrate/phase-m
---

# Phase M — `sc-sha` Extraction and Recursive Template Composition Fingerprints

## Plan status

- Type: planning/design document
- Planning branch: `plan/sha-crate`
- Planning worktree: `../sc-compose-worktrees/plan/sha-crate`
- Implementation target: `develop`, followed by `integrate/phase-m`
- Current related work: [PR #358](https://github.com/randlee/sc-compose/pull/358)
- Related product issue: [sc-compose #360](https://github.com/randlee/sc-compose/issues/360)
- External algorithm authority: synaptic-canvas-dolt, verified at the source
  locations listed below; the exact source commit still must be recorded

This document is intentionally planning-only. It does not add a Cargo member,
move `template_hash.rs`, alter PR #358, or implement recursive hashing.

This is the authoritative Phase M plan, not an implementation sprint. Runtime
work is defined by the two standalone sprint documents:

- [Sprint M.1 — sc-sha core crate](sprint-m-1-sc-sha-core-crate.md)
- [Sprint M.2 — sc-compose integration](sprint-m-2-sc-compose-integration.md)

The sprint documents own their implementation targets, deliverables,
acceptance criteria, paths to delete, and required validation. This plan owns
the phase-level contract, boundaries, sequencing, and external handoff.

## Requirements, ADR, and NFR traceability

The following IDs are the authoritative traceability vocabulary for both
implementation sprint records. They make the scope reviewable without relying
on conversation history.

| ID | Requirement or constraint | Evidence/authority | Sprint closure |
| --- | --- | --- | --- |
| SHA-R1 | Match synaptic-canvas-dolt's verified UTF-8 text, newline, error, digest, and `package_files.sha256` consumption contract. | Issue [#360](https://github.com/randlee/sc-compose/issues/360), PR [#358](https://github.com/randlee/sc-compose/pull/358), source vectors in `Source-of-truth verification gate` | M.1 defines; M.2 verifies Rust/Python consumption |
| SHA-R2 | Provide exactly two public operations: typed per-content hash calculation and composition hashing over a caller-supplied resolved manifest. | User-approved API constraint; `Proposed sc-sha public API` | M.1 defines; M.2 consumes |
| SHA-R3 | sc-compose owns dependency discovery, canonicalization, ordering, deduplication, memoization, cycle/depth checks, and produces an injectively encodable resolved manifest for sc-sha. | `Sprint M.2 — Recursive sc-compose integration contract` and `Resolved manifest contract` | M.2 |
| SHA-R4 | Allow atm-core to restore cached context from JSON variables plus recursive template identity; graph construction and persistence remain atm-core concerns. | atm-core consumer contract and acceptance fixtures | M.1 defines; external acceptance after M.2 |
| SHA-R5 | Provide a thin maturin/PyO3 adapter with the same two operations and no duplicated algorithm. | `Maturin/Python API` | M.2 |
| SHA-N1 | Hash identity is deterministic across macOS, Linux, and Windows and independent of host path separators/line endings. | `Source-of-truth verification gate`; cross-platform fixtures | M.1 and M.2 |
| SHA-N2 | Keep filesystem policy, template syntax, CLI behavior, and persistence outside the core crate. | `Proposed workspace layout` and boundary rules | M.1 and M.2 |
| SHA-N3 | Keep the implementation production-ready: typed failures, consumer-owned bounded recursion, no duplicate implementation, and reproducible validation evidence. | `API simplicity requirement`, required validation, and QA routing | M.1 and M.2 |
| ADR-SHA-001 | Record shared hash ownership, domain separation, and the sc-compose/consumer boundary in the proposed `docs/adrs/0018-sc-sha-hash-ownership.md`. | Proposed architecture decision; the next ADR number and scope require team-lead approval before M.1 implementation | M.1 |

Each sprint record must cite the applicable IDs in its acceptance criteria. A
requirement is not closed merely because a type or file exists; the cited test,
vector, or external-consumer evidence must be present.

## Goal

Create a standalone workspace crate named `sc-sha` that owns the canonical
file/hash identity primitives shared by sc-compose, atm-core, and
synaptic-canvas-dolt consumers. The crate must match synaptic-canvas-dolt in
both calculation and public API behavior, expose composition hashing over a
caller-supplied resolved manifest, and provide a maturin-built Python module. Then extend
sc-compose so its include expansion can produce a deterministic recursive
composition fingerprint covering the root template and every included
dependency.

The design has three distinct layers:

1. `sc-sha` hashes caller-supplied text and a caller-supplied, already resolved
   manifest. It does not discover dependencies, parse MiniJinja, understand
   `@<path>` syntax, or make filesystem-policy decisions.
2. `sc-composer` resolves its include graph, applies canonical-path and
   confinement policy, orders and deduplicates nodes, memoizes per-source
   hashes, and supplies the completed manifest to `sc-sha`.
3. `sc-sha-python` is a thin maturin/PyO3 adapter over the same Rust crate. It
   must not reimplement hashing or maintain a Python-only algorithm.

This separation prevents atm-core and other consumers from reimplementing the
algorithm while keeping filesystem and template-engine policy in sc-compose.

## Source-of-truth verification gate

Team-lead located the current synaptic-canvas-dolt implementation. Its only
currently verified sc-sha consumer contract is the per-file value stored in
`package_files.sha256`: plain SHA-256 over UTF-8 text, rendered as lowercase
64-character hexadecimal:

```python
content = full_path.read_text(encoding="utf-8")
sha256 = hashlib.sha256(content.encode("utf-8")).hexdigest()
```

Evidence locations:

- `synaptic-canvas-dolt/tools/dolt-ingest.py:285` (`_scan_file`)
- `synaptic-canvas-dolt/tools/dolt-export.py:321` (export verification)
- `synaptic-canvas-dolt/sql/001-create-tables.sql:51` (`package_files.sha256`)
- `synaptic-canvas-dolt/sql/001-create-tables.sql:81` (`cmd_sha256` exists as a
  future-verification column but is not populated by the ingest path)
- `synaptic-canvas-dolt/src/pkg/models/package.go:37,86,113` (model fields,
  including the currently unverified command field)
- `synaptic-canvas-dolt/src/pkg/dolt/queries.go:12,15,18` and
  `client.go:164,189,218`

There is no Noms/base32 content-addressing scheme in that repository and no
existing shared public Rust API to copy. Therefore the compatibility target is
the stored value and its consumable shape, not an invented Noms abstraction.

Before implementation begins, the plan must still record the exact
synaptic-canvas-dolt source commit and produce cross-language vectors for the
verified file-text domain:

- ordinary Unicode text;
- LF and CRLF files;
- BOM and no-final-newline inputs;
- empty text;
- invalid UTF-8 behavior at the file-reading boundary.

Command hashing is not a current compatibility target. The `cmd_sha256` column
exists in the schema, but the live ingest path does not populate or verify it;
any command-hash API is explicitly speculative and forward-looking until
synaptic-canvas-dolt defines a live contract.

The verification must cover both layers:

| Compatibility surface | Required proof |
| --- | --- |
| Calculation | Identical UTF-8 text produces the same digest bytes and lowercase hex as synaptic-canvas-dolt across the complete authoritative file-text vector set. |
| Rust public API | The `sc-sha` text-domain API exposes equivalent input, output, error, and encoding semantics, or a documented adapter with compile/test evidence. |
| Python public API | The maturin module exposes the same file-text digest values and stable string/bytes behavior; it does not claim that synaptic-canvas-dolt currently consumes a Python package. |
| Versioning | The crate/package version and algorithm identifier prevent silent cross-version collisions. |

The resolved file compatibility contract is:

1. Decode bytes as strict UTF-8.
2. Apply Python universal-newline behavior: `CRLF -> LF` and lone `CR -> LF`.
3. Preserve a decoded UTF-8 BOM as `U+FEFF`; do not use `utf-8-sig` semantics.
4. Preserve the presence or absence of the final newline.
5. Hash the normalized Unicode text re-encoded as UTF-8 with SHA-256.
6. Return a typed invalid-UTF-8 error rather than a digest for undecodable
   input.

This normalization is explicit and platform-invariant: macOS, Linux, and
Windows must produce the same digest for the same decoded text regardless of
the host filesystem's native line-ending convention. The Rust implementation
must not delegate this behavior to OS-specific file APIs.

This is a text-file contract, not an assertion that one character occupies one
byte. UTF-8 is the wire encoding and Unicode code points are the decoded text;
valid UTF-8 Markdown, log, and other text files remain fully representable,
including accented characters, CJK text, combining marks, and emoji/non-BMP
characters. Re-encoding the decoded text as UTF-8 preserves those code points.
The implementation must not truncate, reinterpret, or hash UTF-16 code units.
The compatibility vector set must include representative multilingual and
non-BMP text, as well as Markdown punctuation and log-style lines.

This cross-platform behavior is required because the digest is persisted in a
database and later compared with a file in a user's working copy. A logical
UTF-8 Markdown or log file must therefore retain the same database identity
when checked out or edited on Linux, macOS, or Windows. The database identity
is not the host-specific sequence of newline bytes. Files encoded in a legacy
non-UTF-8 encoding, or arbitrary binary files, are outside this text identity
contract and must fail with a typed error rather than receive a misleading
digest.

The verification artifact must be committed as documentation or test vectors in
the `sc-sha` implementation sprint. A plain SHA-256 implementation that merely
matches the empty-string vector is not evidence of compatibility. PR #358's
raw-byte implementation must be corrected or migrated to this normalized
`TemplateSha256` identity before it is used for persisted template identity; it
must not silently preserve a byte-oriented contract that differs from
synaptic-canvas-dolt.

ATM may retain the original source bytes separately for rendering/audit, but
the persisted template identity is the one normalized `TemplateSha256` value;
there is no second sc-sha raw-byte type.

### Cross-repository consumption proof

“Compatible” means synaptic-canvas-dolt can consume the file-text result without
a translation shim or algorithm-specific special case. The `sc-sha` sprint must
therefore produce a small interop fixture containing:

- input text;
- `sc-sha` Rust output;
- `sc-sha` Python/maturin file-text output;
- the expected lowercase 64-character value;
- the value inserted into a representative `package_files.sha256` record;
- synaptic-canvas-dolt ingest/export verification that accepts and reproduces
  the same value.

There is no equivalent current compatibility fixture for `cmd_sha256`, package
aggregate hashes, or a lockfile. Those are explicitly future proposals and
must not be represented as verified sc-sha consumers.

The public API review must confirm that consumers can obtain the same string
using supported public functions. Matching a digest while requiring callers to
reach into private bytes, re-normalize values, or reinterpret an error is not
an API-compatible result.

### External persistence boundary

`synaptic-canvas-dolt` owns persistence and installation state; `sc-sha` owns
calculation. The only current verified integration is the per-file
`package_files.sha256` field and its ingest/export comparison. The local
repository contains no verified `.synaptic/manifest.lock` writer or installer
implementation, and no package-aggregate SHA algorithm was found in the live
Dolt code. The earlier lockfile/package-aggregate discussion is therefore a
future proposal, not an integration contract or sprint deliverable.

Any future installer/lockfile or package aggregate must be a separately scoped
consumer change with its own schema and compatibility decision. It must not
make `sc-sha` own persistence, invent a `PackageSha256` compatibility claim, or
replace `package_files.sha256` with a recursive composition identity.

## Proposed workspace layout

Add one new core workspace member and one separate Python adapter member:

```text
crates/
  sc-sha/
    Cargo.toml
    src/
      lib.rs
      file.rs
      composition.rs
    tests/
      compatibility_vectors.rs
bindings/
  sc-sha-python/
    Cargo.toml
    pyproject.toml
    src/lib.rs
    tests/
      test_compatibility.py

boundaries/
  sc-sha/shared-library.toml
  sc-sha-python/python-adapter.toml
```

The exact module split may change during implementation, but the public crate
boundary must remain stable. `sc-sha` should be publishable independently of
the CLI and renderer crates so atm-core can depend on a released version rather
than a path into sc-compose.

Cargo dependency direction:

```text
bindings/python -> sc-composer -> sc-sha
sc-compose    -> sc-composer
atm-core      -> published sc-sha
synaptic-canvas-dolt -> published sc-sha or future compatibility implementation
sc-sha-python -> sc-sha
```

`sc-compose` must not add a direct `sc-sha` dependency because the repository's
boundary rules keep CLI behavior behind `sc-composer`. `sc-sha` must not depend
on either sc-composer or sc-compose. The existing `bindings/python` package
must not be broadened to become the SHA adapter; `sc-sha-python` is a separate
adapter whose only Rust product dependency is `sc-sha`.

The crate should have the smallest practical dependency surface. `sha2` may be
workspace-managed, but `sc-sha` should not depend on MiniJinja, serde, a
filesystem abstraction, ATM crates, or CLI libraries merely to compute a
digest. Canonical manifest encoding should be deterministic without relying on
map iteration or platform-specific serialization.

The `sc-boundary` inventory is part of the implementation, not merely review
documentation. M.1 must add `boundaries/sc-sha/shared-library.toml` with an
explicit allowlist containing only the approved hashing/framing dependencies
and permitted dependents (`sc-composer` and the future `sc-sha-python`
adapter). An unlisted dependency must fail the boundary check; this protects
against accidental dependencies on `sc-compose`, `sc-composer` in the wrong
direction, MiniJinja, filesystem/CLI libraries, ATM crates, PyO3, maturin, or
other runtime/tooling packages. M.2 must add the separate
`boundaries/sc-sha-python/python-adapter.toml`, allowing only published
`sc-sha` plus the approved PyO3 adapter dependencies. These records make the
shared-library-first design executable by the repository's existing sc-lint
boundary enforcement.

## Proposed `sc-sha` public API

`sc-sha` owns byte/text normalization, typed per-content identities, and
injective hashing of a completed manifest. It does not discover dependencies,
parse MiniJinja or `@<path>`, canonicalize filesystem paths, access a
filesystem, order/deduplicate nodes, detect cycles, or enforce depth/confinement
policy. Those are sc-compose resolver responsibilities.

The core has exactly two public operations:

```rust
pub fn calculate_hash(input: HashInput<'_>) -> Result<HashResult, ShaError>;

pub fn calculate_composition_hash(
    manifest: &ResolvedTemplateManifest,
) -> Result<CompositionSha256, CompositionError>;
```

`calculate_hash` is the per-content/identity operation. The second operation
accepts a caller-supplied manifest that is already resolved, ordered,
deduplicated by canonical source node, and checked by sc-compose. It never
accepts a resolver callback or performs graph discovery. A stream implementation
and a file adapter may share private code; the file adapter must delegate to
the stream path.

### Explicit hash domain

There is one current sc-sha template identity: strict UTF-8 text decoded from
caller-supplied file bytes, normalized with the verified universal-newline
policy, then hashed as UTF-8. ATM may retain original source bytes separately
for rendering or audit, but that is ATM storage behavior and is not a second
sc-sha identity.

```rust
pub struct TemplateSha256([u8; 32]);
pub struct CompositionSha256([u8; 32]);

pub enum HashInput<'a> {
    TextFileBytes { utf8_file_bytes: &'a [u8] },
}

pub enum HashResult {
    Template(TemplateSha256),
}
```

`TextFileBytes` strictly decodes UTF-8, applies the verified newline policy,
and returns the single `TemplateSha256` domain. It must work for Markdown,
logs, combining marks, CJK, emoji, and other valid Unicode text. The digest
types expose `as_bytes`, lowercase `to_hex`, and `Display` without requiring
private-field access. A private streaming helper and a caller-supplied byte
byte-slice adapter may share implementation; sc-sha does not open files.

JSON-variable serialization, renderer options, cache-context identity, and
whether ATM also stores original bytes are ATM concerns. They are not sc-sha
public input domains or requirements in this plan. There is no current
command-hash operation. A future `CommandSha256` domain may be added only
after synaptic-canvas-dolt makes its currently unused `cmd_sha256` column a
live, specified consumer contract.

### Resolved manifest contract

The graph is constructed by sc-compose, not by sc-sha. The manifest has unique
nodes but preserves ordered include occurrences as edges so repeated references
and edge order are not lost:

```rust
pub struct CanonicalTemplatePath(String);
pub struct CanonicalSourceUrl(String);

pub enum CanonicalSourceError {
    InvalidRepresentation,
}

impl TryFrom<String> for CanonicalTemplatePath {
    type Error = CanonicalSourceError;
    fn try_from(value: String) -> Result<Self, Self::Error>;
}

impl CanonicalTemplatePath {
    pub fn as_str(&self) -> &str;
}

impl TryFrom<String> for CanonicalSourceUrl {
    type Error = CanonicalSourceError;
    fn try_from(value: String) -> Result<Self, Self::Error>;
}

impl CanonicalSourceUrl {
    pub fn as_str(&self) -> &str;
}

pub enum ManifestSchemaVersion {
    V1,
}

pub enum CanonicalSource {
    LocalPath(CanonicalTemplatePath),
    Url(CanonicalSourceUrl),
}

pub struct ResolvedTemplateNode {
    pub source: CanonicalSource,
    pub content_hash: TemplateSha256,
}

pub struct ResolvedIncludeEdge {
    pub parent: CanonicalSource,
    pub child: CanonicalSource,
    pub occurrence: u32,
}

pub struct ResolvedTemplateManifest {
    pub schema: ManifestSchemaVersion,
    pub nodes: Vec<ResolvedTemplateNode>,
    pub edges: Vec<ResolvedIncludeEdge>,
}
```

`CanonicalTemplatePath` is an opaque newtype around a canonical path string;
`CanonicalSourceUrl` is an opaque newtype around a canonical URL string; and
`ManifestSchemaVersion` is an explicit version enum whose supported value is
`V1`. `CanonicalTemplatePath` and `CanonicalSourceUrl` are opaque tagged keys.
sc-compose applies filesystem canonicalization and confinement before calling
the `TryFrom<String>` constructors; the constructors validate only the
already-canonical representation and `as_str()` is the read-only encoding
accessor. sc-sha never decides whether a path exists or is allowed. The source
tag ensures a
future URL include cannot collide with or false-deduplicate against a local
path. The public fields remain caller-constructible for manifest assembly; no
validating manifest constructor is promised. `calculate_composition_hash` is
the sole public structural-validation gate: it rejects duplicate nodes,
unknown edge endpoints, unsupported schema versions, and malformed tagged
sources before producing a digest. It does not discover, reorder, deduplicate,
or cycle-check the graph; graph creation and resolver policy remain owned by
sc-compose.

The composition encoding is explicitly injective before SHA-256:

1. emit a fixed `sc-sha/manifest/v1` domain and schema tag;
2. encode node count, then each ordered node as source-kind tag, length-delimited
   source value, and length-delimited `TemplateSha256` bytes;
3. encode edge count, then each ordered edge as tagged parent, tagged child, and
   occurrence number with unambiguous length framing;
4. reject duplicate nodes, unknown edge endpoints, unsupported schema versions,
   and malformed tagged sources inside `calculate_composition_hash`, before
   hashing the framed bytes.

The encoding is required to be provably injective: different source tags,
paths/URLs, node order, per-file hashes, edge order, or occurrence structure
must produce different pre-hash bytes. Cycles are rejected by sc-compose before
manifest construction; sc-sha never detects them or returns a fingerprint for
an invalid manifest. A diamond dependency hashes a canonical source once in
sc-compose's memoized node builder, while both edges remain in the manifest.

### Hash-domain taxonomy

| Domain | API/result | Current authority and scope |
| --- | --- | --- |
| Normalized file text | `calculate_hash(TextFileBytes)` → `TemplateSha256` | **Verified current synaptic-canvas-dolt need** for `package_files.sha256`; also the single sc-sha template identity used by callers. |
| Resolved composition | `calculate_composition_hash(manifest)` → `CompositionSha256` | sc-sha framing over sc-compose-owned manifest; not a per-file replacement. |
| ATM cache context | ATM-owned composition/JSON/options framing around sc-sha results | ATM owns its graph-manifest model, JSON serialization, cache rows, and stale-cache rules; not a sc-sha domain in this plan. |
| Original source bytes | ATM-owned rendering/audit storage | May coexist with `TemplateSha256`, but is not a second sc-sha hash domain. |
| Command text | Future-only `CommandSha256` proposal | `package_deps.cmd_sha256` is currently unpopulated/unverified in synaptic-canvas-dolt. |
| Package aggregate | Future consumer proposal only | No aggregate algorithm was found in live synaptic-canvas-dolt; no `PackageSha256` compatibility claim. |
| Installer lockfile | Future consumer proposal only | No verified `.synaptic/manifest.lock` writer or installer exists in live synaptic-canvas-dolt. |

Every persisted field must name its domain and framing. No caller receives an
untyped `[u8; 32]` value that can be placed in a different field by accident.

### Error inventory

These stable codes are part of the public Rust/Python contract and must be
documented in the implementation sprint:

```rust
pub enum ShaError {
    InvalidUtf8,
}

pub enum CanonicalSourceError {
    InvalidRepresentation,
}

pub enum CompositionError {
    UnsupportedManifestSchema,
    DuplicateSource,
    UnknownEdgeEndpoint,
}
```

`CompositionError` is the public structural-manifest error type in this plan;
the earlier review shorthand `GraphError` is not a second public API. Resolver
and dependency-discovery failures remain sc-compose's `IncludeError` family,
as listed in the resolver inventory below. This keeps graph construction and
policy errors out of `sc-sha` while giving malformed caller-supplied manifests
an explicit typed result.

| Code | Error type | Cause | Recovery guidance |
| --- | --- | --- | --- |
| `SC_SHA_INVALID_UTF8` | `ShaError::InvalidUtf8` | Text bytes cannot be strictly decoded. | Fix the source encoding or provide a valid UTF-8 text input. |
| `SC_SHA_INVALID_CANONICAL_SOURCE` | `CanonicalSourceError::InvalidRepresentation` | A canonical path or URL is empty, contains control characters, or uses a backslash separator. | Re-canonicalize the source at the owning resolver boundary, then construct the tagged source again. |
| `SC_SHA_UNSUPPORTED_MANIFEST_SCHEMA` | `CompositionError::UnsupportedManifestSchema` | Manifest encoding version is unsupported. | Provide `ManifestSchemaVersion::V1` or upgrade the consumer. |
| `SC_SHA_DUPLICATE_SOURCE` | `CompositionError::DuplicateSource` | Caller supplied duplicate manifest nodes. | Deduplicate nodes in the sc-compose node builder before hashing. |
| `SC_SHA_UNKNOWN_EDGE_ENDPOINT` | `CompositionError::UnknownEdgeEndpoint` | An edge references no manifest node. | Include every edge endpoint in the resolved manifest node list. |

### Resolver error inventory

The following resolver failures are owned by sc-compose, not `sc-sha`. Their
stable codes and family mapping must be preserved in the M.2 implementation and
added to the architecture failure matrix where a new dynamic-include code is
needed:

| Stable code | Type/variant | Architecture family | Cause and recovery |
| --- | --- | --- | --- |
| `ERR_INCLUDE_NOT_FOUND` | `IncludeError` / `DiagnosticCode::ErrIncludeNotFound` | Existing `IncludeError` family | The include target is missing; correct the target or package contents. |
| `ERR_INCLUDE_CYCLE` | `IncludeError` / `DiagnosticCode::ErrIncludeCycle` | Existing `IncludeError` family | The include graph cycles; remove the cycle before requesting a cacheable manifest. |
| `ERR_INCLUDE_DEPTH` | `IncludeError` / `DiagnosticCode::ErrIncludeDepth` | Existing `IncludeError` family | The configured depth bound is exceeded; reduce nesting or raise the consumer policy explicitly. |
| `ERR_INCLUDE_ESCAPE` | `IncludeError` / `DiagnosticCode::ErrIncludeEscape` | Existing `IncludeError` family | A path or symlink escapes the allowed root; correct the include or confinement configuration. |
| `ERR_INCLUDE_DYNAMIC_UNRESOLVED` | `IncludeError` / new `DiagnosticCode::ErrIncludeDynamicUnresolved` | Existing `IncludeError` family; add this missing code to `docs/architecture.md` before implementation | A dynamic include cannot be exhaustively enumerated; resolve it statically or mark the result non-cacheable rather than hashing an incomplete graph. |

### API simplicity requirement

The core and maturin adapter expose exactly `calculate_hash` and
`calculate_composition_hash`. There is no resolver trait, filesystem callback,
path-policy helper, graph-discovery operation, generic `recursive: bool`, or
optional context field. The manifest is caller-supplied and the ownership
boundary is testable by compiling sc-sha without sc-composer, MiniJinja, or
filesystem dependencies.

### atm-core consumer contract (no atm-core implementation here)

Recursive hashing is a required atm-core consumer use case, not an optional
sc-compose feature. atm-core must be able to restore the correct rendered
context from its persisted JSON variables plus the current recursive template
identity. A per-file hash is insufficient because changing an included
template must invalidate a cached root even when the root file itself is
unchanged.

The sc-sha API provides the normalized template and composition values needed by
atm-core:

```rust
pub struct TemplateSha256([u8; 32]);
pub struct CompositionSha256([u8; 32]);
```

The recursive atm-core consumer obtains or builds its own resolved manifest,
calls `calculate_composition_hash`, and combines that result with its own JSON
variable serialization and cache-row identity rules. No sc-sha resolver
callback, JSON canonicalizer, renderer-option model, or context-row schema is
involved. ATM may use the same `calculate_hash` operation for its own
caller-supplied serialized values, but those serialization rules remain an ATM
contract rather than a sc-sha requirement.

The atm-core consumer must recompute its resolved manifest and combine the
composition result with its own persisted JSON/cache identity on restore; it
must not trust an isolated per-file SHA or reconstruct context from the root
file alone. Its database schema, graph-manifest model, JSON serialization, and
stale-cache rules are outside this sc-compose plan. The consumer contract is
external acceptance evidence for `sc-sha`, not an atm-core implementation
sprint here.

### Maturin/Python API

Keep PyO3 and maturin dependencies out of `sc-sha` itself. The adapter package
should expose a stable Python module named `sc_sha` and delegate every
calculation to Rust:

```python
from sc_sha import calculate_hash, calculate_composition_hash

calculate_hash({"kind": "text-file", "content": "template text"})
# -> tagged TemplateSha256 result with lowercase hexadecimal value

manifest = {
    "schema": "sc-sha/manifest/v1",
    "nodes": [
        {"source": {"kind": "local-path", "value": "root.md"},
         "content_hash": "..."},
        {"source": {"kind": "local-path", "value": "partial.md"},
         "content_hash": "..."},
    ],
    "edges": [
        {"parent": "root.md", "child": "partial.md", "occurrence": 0},
    ],
}
calculate_composition_hash(manifest)
# -> CompositionSha256; graph construction remains with the caller
```

The final Python names and result mapping must be reconciled with the verified
sc-sha API and synaptic-canvas-dolt's file-field consumption shape; the Dolt
repository has no existing shared Rust or Python API to copy. The adapter must
define:

- accepted Python types for UTF-8 text bytes and tagged manifest sources;
- lowercase hex and explicitly documented text/bytes input-output behavior;
- typed Python exceptions for invalid domains and malformed manifests;
- deterministic dictionary/list serialization for the caller-supplied manifest;
- minimum supported Python versions and platform wheels;
- package/module version alignment with the Rust algorithm version;
- a clean-consumer `maturin develop` and wheel-install test.

The Python API must expose the same two operations as the Rust API. It must not
discover dependencies or accept an unordered set as the only representation of
the manifest. The caller's ordered nodes and edges remain authoritative.

## What moves from PR #358

The implementation currently proposed in PR #358 moves into `sc-sha` with
explicit domain names:

- `TemplateSha256` storage and trait implementations for the verified
  synaptic-canvas-dolt text behavior;
- `as_bytes()`;
- `to_hex()`;
- `Display` hexadecimal formatting;
- the stream-based normalization and any explicitly retained UTF-8 migration
  facade; ATM-owned original source bytes remain outside sc-sha;
- text golden vectors and line-ending/BOM/final-newline tests, updated to the
  verified synaptic-canvas-dolt vectors.

The `sha2` dependency moves with the implementation. `sc-composer/src/lib.rs`
stops declaring or implementing the hash directly and re-exports the shared
types/functions only if its public API needs to remain source-compatible.
It must re-export only the single normalized `TemplateSha256` identity when
source compatibility requires it; ATM's separately retained original bytes are
not a second sc-sha type. The PR #358 follow-up must document that the
persisted template identity is LF-normalized.

The following PR #358 features stay in sc-composer:

- MiniJinja directive-span inspection;
- confined MiniJinja loading;
- renderer behavior and renderer-specific tests.

Those features are not hashing responsibilities. Their dependency inspection
must not be presented as a composition fingerprint until it is connected to a
real graph and the graph is hashed through `sc-sha`.

## Sequence recommendation

Choose **land `sc-sha` first, then update PR #358 to consume it**.

Rationale:

1. The standalone crate is the compatibility boundary needed by atm-core and
   future synaptic-canvas-dolt consumers.
2. Merging the current local hash first would create a duplicate algorithm and
   make later extraction a compatibility-sensitive rewrite.
3. Comp2 can fix and QA the current PR's renderer/directive behavior now, so
   this sequencing does not require waiting for CI work to finish.
4. Once `sc-sha` is available, PR #358 can be rebased or amended to remove its
   local `template_hash.rs`, depend on `sc-sha`, and retain its existing public
   compatibility surface through a re-export if needed.
5. M.2 combines the small sc-compose integration and Python adapter after
   the shared API is stable, avoiding unnecessary worktrees and coordination
   overhead.
6. Release acceptance requires atm-core to prove recursive cache restoration
   from JSON variables plus composition SHA; atm-core builds the resolved
   manifest and owns its cache schema. This is consumer evidence, not an
   atm-core implementation sprint in this plan.

The merge gate is therefore:

```text
plan approved
  -> verify synaptic-canvas-dolt algorithm
  -> sc-sha core implementation + QA
  -> sc-compose integration + Python adapter + QA
  -> PR #358 updated to consume sc-sha + QA
  -> external consumer acceptance/release gate
```

Comp2's current PR work and QA may proceed concurrently with the first two
planning/implementation activities, but a green CI result on the current PR
does not authorize merging a duplicate hash implementation.

The PR #358 follow-up is a post-M.2 phase gate, not an M.2 closure criterion.
M.2 may be marked complete once its own implementation, QA, merge, and
revalidation evidence is complete. The follow-up must still be rebased or
amended to consume `sc-sha`, receive full CI and quality-mgr QA, and land before
Phase M closes; this separation prevents an open external PR from making the
M.2 sprint's own status self-contradictory.

## External consumer acceptance and phase QA
### atm-core cache-consumer tests

The atm-core integration must additionally prove:

1. the same recursively resolved manifest restores the same context when ATM's
   own JSON-variable/cache identity is unchanged;
2. changing a nested template invalidates the cache even when the root file is
   unchanged;
3. changing ATM-owned JSON variables or renderer options invalidates the cache
   even when the manifest is unchanged;
4. changing dependency order or occurrence count invalidates the cache when
   those inputs affect rendering;
5. missing/cyclic/depth-exhausted graphs never restore a prior context;
6. cache rows retain and validate the sc-sha per-file/composition values rather
   than trusting a bare or mismatched SHA string. These tests run in atm-core;
   they are external acceptance evidence, not sc-compose implementation.

### Phase-level QA aggregation

The two sprint records above own their deliverables, acceptance criteria, and
required validation; this section intentionally does not restate those lists.
At phase close, quality-mgr aggregates the two QA approvals and verifies the
following cross-sprint gates:

- M.1 and M.2 each have a merged commit, an independent QA result, and
  post-merge revalidation on `integrate/phase-M`.
- The final integration branch has no duplicate SHA implementation in
  sc-composer and PR #358's renderer/directive CI fixes remain isolated from
  the hash-contract migration until its follow-up is explicitly reviewed.
- The external atm-core and synaptic-canvas-dolt acceptance artifacts named in
  this plan exist before issue #360 is marked complete.

## sc-lint cleanup and QA routing

Each implementation sprint must run the applicable sc-lint checks on its final
commit. Minor findings are fixed immediately in the sprint worktree. Remaining
findings are grouped by independent rule class and ownership boundary, not by
individual occurrence:

- create a dedicated `fix/` worktree from the sprint's final commit;
- keep constant/string findings grouped by owning crate;
- keep length-driven refactors separate from semantic or boundary changes;
- send the worktree path, parent commit, finding evidence, tests, and fix
  commit to team-lead;
- team-lead creates the PR and sends it to quality-mgr for independent QA;
- the implementation sprint does not close until required fix PRs are QA
  approved, merged, and revalidated.

The Python adapter sprint applies the same routing to Rust, PyO3, Python, and
packaging findings, while keeping wheel/build failures separate from core hash
semantic fixes.

## Boundary Rules compliance

This plan complies with the repository's `CLAUDE.md` Boundary Rules:

### Rule 1 — `sc-composer` remains a pure library

`sc-sha` is a pure computation crate. It performs no CLI parsing, process
spawning, ATM calls, or filesystem traversal. `sc-composer` remains a library;
it only supplies graph data and verified source text to `sc-sha`.

### Rule 2 — `sc-compose` dependency direction remains intact

The CLI continues to depend on `sc-composer`; it does not directly depend on
`sc-sha`. The new functionality is surfaced through the library boundary, so
the CLI does not become a second hash owner or bypass the composition layer.

### Rule 5 — `sc-composer` does not depend on Python bindings

The dependency graph remains `bindings/python -> sc-composer -> sc-sha`.
`sc-sha` has no Python or binding dependency, and no Python-facing code is
needed to compute a fingerprint.

### Rule 6 — no `ATM_HOME` access

Neither `sc-sha` nor the sc-composer integration reads `ATM_HOME` or any ATM
runtime path. ATM-core consumes the published crate through its own adapter
boundary, outside this repository.

### Proposed Rule 8 — `sc-sha` core dependency boundary

Sprint M.1 must amend `CLAUDE.md` to make this boundary explicit: `sc-sha` may
depend only on the approved hashing/encoding implementation dependencies. It
must not depend on `sc-compose`, `sc-composer`, MiniJinja, filesystem/CLI
libraries, ATM crates, PyO3, or maturin, and it must not implement resolver,
path-policy, cycle, or depth behavior.

The same sprint must add `boundaries/sc-sha/shared-library.toml` and prove it
with `just lint sc-boundary` plus a deliberate forbidden-dependency fixture.
The boundary record is the machine-checked allowlist; `CLAUDE.md` and
ADR-SHA-001 explain its rationale and ownership.

### Proposed Rule 9 — `sc-sha-python` adapter boundary

Sprint M.1 must also amend `CLAUDE.md` to state that
`bindings/sc-sha-python` may depend on published `sc-sha` plus PyO3/maturin
packaging dependencies only. It must not depend on `sc-compose`,
`sc-composer`, ATM-specific crates, or read `ATM_HOME`; it delegates both
public operations without a Python-only algorithm. These proposed rules are
not effective until team-lead explicitly rules on them and signs off
`ADR-SHA-001`.

M.2 must add the corresponding `boundaries/sc-sha-python/python-adapter.toml`
record and prove that the adapter cannot acquire `sc-compose`,
`sc-composer`, ATM, or unrelated runtime dependencies through the same
`sc-boundary` command.

### Proposed ADR status

`ADR-SHA-001` is a plan traceability identifier, not an accepted ADR. The
repository currently ends at ADR-0017;
`docs/adrs/0018-sc-sha-hash-ownership.md` is only the proposed next filename
and does not exist yet. Team-lead must approve the number and scope, then M.1
may author the ADR and obtain sign-off before any `sc-sha` or
`sc-sha-python` implementation source is authored or staged. If the ADR index
assigns a different number, all plan references must be updated before the
implementation gate opens.

Additional boundary checks:

- No `agent-team-mail-*`, `atm_*`, or `agent_team_mail` dependency is added.
- No `use agent_team_mail::` or `use atm_*::` import is introduced.
- No filesystem traversal, MiniJinja, or CLI dependency is added to `sc-sha`.
- No resolver callback, canonical-path policy, cycle detector, depth limiter,
  or filesystem access is added to `sc-sha`; these remain sc-compose-owned.
- PyO3/maturin dependencies are confined to `bindings/sc-sha-python`; they are
  not optional dependencies of the core crate.
- The new adapter is not silently folded into `bindings/python`, whose existing
  dependency contract remains `bindings/python -> sc-composer` only.
- The shared crate may be used by external consumers only through its published
  versioned API and compatibility vectors.

## Phase-level gates and external handoff

### Before implementation

- [ ] Team-lead records the synaptic-canvas-dolt source commit and algorithm.
- [ ] Compatibility vectors are copied into the plan review evidence.
- [ ] The verified synaptic-canvas-dolt file-field consumption shape is mapped
      to `sc-sha` symbols and tested, not merely compared by digest output; no
      nonexistent Dolt Rust/Python API is treated as an authority.
- [ ] Team-lead confirms Phase M's parent integration commit and the two
      frontmatter branch/worktree records before implementation begins.
- [ ] Confirm whether `sc-sha` is published from this workspace or extracted
      to its own repository while preserving the same crate name and API.

### Phase-level gates

- [ ] Before M.1 starts, team-lead records the synaptic-canvas-dolt source
      commit, authoritative vectors, Phase M parent commit, and final sprint
      filenames/branches.
- [ ] After M.1 merges, its QA evidence and the `sc-sha` publication/API
      decision are attached to the integration branch before M.2 starts.
- [ ] **Post-M.2 PR #358 gate:** after M.2 merges, the PR #358 follow-up is
      rebased/amended to consume `sc-sha`; its directive-span and
      confined-loader scope remain separate, then its full CI and quality-mgr
      QA are rerun before Phase M closes. This is not an M.2 closure
      criterion.
- [ ] At phase close, the integration branch has the two QA-approved sprint
      commits, all routed sc-lint fix worktrees merged/revalidated, and the
      external acceptance evidence below.
- [ ] **QM-010 release-artifact gate (explicitly deferred from M.2):** the
      `team-lead`/release maintainers must register both `sc-sha` and
      `sc-sha-python` in `release/publish-artifacts.toml` before any release
      containing either artifact. That registration must include matching
      crate/package publish entries, all-platform wheel build/install/test
      coverage in `.github/workflows/release.yml`, and a passing release
      preflight. M.2 does not publish these artifacts and does not change the
      release workflow; the release registration is a named post-M.2 gate,
      not an untracked omission.

### External consumer acceptance (no consumer implementation here)

- [ ] Provide atm-core with the published two-operation API and compatibility
      vectors for recursive cache restoration.
- [ ] Record evidence that atm-core can recompute recursive and JSON-variable
      identities before restoring context.
- [ ] Provide synaptic-canvas-dolt with the per-file compatibility vectors and
      public API mapping for `package_files.sha256` consumption. Any future
      database/lockfile integration is separately scoped and not claimed here.

## Explicit non-goals

- Do not merge a second independent implementation merely because it produces
  ordinary SHA-256 values.
- Do not use an unordered array of child hashes as the root identity.
- Do not make atm-core depend on a path inside the sc-compose repository.
- Do not make sc-sha aware of MiniJinja syntax or `@<path>` syntax.
- Do not put PyO3/maturin into the core `sc-sha` dependency graph.
- Do not include absolute paths, mtimes, host-specific separators, or
  incidental resolver traversal order in the identity; sc-compose/atm-core
  must provide the deterministic ordered manifest consumed by sc-sha.
- Do not claim PR #358 or this planning document closes issue #360 by itself.
