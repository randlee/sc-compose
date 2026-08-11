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

The verification artifact must be committed as documentation or test vectors in
the `sc-sha` implementation sprint. A plain SHA-256 implementation that merely
matches the empty-string vector is not evidence of compatibility. The
implementation must explicitly settle whether newline translation occurs at
the file-reading boundary, because PR #358 currently hashes an arbitrary byte
slice while synaptic-canvas-dolt hashes text loaded with `read_text`.

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

impl TemplateSha256 {
    pub const fn as_bytes(&self) -> &[u8; 32];
    pub fn to_hex(self) -> String;
}

impl std::fmt::Display for TemplateSha256 { /* lowercase contract */ }

#[must_use]
pub fn sha256_text(content: &str) -> TemplateSha256;

/// Optional migration facade for UTF-8 file bytes. Its conversion and newline
/// policy must be proven equivalent to the synaptic-canvas-dolt reader.
pub fn template_sha256(utf8_file_bytes: &[u8]) -> Result<TemplateSha256, ShaError>;
```

`sha256_text` is the canonical compatibility function. `template_sha256` may
remain as a migration facade for PR #358 only if its UTF-8 decoding and newline
policy are proven equivalent. It must not imply that arbitrary binary bytes are
accepted when the upstream contract is text-based. If a raw-byte digest is
useful for a separate consumer, expose it under a distinct name and do not use
it for synaptic-canvas-dolt-compatible identity.

The digest type retains the currently proposed `as_bytes`, `to_hex`, and
lowercase `Display` surface. Add compile-level/API-shape examples showing that
a consumer can obtain the stored-compatible string without reaching into
private fields.

The recursive API must prevent accidental mixing of a file digest and a
composition digest. Prefer distinct types even if both currently contain 32
bytes:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyEdge {
    /// Canonical, repository-relative include target using `/` separators.
    pub include_path: String,
    /// Child node identity after path and verified source text are incorporated.
    pub child_sha: TemplateSha256,
    /// Zero-based occurrence in the parent's source order.
    pub occurrence: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionManifest {
    /// Versioned algorithm/domain identifier.
    pub algorithm: String,
    /// Canonical path of the composed root.
    pub root_path: String,
    /// Identity of the root node.
    pub root_sha: TemplateSha256,
    /// Ordered edges, including repeated occurrences.
    pub edges: Vec<DependencyEdge>,
    /// Optional canonical render-option identity when source alone is not
    /// sufficient to determine the rendered artifact.
    pub render_options_sha: Option<TemplateSha256>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompositionSha256([u8; 32]);

impl CompositionSha256 {
    pub const fn as_bytes(&self) -> &[u8; 32];
    pub fn to_hex(self) -> String;
}

#[must_use]
pub fn composition_sha256(manifest: &CompositionManifest) -> CompositionSha256;
```

The final field names and encoding remain subject to the synaptic-canvas-dolt
verification gate. The plan requires the following invariants regardless of
the final spelling:

- canonical relative paths only; no absolute machine-specific paths;
- normalized `/` separators;
- deterministic length-delimited or equivalently unambiguous encoding;
- explicit algorithm/version domain separation;
- exact UTF-8 text identity matching the upstream reader, or an explicitly
  documented source normalization rule;
- nested and transitive dependencies included recursively;
- include order and repeated occurrences preserved;
- render-option identity included only when it affects output;
- no mtime, directory traversal order, locale, or host-path dependence;
- cycles and missing dependencies fail before producing a success fingerprint.

The manifest is diagnostic evidence. Consumers compare the single
`CompositionSha256`; they must not use an unordered array of child hashes as
the identity.

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

pub fn recursive_composition_sha256<S: DependencySource>(
    root_path: &str,
    source: &mut S,
) -> Result<CompositionFingerprint, CompositionHashError<S::Error>>;
```

The exact ownership of `TemplateSource` and the error wrapper may change after
the synaptic-canvas-dolt API is verified, but the behavior is mandatory:

- maintain an active recursion stack and reject a path already on that stack;
- return a typed cycle error containing the deterministic cycle path;
- enforce a configurable depth ceiling before recursion can overflow the stack;
- cache completed node identities without dropping repeated edge occurrences;
- load dependencies in declared/source order;
- reject malformed canonical paths before hashing;
- never return a successful root fingerprint for a missing, cyclic, or failed
  dependency.

The resolver must be deliberately generic. A filesystem-backed resolver can be
implemented by sc-composer; an in-memory resolver can be used by atm-core,
Python, and deterministic tests. This keeps `sc-sha` reusable without hiding
filesystem security policy inside a hash crate.

The returned `CompositionFingerprint` must include the `CompositionManifest`
and root `CompositionSha256`. It may include per-node evidence, but the public
contract must make the single root identity easy to consume.

### Maturin/Python API

Keep PyO3 and maturin dependencies out of `sc-sha` itself. The adapter package
should expose a stable Python module named `sc_sha` and delegate every
calculation to Rust:

```python
from sc_sha import sha256_text, recursive_composition_sha256

sha256_text("template text")
# -> lowercase hexadecimal string

recursive_composition_sha256(
    root_path="root.md",
    templates={
        "root.md": {"text": "@<partial.md>\n", "dependencies": ["partial.md"]},
        "partial.md": {"text": "body\n", "dependencies": []},
    },
)
# -> a stable result containing composition_sha and manifest evidence
```

The final Python names and result mapping must be reconciled with the verified
synaptic-canvas-dolt public API. The adapter must define:

- accepted Python types for bytes, paths, dependency records, and optional
  render options;
- lowercase hex and explicitly documented text/bytes input-output behavior;
- typed Python exceptions for invalid paths, missing nodes, cycles, and depth
  exhaustion;
- deterministic dictionary/list serialization for manifest evidence;
- minimum supported Python versions and platform wheels;
- package/module version alignment with the Rust algorithm version;
- a clean-consumer `maturin develop` and wheel-install test.

The Python API must not accept an unordered set as the only representation of
dependencies. A mapping may be used for node lookup, but each node's ordered
dependency list remains authoritative.

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

- repeated includes are deduplicated in `resolved_files`;
- the public model does not expose an ordered edge list for each occurrence;
- source text is retained as `String`, while the hash contract may require raw
  bytes;
- the MiniJinja loader introduced by PR #358 is separate from the native
  `@<path>` expansion path.

The integration sprint must make these choices explicit rather than silently
hashing whichever list happens to be available.

### Required graph behavior

During expansion, record an ordered edge for every include occurrence:

```text
root.md
  edge[0] -> partials/header.md
  edge[1] -> partials/item.md
  edge[2] -> partials/item.md   # repeated occurrence is retained
```

For each visited node:

1. Resolve and canonicalize the path under the existing confinement policy.
2. Read and retain source text under the verified upstream text policy.
3. Retain raw bytes separately only when diagnostics or a distinct
   non-compatible digest require them.
4. Compute the node identity from the canonical relative path and verified
   source text, if required by the verified algorithm.
5. Append each parent-to-child edge in source order, retaining occurrence.
6. Recurse into the child and fail deterministically for missing files, cycles,
   invalid source, or depth exhaustion.
7. Compute the root composition identity only after the complete graph is
   known.

The result should expose both the source identity and inspectable evidence:

```rust
pub struct CompositionFingerprint {
    pub source_sha: sc_sha::CompositionSha256,
    pub manifest: sc_sha::CompositionManifest,
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
  adapt its loader callbacks to emit the same canonical graph edges and use the
  same `sc-sha` manifest API. Do not create a second fingerprint algorithm.

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
- Verified file-hash calculation and public API compatibility with
  synaptic-canvas-dolt.
- Resolver-driven recursive composition-hash API with deterministic encoding,
  cycle detection, and typed errors.
- sc-composer re-export/migration surface without duplicate hash code.
- Boundary and dependency review evidence.

**Non-closure:**

- Does not yet discover recursive include edges in sc-compose.
- Does not change renderer loading behavior.
- Does not claim issue #360 is closed until the recursive integration slice
  lands.

**Parallelism:**

- Can proceed in parallel with comp2's CI fixes and QA on PR #358.
- Must land before the PR #358 final merge if PR #358 is to expose this hash
  API; after comp2 QA, PR #358 should be updated to consume `sc-sha`.

### Implementation slice 2 — `sc-sha` maturin/Python adapter

**Branch/worktree:** `sprint/<phase>-2-sc-sha-python`, created from the
integration branch containing slice 1.

**Exact targets:**

- `Cargo.toml` workspace member list
- `bindings/sc-sha-python/Cargo.toml`
- `bindings/sc-sha-python/pyproject.toml`
- `bindings/sc-sha-python/src/lib.rs`
- `bindings/sc-sha-python/tests/test_compatibility.py`
- Python package metadata and wheel CI configuration, if the repository owns
  the release workflow

**Deliverables:**

- Maturin-built `sc_sha` module that delegates to `sc-sha`.
- File and recursive composition hash functions.
- Stable hex/result/error contracts documented and tested.
- Linux, macOS, and Windows wheel/build characterization.
- Python compatibility vectors matching Rust and synaptic-canvas-dolt.

**Hard dependency:** slice 1's verified Rust API and algorithm vectors.

**Parallelism:** can run in parallel with implementation slice 3 after slice 1
  lands. It can also proceed while comp2 completes PR #358 CI fixes, but it
  must not fork or reimplement the PR's hash logic.

**Non-closure:** does not modify `bindings/python` or expose sc-compose CLI
  behavior through Python; it provides the standalone `sc_sha` package only.

### Implementation slice 3 — recursive sc-compose composition fingerprint

**Branch/worktree:** `sprint/<phase>-3-recursive-composition-sha`, created from
the integration branch containing slice 1.

**Exact targets:**

- `crates/sc-composer/src/include.rs`
- `crates/sc-composer/src/include/expansion.rs`
- `crates/sc-composer/src/include/path.rs` if canonical path data needs a
  narrow helper
- the chosen public result/type module for `CompositionFingerprint`
- recursive include fixtures and integration tests
- `docs/architecture.md` and `docs/requirements.md` only where the public
  contract requires normative updates

**Deliverables:**

- Ordered, occurrence-preserving dependency edges from actual expansion.
- Verified source-text retention or a documented equivalent compatible with
  `sc-sha`.
- Recursive source composition fingerprint returned with inspectable manifest.
- Deterministic failure behavior for missing files, cycles, depth limits, and
  confinement violations.
- Documented compatibility behavior for non-nested templates.

**Non-closure:**

- Does not implement the synaptic-canvas-dolt algorithm in sc-compose.
- Does not add ATM runtime dependencies or direct atm-core integration.
- Does not hash rendered output unless a separately approved requirement adds
  that contract.

**Parallelism:**

- Starts only after slice 1 is merged to the integration branch.
- Can run in parallel with slice 2 and unrelated PR #358 CI/QA follow-up after
  the shared crate API is stable, but cannot merge before slice 1.

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
5. The recursive composition sprint then consumes the same crate without
   another hash migration.

The merge gate is therefore:

```text
plan approved
  -> verify synaptic-canvas-dolt algorithm
  -> sc-sha core implementation + QA
  -> (sc-sha Python adapter + QA || recursive sc-compose fingerprint + QA)
  -> PR #358 updated to consume sc-sha + QA
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
- path-collision distinction when paths are part of node identity;
- deterministic manifest encoding independent of map/traversal order;
- algorithm/version domain separation;
- repeated and reordered edges producing distinct composition identities;
- optional render-option identity behavior.

### Maturin/Python tests

The adapter must test:

- Rust/Python equality for every authoritative file vector;
- Rust/Python equality for every recursive composition fixture;
- exact `bytes` versus `str` input policy;
- lowercase hex output and manifest result shape;
- missing dependency, cycle, depth, and invalid-path exception mapping;
- repeated and reordered dependency preservation;
- `maturin develop` in a clean virtual environment;
- wheel build and install on the supported platform matrix.

### Recursive sc-compose tests

Use checked-in fixtures covering:

1. root only;
2. one-level include;
3. multi-level include;
4. changing only a nested child;
5. identical bytes at two different canonical paths;
6. adding and removing an edge;
7. reordering edges;
8. repeating an edge;
9. mixed directory nesting and canonical `/` paths;
10. CRLF/LF/BOM/final-newline behavior;
11. missing include;
12. cyclic include;
13. depth-limit failure;
14. confinement/symlink escape;
15. render-option changes when those options are included in the contract;
16. legacy non-nested compatibility behavior.

Every test must assert both the single root fingerprint and enough manifest
evidence to explain why it changed or remained stable.

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
- no unordered or first-seen-only dependency list is used as identity;
- nested changes alter the root fingerprint;
- exact path and occurrence rules are tested;
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
- [ ] Add composition manifest types and deterministic hash function.
- [ ] Add resolver-driven recursive hashing with active-stack cycle detection,
      depth protection, and deterministic typed errors.
- [ ] Remove duplicate `template_hash.rs` implementation from sc-composer.
- [ ] Re-export compatible names from sc-composer if PR #358 requires them.
- [ ] Update Cargo.lock and dependency manifests.
- [ ] Run full validation and obtain independent QA approval.

### Maturin adapter

- [ ] Add the separate `bindings/sc-sha-python` package and stable `sc_sha`
      module.
- [ ] Delegate file and recursive composition calculations to `sc-sha`.
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

- [ ] Add ordered dependency occurrence tracking to include expansion.
- [ ] Preserve the verified source text semantics needed by the hash contract;
      retain raw file bytes only when required for diagnostics or a separately
      named non-compatible digest.
- [ ] Compute and expose the recursive manifest/root fingerprint.
- [ ] Add all required nested, ordering, repetition, collision, and failure
      fixtures.
- [ ] Update normative architecture/requirements documentation.
- [ ] Run full validation and obtain independent QA approval.

## Explicit non-goals

- Do not merge a second independent implementation merely because it produces
  ordinary SHA-256 values.
- Do not use an unordered array of child hashes as the root identity.
- Do not make atm-core depend on a path inside the sc-compose repository.
- Do not make sc-sha aware of MiniJinja syntax or `@<path>` syntax.
- Do not put PyO3/maturin into the core `sc-sha` dependency graph.
- Do not include absolute paths, mtimes, host-specific separators, or traversal
  order in the identity.
- Do not claim PR #358 or this planning document closes issue #360 by itself.
