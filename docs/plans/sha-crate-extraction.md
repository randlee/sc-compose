# Plan: `sc-sha` Extraction and Recursive Template Composition Fingerprints

## Plan status

- Type: planning/design document
- Planning branch: `plan/sha-crate`
- Planning worktree: `../sc-compose-worktrees/plan/sha-crate`
- Implementation target: `develop`, followed by the normal `integrate/phase-*`
  sprint flow
- Current related work: [PR #358](https://github.com/randlee/sc-compose/pull/358)
- Related product issue: [sc-compose #360](https://github.com/randlee/sc-compose/issues/360)
- External algorithm authority: synaptic-canvas-dolt, verified at the source
  locations listed below; the exact source commit still must be recorded

This document is intentionally planning-only. It does not add a Cargo member,
move `template_hash.rs`, alter PR #358, or implement recursive hashing.

## Goal

Create a standalone workspace crate named `sc-sha` that owns the canonical
file/hash identity primitives shared by sc-compose, atm-core, and
synaptic-canvas-dolt consumers. The crate must match synaptic-canvas-dolt in
both calculation and public API behavior, expose recursive composition hashing
with cycle protection, and provide a maturin-built Python module. Then extend
sc-compose so its include expansion can produce a deterministic recursive
composition fingerprint covering the root template and every included
dependency.

The design has two distinct layers:

1. `sc-sha` hashes caller-supplied text and walks a caller-supplied dependency
   source resolver. It does not parse MiniJinja, understand `@<path>` syntax,
   or make filesystem-policy decisions.
2. `sc-composer` adapts its include resolver to the `sc-sha` resolver contract,
   supplies canonical paths and source text under the verified text policy, and
   exposes the resulting fingerprint to its consumers.
3. `sc-sha-python` is a thin maturin/PyO3 adapter over the same Rust crate. It
   must not reimplement hashing or maintain a Python-only algorithm.

This separation prevents atm-core and other consumers from reimplementing the
algorithm while keeping filesystem and template-engine policy in sc-compose.

## Source-of-truth verification gate

Team-lead located the current synaptic-canvas-dolt implementation. The
reference is plain SHA-256 over UTF-8 text, rendered as lowercase 64-character
hexadecimal:

```python
content = full_path.read_text(encoding="utf-8")
sha256 = hashlib.sha256(content.encode("utf-8")).hexdigest()
```

Evidence locations:

- `synaptic-canvas-dolt/tools/dolt-ingest.py:285` (`_scan_file`)
- `synaptic-canvas-dolt/tools/dolt-export.py:321` (export verification)
- `synaptic-canvas-dolt/sql/001-create-tables.sql:51,81` (`sha256` and
  `cmd_sha256` are `VARCHAR(64)`)
- `synaptic-canvas-dolt/src/pkg/models/package.go:37,86,113`
- `synaptic-canvas-dolt/src/pkg/dolt/queries.go:12,15,18` and
  `client.go:164,189,218`

There is no Noms/base32 content-addressing scheme in that repository and no
existing shared public Rust API to copy. Therefore the compatibility target is
the stored value and its consumable shape, not an invented Noms abstraction.

Before implementation begins, the plan must still record the exact
synaptic-canvas-dolt source commit and produce cross-language vectors for:

- ordinary Unicode text;
- LF and CRLF files;
- BOM and no-final-newline inputs;
- empty text;
- invalid UTF-8 behavior at the file-reading boundary;
- command strings used for `cmd_sha256`.

The verification must cover both layers:

| Compatibility surface | Required proof |
| --- | --- |
| Calculation | Identical UTF-8 text produces the same digest bytes and lowercase hex as synaptic-canvas-dolt across the complete authoritative vector set. |
| Rust public API | The `sc-sha` API exposes equivalent input, output, error, and encoding semantics, or a documented adapter with compile/test evidence. |
| Python public API | The maturin module exposes the same digest values and stable string/bytes behavior needed by synaptic-canvas-dolt Python consumers. |
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
raw-byte function must be changed to apply this contract before it is used as
the synaptic-canvas-dolt-compatible file identity.

If the verified text behavior differs from PR #358's current
`template_sha256(raw_file_bytes)` behavior, the synaptic-canvas-dolt contract
wins. PR #358 must consume `sc-sha` rather than preserve a competing local
implementation.

### Cross-repository consumption proof

“Compatible” means synaptic-canvas-dolt can consume the result without a
translation shim or algorithm-specific special case. The `sc-sha` sprint must
therefore produce a small interop fixture containing:

- input text;
- `sc-sha` Rust output;
- `sc-sha` Python/maturin output;
- the expected lowercase 64-character value;
- the value inserted into a representative `package_files.sha256` or
  `package_deps.cmd_sha256` record;
- synaptic-canvas-dolt ingest/export verification that accepts and reproduces
  the same value.

The public API review must confirm that consumers can obtain the same string
using supported public functions. Matching a digest while requiring callers to
reach into private bytes, re-normalize values, or reinterpret an error is not
an API-compatible result.

### Online-install integration boundary

`synaptic-canvas-dolt` owns persistence and installation state; `sc-sha` owns
the calculation. The sc-sha plan must not add a lockfile writer or a Synaptic
Canvas runtime dependency to sc-compose. The integration contract is:

- Dolt `package_files.sha256` stores the lowercase hexadecimal
  `TemplateSha256` content identity for the source text ingested into the
  package database.
- After install-time template rendering, the Synaptic Canvas installer stores
  the resulting per-file `TemplateSha256` values in
  `{repo-root}/.synaptic/manifest.lock` under `[skills.files]`, keyed by the
  materialized relative path.
- The lockfile also records package version/channel and the Dolt commit needed
  to identify the package revision; those are release/install metadata, not
  part of the per-file digest.
- `OrderedTemplateHashes` and `CompositionSha256` are sc-compose
  recursive-source identities. They must not replace the per-file value in
  `package_files.sha256` or `[skills.files]` unless Synaptic Canvas separately
  versions its schema for that purpose.

The online integration test is an end-to-end release gate owned jointly with
synaptic-canvas-dolt: ingest a UTF-8 Markdown/log fixture, fetch it from the
database, materialize it into a temporary project, write the lockfile through
the installer, and verify the local file against the recorded hash. Repeat
with CRLF and LF representations on the supported OS matrix, rendered
templates, Unicode/non-BMP content, modified files, and missing files. This
proves that the published crate/API can support the actual online workflow
without requiring a local algorithm fork.

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
```

The exact module split may change during implementation, but the public crate
boundary must remain stable. `sc-sha` should be publishable independently of
the CLI and renderer crates so atm-core can depend on a released version rather
than a path into sc-compose.

Cargo dependency direction:

```text
sc-compose-py -> sc-composer -> sc-sha
sc-compose    -> sc-composer
atm-core      -> published sc-sha
synaptic-canvas-dolt -> published sc-sha or compatibility implementation
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

## Proposed `sc-sha` public API

The API must preserve the digest and output behavior consumed by
synaptic-canvas-dolt and atm-core. PR #358's current raw-byte API is a
candidate surface, not the authority; it must be corrected if its behavior
diverges from the verified text contract.

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateSha256([u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CommandSha256([u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct JsonVarsSha256([u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateContextSha256([u8; 32]);

pub enum HashInput<'a> {
    FileText { utf8_file_bytes: &'a [u8] },
    CommandText { command: &'a str },
    JsonVariables { canonical_json: &'a [u8] },
    GraphContext {
        graph: &'a OrderedTemplateHashes,
        canonical_json: Option<&'a [u8]>,
    },
}

pub enum HashResult {
    File(TemplateSha256),
    Command(CommandSha256),
    JsonVariables(JsonVarsSha256),
    Composition(CompositionSha256),
    Context(TemplateContextSha256),
}

/// The single public hash operation. The tagged input and result prevent
/// callers from confusing file, command, JSON-variable, and graph identities.
pub fn calculate_hash(input: HashInput<'_>) -> Result<HashResult, ShaError>;

/// The single public graph operation. It discovers dependencies, validates
/// paths/cycles/depth, and returns the deterministic ordered graph consumed by
/// `HashInput::GraphContext`.
pub fn calculate_graph_and_order<S: DependencySource>(
    root_path: &str,
    source: &mut S,
) -> Result<OrderedTemplateHashes, GraphError<S::Error>>;
```

`HashInput::FileText` strictly decodes UTF-8, applies the verified newline
policy, and returns `HashResult::File`. `HashInput::CommandText` hashes command
text directly for `cmd_sha256`; `HashInput::JsonVariables` hashes canonical
JSON variables; and `HashInput::GraphContext` hashes the completed graph and,
when present, derives the rendered-context identity from graph plus variables.
None of these variants accepts arbitrary binary content as a
synaptic-canvas-dolt file identity. If a raw-byte digest is useful for another
consumer, it must be a separately named, explicitly non-compatible domain.

The digest type retains the currently proposed `as_bytes`, `to_hex`, and
lowercase `Display` surface. Add compile-level/API-shape examples showing that
a consumer can obtain the stored-compatible string without reaching into
private fields.

`TemplateSha256` remains the content-only identity consumed by
synaptic-canvas-dolt. The graph operation returns a deduplicated, ordered list
of path/content identities rather than a second per-node hash type:

```rust
pub struct TemplateHashEntry {
    pub canonical_path: String,
    pub file_sha: TemplateSha256,
}

pub struct OrderedTemplateHashes {
    /// Root entry first; each canonical path appears at most once.
    pub entries: Vec<TemplateHashEntry>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompositionSha256([u8; 32]);

impl CompositionSha256 {
    pub const fn as_bytes(&self) -> &[u8; 32];
    pub fn to_hex(self) -> String;
}

```

The list is ordered by deterministic first discovery from the root, while
deduplication is by canonical path. Repeated references to an already-seen
path do not create another list entry because the referring template's own
content hash already captures its include syntax, condition, and occurrence.
Consumers that need one aggregate identity may pass the list to
`HashInput::GraphContext` with explicit parent/root framing; consumers that
need per-file identities may retain the list directly.

### Hash-domain taxonomy

The algorithm primitive is SHA-256, but these identities are not
interchangeable because their inputs, normalization, framing, and owners
differ:

| Domain | Proposed identity/API | Owner and storage |
| --- | --- | --- |
| File content | `HashInput::FileText` → `HashResult::File(TemplateSha256)` | `sc-sha`; consumed by synaptic-canvas-dolt as `package_files.sha256` and installer lockfile file entries |
| Command text | `HashInput::CommandText` → `HashResult::Command(CommandSha256)` | `sc-sha` calculation; consumed by synaptic-canvas-dolt as `package_deps.cmd_sha256` |
| JSON variables | `HashInput::JsonVariables` → `HashResult::JsonVariables(JsonVarsSha256)` | `sc-sha` calculation; consumed by atm-core cache rows |
| Recursive template list | `OrderedTemplateHashes` of `(canonical path, TemplateSha256)` entries | `sc-sha` graph operation; transient input to aggregate/context hashing |
| Recursive composition | `CompositionSha256` | `sc-sha` calculation; consumed by sc-compose composition metadata, not a replacement for per-file hashes |
| Rendered context | `HashInput::GraphContext` → `HashResult::Context(TemplateContextSha256)` | `sc-sha` calculation from composition plus JSON-variable identities; consumed by atm-core cache restoration |
| Package aggregate | `PackageSha256` or equivalent | synaptic-canvas-dolt owns sorted path-plus-file-hash framing and package/lockfile schema |
| Release/artifact checksum | release-specific SHA-256 value | publishing/distribution tooling owns the artifact bytes and checksum metadata |

Every persisted field must document its domain and framing. The API must not
return one untyped `[u8; 32]` value that callers can accidentally place in a
different field. Package aggregation and release checksums may reuse the
primitive internally, but they must not silently reuse the file-text or
recursive-composition contract.

#### API simplicity requirement

The core has exactly two public operations: `calculate_hash` and
`calculate_graph_and_order`. There is no generic boolean such as
`hash(input, recursive: bool)`, no parallel convenience function family, and
no bare digest result whose domain the caller must infer. The tagged
`HashInput`/`HashResult` pair is the single justified mechanism for keeping the
supported hash domains explicit. The same two-operation shape applies to the
maturin adapter.

The hash implementation may have a private/generic streaming core and a file
adapter layered over it. The file adapter opens the source, applies the same
explicit UTF-8/newline policy, and delegates to the stream implementation; it
must not duplicate hashing or normalization logic. This is an implementation
seam, not a third public hash operation. Filesystem traversal and dependency
ordering remain the responsibility of `calculate_graph_and_order`'s injected
resolver.

The final field names and encoding remain subject to the synaptic-canvas-dolt
verification gate. The plan requires the following invariants regardless of
the final spelling:

- canonical relative paths only; no absolute machine-specific paths;
- normalized `/` separators;
- deterministic length-delimited or equivalently unambiguous encoding;
- explicit algorithm/version domain separation;
- exact UTF-8 text identity matching the upstream reader, with the explicit
  newline normalization contract above;
- nested and transitive dependencies included recursively;
- include order and repeated occurrences remain represented by the referring
  template's content hash; the returned path/hash list deduplicates paths;
- render-option identity included only when it affects output;
- no mtime, incidental resolver traversal order, locale, or host-path
  dependence; only the defined deterministic candidate-discovery order may be
  represented;
- cycles and missing dependencies fail before producing a success fingerprint.

The ordered path/hash list is diagnostic evidence. Consumers may compare a
single `CompositionSha256` when they need an aggregate, but they must not use
an unordered array of child hashes as the identity.

### Recursive resolver API

The recursive computation belongs in `sc-sha`, but file discovery remains with
the caller. Use a resolver trait or equivalent generic callback so the core
crate can enforce recursion and cycle invariants without reading a filesystem:

```rust
pub struct TemplateSource {
    pub canonical_path: String,
    pub source_text: String,
    pub dependencies: Vec<String>,
}

pub trait DependencySource {
    type Error;

    fn load(&mut self, canonical_path: &str)
        -> Result<TemplateSource, Self::Error>;
}

pub fn calculate_graph_and_order<S: DependencySource>(
    root_path: &str,
    source: &mut S,
) -> Result<OrderedTemplateHashes, GraphError<S::Error>>;
```

The exact ownership of `TemplateSource` and the error wrapper may change after
the synaptic-canvas-dolt API is verified, but the behavior is mandatory:

- maintain an active recursion stack and reject a path already on that stack;
- return a typed cycle error containing the deterministic cycle path;
- enforce a configurable depth ceiling before recursion can overflow the stack;
- cache completed path identities without dropping statically discoverable
  candidate templates;
- load dependencies in declared/source order;
- reject malformed canonical paths before hashing;
- never return a successful root fingerprint for a missing, cyclic, or failed
  dependency.

The resolver must be deliberately generic. A filesystem-backed resolver can be
implemented by sc-composer; an in-memory resolver can be used by atm-core,
Python, and deterministic tests. This keeps `sc-sha` reusable without hiding
filesystem security policy inside a hash crate.

The graph operation returns the ordered path/hash list. The hash operation
returns the tagged `CompositionSha256` or `TemplateContextSha256` result;
callers do not rebuild ordering from rendered text.

### atm-core consumer contract (no atm-core implementation here)

Recursive hashing is a required atm-core consumer use case, not an optional
sc-compose feature. atm-core must be able to restore the correct rendered
context from its persisted JSON variables plus the current recursive template
identity. A per-file hash is insufficient because changing an included
template must invalidate a cached root even when the root file itself is
unchanged.

The sc-sha API must provide three separately typed values:

```rust
pub struct JsonVarsSha256([u8; 32]);
pub struct TemplateContextSha256([u8; 32]);
```

The recursive atm-core operation must obtain the ordered graph through
`calculate_graph_and_order`, then call `calculate_hash` with
`HashInput::JsonVariables` and `HashInput::GraphContext`. The canonical JSON
contract must define object-key
ordering, number representation, Unicode escaping, duplicate-key rejection,
and whitespace handling; semantically identical JSON objects must produce the
same `JsonVarsSha256` regardless of input formatting or map iteration order.
The API may accept already canonical JSON bytes to keep `sc-sha` independent of
a particular JSON crate, but canonicalization must be one shared, tested
contract rather than an atm-core-only convention.

The atm-core consumer must recompute the graph and JSON-variable identity on
restore and reject stale/mismatched cache state; it must not trust an isolated
SHA or reconstruct context from the root file alone. Its database schema and
cache-row storage are outside this sc-compose plan. The consumer contract is a
release-blocking acceptance test for `sc-sha`, not an atm-core implementation
sprint here.

### Maturin/Python API

Keep PyO3 and maturin dependencies out of `sc-sha` itself. The adapter package
should expose a stable Python module named `sc_sha` and delegate every
calculation to Rust:

```python
from sc_sha import calculate_hash, calculate_graph_and_order

calculate_hash({"kind": "file", "content": "template text"})
# -> tagged file result with lowercase hexadecimal value

graph = calculate_graph_and_order(
    root_path="root.md",
    templates={
        "root.md": {"text": "@<partial.md>\n", "dependencies": ["partial.md"]},
        "partial.md": {"text": "body\n", "dependencies": []},
    },
)
calculate_hash({"kind": "graph-context", "graph": graph,
                "json_variables": {}})
# -> tagged composition/context result with graph evidence
```

The final Python names and result mapping must be reconciled with the verified
synaptic-canvas-dolt public API. The adapter must define:

- accepted Python types for bytes, paths, dependency records, and optional
  render options;
- lowercase hex and explicitly documented text/bytes input-output behavior;
- typed Python exceptions for invalid paths, missing nodes, cycles, and depth
  exhaustion;
- deterministic dictionary/list serialization for path/hash evidence;
- minimum supported Python versions and platform wheels;
- package/module version alignment with the Rust algorithm version;
- a clean-consumer `maturin develop` and wheel-install test.

The Python API must expose the same two operations as the Rust API. It must not
accept an unordered set as the only representation of dependencies. A mapping
may be used for node lookup, but each node's ordered dependency list remains
authoritative.

## What moves from PR #358

The implementation currently proposed in
`crates/sc-composer/src/template_hash.rs` moves into `sc-sha`:

- `TemplateSha256` storage and trait implementations;
- `as_bytes()`;
- `to_hex()`;
- `Display` hexadecimal formatting;
- the text SHA function and any explicitly retained UTF-8 migration facade;
- text golden vectors and line-ending/BOM/final-newline tests, updated to the
  verified synaptic-canvas-dolt vectors.

The `sha2` dependency moves with the implementation. `sc-composer/src/lib.rs`
stops declaring or implementing the hash directly and re-exports the shared
types/functions only if its public API needs to remain source-compatible.

The following PR #358 features stay in sc-composer:

- MiniJinja directive-span inspection;
- confined MiniJinja loading;
- renderer behavior and renderer-specific tests.

Those features are not hashing responsibilities. Their dependency inspection
must not be presented as a composition fingerprint until it is connected to a
real graph and the graph is hashed through `sc-sha`.

## Recursive sc-compose integration

The existing include engine is the correct ownership point for recursive
composition discovery. It already resolves `@<path>` includes, enforces root
confinement, detects cycles/depth, and records resolved files and source text.
However, its first-seen file collection cannot represent the full identity:

- the public model does not expose a deterministic ordered list of unique
  `(canonical path, file hash)` entries;
- source text is retained as `String`, while the hash contract may require raw
  bytes;
- the MiniJinja loader introduced by PR #358 is separate from the native
  `@<path>` expansion path.

The integration sprint must make these choices explicit rather than silently
hashing whichever list happens to be available.

### Required graph behavior

The graph operation is a conservative static dependency walk, not an
execution trace. If an include appears inside a condition, the result contains
every statically discoverable template that may be selected. The result is an
ordered, deduplicated list:

```text
root.md                 -> sha(root)
partials/header.md      -> sha(header)
partials/item.md        -> sha(item)
partials/other-item.md  -> sha(other-item)  # conditional candidate
```

The root is first, and each canonical path appears once at its first
deterministic discovery. A repeated reference does not add another entry:
the referring template's own content hash already captures its include syntax,
condition, and occurrence. This list is the transient graph evidence; it is
not required to be duplicated in atm-core storage.

For each visited/candidate node:

1. Resolve and canonicalize the path under the existing confinement policy.
2. Read and retain source text under the verified upstream text policy.
3. Retain raw bytes separately only when diagnostics or a distinct
   non-compatible digest require them; they are not the upstream file identity.
4. Compute the content-only `TemplateSha256` from verified source text and add
   one deduplicated `(canonical relative path, TemplateSha256)` entry to the
   ordered result.
5. Recurse into every statically discoverable candidate and fail
   deterministically for missing files, cycles, invalid source, or depth
   exhaustion.
6. If a dynamic include cannot be conservatively enumerated, return an
   explicit unresolved-dependency result; do not silently claim the list is
   exhaustive or produce a cacheable identity.
7. Compute an optional aggregate/context identity only after the complete
   deduplicated list is known.

The result should expose both the source identity and inspectable evidence:

```rust
pub struct CompositionFingerprint {
    pub source_sha: sc_sha::CompositionSha256,
    pub hashes: sc_sha::OrderedTemplateHashes,
    pub resolved_files: Vec<PathBuf>,
}
```

The exact public placement may be `ExpandedTemplate` or a dedicated
`CompositionFingerprint` result, but it must not require consumers to rebuild
the graph from rendered text. A separately named rendered-output SHA may be
added later when output verification is required; it must not be conflated
with the source composition identity.

### Native includes and MiniJinja directives

PR #358 adds MiniJinja `{% include %}`, `{% import %}`, and `{% from %}`
inspection/loading, while sc-compose currently expands native `@<path>`
directives. The implementation sprint must choose and document one of these
scopes:

- **Preferred initial scope:** hash the production `@<path>` expansion graph,
  because that is the existing sc-compose composition path. Keep MiniJinja
  dependency loading as a separately tested renderer capability until it is
  wired into the same graph model.
- **Required follow-on if MiniJinja loading becomes production composition:**
  adapt its loader callbacks to emit the same canonical path/hash list and use
  the same `sc-sha` graph/hash APIs. Do not create a second fingerprint
  algorithm.

This avoids claiming that scanning a directive span is equivalent to resolving
and hashing the dependency it names.

## Proposed implementation sprint decomposition

The following are implementation slices to be turned into normal numbered
sprint documents after team-lead assigns the phase letter. They are kept
separate because the shared crate can be reviewed independently from recursive
filesystem/renderer behavior.

### Implementation slice 1 — `sc-sha` core crate and compatibility contract

**Branch/worktree:** `sprint/<phase>-1-sc-sha-crate`, created from the
integration branch after this plan is approved.

**Exact targets:**

- `Cargo.toml`
- `Cargo.lock`
- `crates/sc-sha/Cargo.toml`
- `crates/sc-sha/src/lib.rs`
- `crates/sc-sha/src/file.rs` and/or `composition.rs`
- `crates/sc-sha/tests/compatibility_vectors.rs`
- `crates/sc-composer/Cargo.toml`
- `crates/sc-composer/src/lib.rs`
- `crates/sc-composer/src/template_hash.rs` (delete after migration)
- `docs/architecture.md` or a dedicated ADR for the shared hash ownership
  decision

**Deliverables:**

- Workspace member and publishable `sc-sha` crate.
- Migrate the existing PR #358 calculation as the starting point; correct its
  raw-byte/text encoding behavior rather than inventing a parallel algorithm.
- Verified file-hash calculation and public API compatibility with
  synaptic-canvas-dolt.
- Two-operation public API: hash calculation plus graph/order calculation.
- Stream-backed hash implementation with an optional file adapter that
  delegates to the stream path.
- JSON-variable and rendered-context identity APIs sufficient for atm-core's
  cache restore contract.
- sc-composer re-export/migration surface without duplicate hash code.
- Boundary and dependency review evidence.

**Non-closure:**

- Does not yet discover recursive include candidates in sc-compose.
- Does not change renderer loading behavior.
- Does not claim issue #360 is closed until the recursive integration slice
  lands.

**Parallelism:**

- Can proceed in parallel with comp2's CI fixes and QA on PR #358.
- Must land before the PR #358 final merge if PR #358 is to expose this hash
  API; after comp2 QA, PR #358 should be updated to consume `sc-sha`.

### Implementation sprint 2 — sc-compose integration and Python adapter

**Branch/worktree:** `sprint/<phase>-2-sc-sha-consumers`, created from the
integration branch containing sprint 1.

**Exact targets:**

- `crates/sc-composer/src/include.rs`
- `crates/sc-composer/src/include/expansion.rs`
- `crates/sc-composer/src/include/path.rs` only if a narrow canonical-path
  helper is required
- the chosen sc-composer public result/type module
- `bindings/sc-sha-python/Cargo.toml`
- `bindings/sc-sha-python/pyproject.toml`
- `bindings/sc-sha-python/src/lib.rs`
- recursive include fixtures, compatibility vectors, and Python tests

**Deliverables:**

- Exhaustive candidate discovery for static includes and an ordered,
  path-deduplicated `(canonical path, TemplateSha256)` list.
- sc-compose integration using the two published `sc-sha` operations without
  duplicate hashing or graph logic.
- Separate maturin-built `sc_sha` adapter exposing the same two operations.
- Deterministic failure behavior for missing files, cycles, depth limits,
  confinement violations, and unresolved dynamic includes.
- Cross-platform Rust/Python vectors and non-nested compatibility evidence.

**Hard dependency:** sprint 1's verified Rust API and algorithm vectors.

**Parallelism:** may proceed while comp2 completes PR #358 CI fixes, but must
not fork or reimplement its hash logic. It is the only planned follow-on sprint
unless QA identifies a genuinely independent fix class.

**Non-closure:** does not modify atm-core or synaptic-canvas-dolt. Those repos
consume the published API and provide external acceptance evidence.

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
5. Sprint 2 combines the small sc-compose integration and Python adapter after
   the shared API is stable, avoiding unnecessary worktrees and coordination
   overhead.
6. Release acceptance requires atm-core to prove recursive cache restoration
   from JSON variables plus composition SHA; this is consumer evidence, not an
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

## Test and QA plan

### `sc-sha` tests

The shared crate must test:

- all authoritative synaptic-canvas-dolt vectors;
- empty input, ordinary UTF-8, LF, CRLF, BOM, and no-final-newline cases when
  those are part of the verified contract;
- exact digest bytes and lowercase hex display;
- canonical path separator behavior;
- path-collision distinction when paths are part of the path/hash list;
- deterministic path/hash-list encoding independent of map/traversal
  implementation details;
- algorithm/version domain separation;
- conditional candidates are exhaustive, while include order/repetition remain
  represented by the referring template's file hash;
- canonical JSON-variable vectors with reordered keys, equivalent whitespace,
  Unicode escapes, numeric edge cases, and duplicate-key rejection;
- distinct JSON-variable, composition, and rendered-context identities;
- optional render-option identity behavior.

### Maturin/Python tests

The adapter must test:

- Rust/Python equality for every authoritative file vector;
- Rust/Python equality for every recursive composition fixture;
- exact `bytes` versus `str` input policy;
- lowercase hex output and path/hash result shape;
- missing dependency, cycle, depth, and invalid-path exception mapping;
- exhaustive candidate preservation and path deduplication;
- `maturin develop` in a clean virtual environment;
- wheel build and install on the supported platform matrix.

### Recursive sc-compose tests

Use checked-in fixtures covering:

1. root only;
2. one-level include;
3. multi-level include;
4. changing only a nested child;
5. identical bytes at two different canonical paths;
6. adding and removing a candidate include;
7. reordering or repeating an include in the parent source;
8. repeated candidate paths deduplicating to one list entry;
9. mixed directory nesting and canonical `/` paths;
10. CRLF/LF/BOM/final-newline behavior;
11. missing include;
12. cyclic include;
13. depth-limit failure;
14. confinement/symlink escape;
15. dynamic/unresolved include becoming explicitly non-cacheable;
16. render-option changes when those options are included in the contract;
17. legacy non-nested compatibility behavior.

Every test must assert both the single root fingerprint and enough path/hash
evidence to explain why it changed or remained stable.

### atm-core cache-consumer tests

The atm-core integration must additionally prove:

1. the same recursive template graph plus semantically equivalent canonical
   JSON variables restores the same context;
2. changing a nested template invalidates the cache even when the root file and
   JSON variables are unchanged;
3. changing JSON variables invalidates the cache even when the template graph
   is unchanged;
4. changing dependency order, occurrence count, or renderer-relevant options
   invalidates the cache when those inputs affect rendering;
5. missing/cyclic/depth-exhausted graphs never restore a prior context;
6. cache rows retain and validate all typed identities rather than trusting a
   bare or mismatched SHA string.

### Required validation

Each implementation sprint must run:

```text
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

The shared crate sprint must additionally run its compatibility-vector tests
against the exact source commit recorded in the verification artifact. The
recursive sprint must run the complete include fixture suite on Linux, macOS,
and Windows CI because path, separator, and filesystem confinement behavior
are part of the contract.

The Python adapter sprint must additionally run `maturin build` and install the
resulting wheel into a clean environment before QA handoff. A source checkout
import is not sufficient evidence that the package is usable by Python
consumers.

QA must review directly from the sprint document and verify:

- no duplicate SHA implementation remains in sc-composer;
- no unordered dependency list is used as identity; path deduplication is
  deterministic;
- nested changes alter the root fingerprint;
- exact path and candidate-set rules are tested;
- failure cases do not produce a misleading success fingerprint;
- PR #358's CI fixes remain isolated from the hash-contract migration.

## sc-lint cleanup and QA routing

Each implementation slice must run the applicable sc-lint checks on its final
commit. Minor findings are fixed immediately in the slice worktree. Remaining
findings are grouped by independent rule class and ownership boundary, not by
individual occurrence:

- create a dedicated `fix/` worktree from the slice's final commit;
- keep constant/string findings grouped by owning crate;
- keep length-driven refactors separate from semantic or boundary changes;
- send the worktree path, parent commit, finding evidence, tests, and fix
  commit to team-lead;
- team-lead creates the PR and sends it to quality-mgr for independent QA;
- the implementation slice does not close until required fix PRs are QA
  approved, merged, and revalidated.

The Python adapter slice applies the same routing to Rust, PyO3, Python, and
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

Additional boundary checks:

- No `agent-team-mail-*`, `atm_*`, or `agent_team_mail` dependency is added.
- No `use agent_team_mail::` or `use atm_*::` import is introduced.
- No filesystem traversal, MiniJinja, or CLI dependency is added to `sc-sha`.
- PyO3/maturin dependencies are confined to `bindings/sc-sha-python`; they are
  not optional dependencies of the core crate.
- The new adapter is not silently folded into `bindings/python`, whose existing
  dependency contract remains `bindings/python -> sc-composer` only.
- The shared crate may be used by external consumers only through its published
  versioned API and compatibility vectors.

## Migration and release checklist

### Before implementation

- [ ] Team-lead records the synaptic-canvas-dolt source commit and algorithm.
- [ ] Compatibility vectors are copied into the plan review evidence.
- [ ] The synaptic-canvas-dolt Rust/public API and Python/public API are mapped
      to `sc-sha` symbols and tested, not merely compared by digest output.
- [ ] Team-lead assigns the actual phase/sprint numbers; this document does not
      reserve a phase number.
- [ ] Confirm whether `sc-sha` is published from this workspace or extracted
      to its own repository while preserving the same crate name and API.

### Shared crate sprint

- [ ] Add `crates/sc-sha` to the workspace.
- [ ] Move the verified file hash and tests.
- [ ] Add ordered, path-deduplicated template-hash list types and deterministic
      aggregate/context framing.
- [ ] Add canonical JSON-variable, rendered-context, and distinct typed
      identity results required by atm-core cache restoration.
- [ ] Add resolver-driven recursive hashing with active-stack cycle detection,
      depth protection, and deterministic typed errors.
- [ ] Remove duplicate `template_hash.rs` implementation from sc-composer.
- [ ] Re-export compatible names from sc-composer if PR #358 requires them.
- [ ] Update Cargo.lock and dependency manifests.
- [ ] Run full validation and obtain independent QA approval.

### Maturin adapter

- [ ] Add the separate `bindings/sc-sha-python` package and stable `sc_sha`
      module.
- [ ] Delegate both `calculate_hash` and `calculate_graph_and_order` to
      `sc-sha`.
- [ ] Verify Python API values, result shape, errors, and package version
      against synaptic-canvas-dolt.
- [ ] Build/install a wheel in a clean environment on the supported platforms.
- [ ] Obtain independent QA approval.

### PR #358 follow-up

- [ ] Comp2 completes current CI fixes and QA without mixing the extraction.
- [ ] Rebase/amend PR #358 onto the merged `sc-sha` crate.
- [ ] Remove its local SHA implementation and point the public API at sc-sha.
- [ ] Preserve directive-span and confined-loader scope separately.
- [ ] Re-run PR #358's full CI and request QA review of the combined contract.

### Recursive integration sprint

- [ ] Add exhaustive candidate discovery and path-deduplicated hash-list
      tracking to include expansion.
- [ ] Preserve the verified source text semantics needed by the hash contract;
      retain raw file bytes only when required for diagnostics or a separately
      named non-compatible digest.
- [ ] Compute and expose the recursive path/hash-list and optional root
      fingerprint.
- [ ] Add all required nested, ordering, repetition, collision, and failure
      fixtures.
- [ ] Update normative architecture/requirements documentation.
- [ ] Run full validation and obtain independent QA approval.

### External consumer acceptance (no consumer implementation here)

- [ ] Provide atm-core with the published two-operation API and compatibility
      vectors for recursive cache restoration.
- [ ] Record evidence that atm-core can recompute recursive and JSON-variable
      identities before restoring context.
- [ ] Provide synaptic-canvas-dolt with the per-file compatibility vectors and
      public API mapping for database/lockfile consumption.

## Explicit non-goals

- Do not merge a second independent implementation merely because it produces
  ordinary SHA-256 values.
- Do not use an unordered array of child hashes as the root identity.
- Do not make atm-core depend on a path inside the sc-compose repository.
- Do not make sc-sha aware of MiniJinja syntax or `@<path>` syntax.
- Do not put PyO3/maturin into the core `sc-sha` dependency graph.
- Do not include absolute paths, mtimes, host-specific separators, or
  incidental resolver traversal order in the identity; the defined
  deterministic discovery order is part of the path/hash-list contract.
- Do not claim PR #358 or this planning document closes issue #360 by itself.
