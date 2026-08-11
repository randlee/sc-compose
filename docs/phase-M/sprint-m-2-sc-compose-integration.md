---
id: M.2
title: sc-compose Integration and Python Adapter
phase: M
status: complete
branch: sprint/m-2-sc-compose-integration
worktree: ../sc-compose-worktrees/sprint/m-2-sc-compose-integration
target: integrate/phase-m
---

# Sprint M.2 — sc-compose integration and Python adapter

## Goal

Connect sc-compose's production include graph to the shared two-operation API
and ship the thin maturin adapter with deterministic nested-graph evidence.

This is an implementation sprint, not a planning-only sprint; it produces
sc-composer and Python runtime changes described by its non-doc targets.

**Traceability:** closes SHA-R2 (API consumption), SHA-R3 (recursive graph),
SHA-R4 (external cache-restore evidence), SHA-R5 (maturin adapter), SHA-N1
(cross-platform behavior), SHA-N2 (ownership boundary), and SHA-N3 (failure and
validation evidence).

## Hard Dependencies

M.1's merged, QA-approved public Rust API and algorithm
vectors. It must start from `integrate/phase-m` after M.1 is merged. It may
run in parallel with comp2's PR #358 CI fixes and QA, but it may not fork or
reimplement hash logic. No other planned sprint is required.

**Target:** `integrate/phase-m` after M.1 is merged.

The authoritative branch and worktree are in this document's frontmatter.

## Parallel Execution

M.2 may run in parallel with comp2's independent PR #358 CI/QA work after M.1
is merged; it is not parallel with M.1 because it consumes the verified API.
Any sc-lint cleanup worktree is created from M.2's final commit and grouped by
independent rule class under the phase-level routing policy.

## Exact Targets

- `crates/sc-composer/src/include.rs`
- `crates/sc-composer/src/include/expansion.rs`
- `crates/sc-composer/src/include/fingerprint.rs`
- `crates/sc-composer/src/include/path.rs`
- `crates/sc-composer/src/diagnostics/schema.rs`
- `docs/architecture.md`
- `bindings/sc-sha-python/Cargo.toml`
- `bindings/sc-sha-python/pyproject.toml`
- `bindings/sc-sha-python/src/lib.rs`
- `bindings/sc-sha-python/README.md`
- `bindings/sc-sha-python/python/sc_sha/__init__.py`
- `bindings/sc-sha-python/python/sc_sha/__init__.pyi`
- `bindings/sc-sha-python/tests/test_compatibility.py`
- `boundaries/sc-sha-python/python-adapter.toml`
- recursive include fixtures and compatibility vectors

## Paths to Delete

None planned; if implementation reveals a duplicate
hash/graph helper, record it here before deletion and route it as a separate
fix class if it is not covered by M.2's exact targets.

## Required Work

- Implement the graph exactly as specified in the `Recursive sc-compose
  integration contract` below: caller-supplied manifest assembly, statically
  exhaustive candidates, ordered edges, tagged-source deduplication, memoized
  per-source hashing, and deterministic resolver failures.
- Implement a separate maturin `sc_sha` adapter delegating the same two Rust
  operations, with stable result/error mappings and clean wheel installation.
- Add `boundaries/sc-sha-python/python-adapter.toml` with an explicit allowlist
  for published `sc-sha` and the approved PyO3 adapter dependencies only;
  prove that the adapter cannot acquire sc-compose, sc-composer, ATM, or
  unrelated runtime dependencies.
- Keep MiniJinja loader integration as the explicitly deferred scope described
  in `Native includes and MiniJinja directives`.
- Record the post-M.2 handoff for PR #358 to consume the published `sc-sha`
  API, remove its local hash implementation, preserve its directive-span and
  confined-loader scope, and rerun its full CI/QA. This follow-up is a
  phase-level gate after M.2, not an M.2 deliverable or an unowned third
  sprint.

## Explicit Code Samples

```rust
pub fn calculate_hash(input: HashInput<'_>) -> Result<HashResult, ShaError>;
pub fn calculate_composition_hash(
    manifest: &ResolvedTemplateManifest,
) -> Result<CompositionSha256, CompositionError>;
```

```python
def calculate_hash(input: dict) -> dict: ...
def calculate_composition_hash(manifest: dict) -> dict: ...
```

## Deliverables

- Exhaustive static candidate discovery and deterministic,
  path-deduplicated manifest nodes plus ordered include edges.
- sc-compose integration using the two published sc-sha operations without
  duplicate hashing or graph logic.
- A maturin-built `sc_sha` adapter exposing those same two operations.
- Deterministic missing-file, cycle, depth, confinement, and unresolved
  dynamic-include outcomes.
- Cross-platform Rust/Python vectors, nested fixtures, and wheel evidence.
- A complete handoff record for the post-M.2 PR #358 follow-up, which will
  consume `sc-sha` without restoring a duplicate implementation.

## Acceptance Criteria

- `[SHA-R2, SHA-R3]` The root appears first, each tagged canonical source node
  appears once, conditional candidates are exhaustive, and repeated references
  remain represented as ordered edges while nodes are deduplicated.
- `[SHA-R3]` A diamond fixture proves each canonical source is hashed once per
  graph computation, while all distinct edges remain in the manifest.
- `[SHA-R3]` The manifest encoder's version tag and length-delimited framing are
  proven injective by adversarial path/tag/order/hash/edge fixtures; malformed
  manifests fail before hashing.
- `[SHA-N1]` Linux, macOS, and Windows CI produce identical path/hash and
  composition results for the defined fixtures.
- `[SHA-R4]` Consumer evidence proves a nested child change changes
  `CompositionSha256`, while touching an unrelated file outside the graph does
  not change it; no atm-core code is added to this repository.
- `[SHA-R5]` `maturin develop` and a clean wheel install expose the same two
  operations, values, result shapes, and typed errors as Rust.
- `[SHA-N2, SHA-N3]` Missing, cyclic, depth-exhausted, confinement-violating,
  and unresolved dynamic graphs are rejected by sc-compose and never passed as
  a successful manifest; each failure has the stable code and `IncludeError`
  mapping listed in the [resolver error inventory](phase-M-plan.md#resolver-error-inventory);
  no duplicate implementation remains.
- `[SHA-N2]` The Python adapter boundary record passes `just lint sc-boundary`
  with only the approved `sc-sha`/PyO3 dependency edges and a negative fixture
  rejects an attempted dependency on sc-compose, sc-composer, ATM, or an
  unrelated runtime package.
- `[SHA-N2, SHA-N3]` The PR #358 follow-up is recorded as a post-M.2 phase
  gate, with its renderer/directive behavior isolated from the shared hash
  migration. M.2 closure does not depend on that separate PR being merged.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `just lint sc-boundary`
- `cargo test -p sc-compose --test sc_lint_sc_boundary`
- complete recursive fixture suite on Linux, macOS, and Windows CI
- `maturin build` followed by installation into a clean virtual environment
- Rust/Python compatibility-vector and result/error-shape tests
- `git diff --check`

## QA Handoff

Send team-lead the branch/worktree, M.1 parent commit, exact
targets, fixtures, requirement mapping, validation output, wheel artifact
evidence, and any sc-lint findings. Team-lead opens the PR and routes it to
quality-mgr. M.2 is not complete until QA approval, merge, and post-merge
revalidation are recorded.

The PR #358 follow-up is not part of this sprint's closure boundary. Its
post-M.2 phase gate remains recorded in `phase-M-plan.md` and must be routed
through its own CI and QA handoff before Phase M closes.

The phase plan's `sc-lint cleanup and QA routing` section is authoritative for
minor findings and dedicated `fix/` worktree routing; this sprint handoff must
include its finding evidence and parent commit.

## Recursive sc-compose integration contract

The existing include engine is the correct ownership point for recursive
composition discovery. It already resolves `@<path>` includes, enforces root
confinement, detects cycles/depth, and records resolved files and source text.
sc-compose must build the resolved manifest and then call
`sc_sha::calculate_composition_hash`; sc-sha must not absorb any of those
resolver responsibilities. The current first-seen file collection cannot
represent the full identity:

- the public model does not expose a deterministic ordered list of unique
  tagged source/hash nodes plus ordered include edges;
- source text is retained as `String`, while the hash contract may require raw
  bytes at the file-reading boundary before strict UTF-8 normalization;
- the MiniJinja loader introduced by PR #358 is separate from the native
  `@<path>` expansion path.

The integration sprint must make these choices explicit rather than silently
hashing whichever traversal list happens to be available.

### Required graph behavior

The sc-compose graph builder is a conservative static dependency walk, not an
execution trace. If an include appears inside a condition, the manifest
contains every statically discoverable template that may be selected. The
caller-owned manifest contains unique nodes and ordered edges:

```text
root.md                 -> sha(root)
partials/header.md      -> sha(header)
partials/item.md        -> sha(item)
partials/other-item.md  -> sha(other-item)  # conditional candidate
root.md -> partials/item.md (occurrence 0)
```

Nodes are ordered by deterministic first discovery and deduplicated by the
tagged canonical source. Repeated references do not duplicate a node, but each
ordered edge/occurrence is retained so the composition encoding remains
injective. The referring template's content hash also captures its include
syntax, condition, and occurrence. This manifest is transient graph evidence;
ATM need not duplicate it when every child is already stored individually.

For each visited/candidate node, sc-compose must:

1. Resolve and canonicalize the local path under the existing confinement
   policy, then wrap it as `CanonicalSource::LocalPath(CanonicalTemplatePath)`.
2. Read source bytes, call `sc_sha::calculate_hash`, and memoize the resulting
   `TemplateSha256` by the tagged canonical source. A diamond dependency must
   hash a source exactly once per graph computation.
3. Add one node per canonical source and preserve every ordered include
   occurrence as an edge.
4. Recurse into every statically discoverable candidate and fail
   deterministically for missing files, cycles, invalid source, or depth
   exhaustion. These are sc-compose errors, not sc-sha errors.
5. If a dynamic include cannot be conservatively enumerated, return an explicit
   unresolved-dependency result; do not silently claim the manifest is
   exhaustive or produce a cacheable identity.
6. After the complete graph manifest is assembled, call
   `sc_sha::calculate_composition_hash` exactly once; that operation is the
   sole sc-sha structural-validation gate before the digest is produced.

The result should expose both the source identity and inspectable evidence:

```rust
pub struct CompositionFingerprint {
    pub source_sha: sc_sha::CompositionSha256,
    pub manifest: sc_sha::ResolvedTemplateManifest,
    pub resolved_files: Vec<PathBuf>,
}
```

The exact public placement may be `ExpandedTemplate` or a dedicated
`CompositionFingerprint` result, but it must not require consumers to rebuild
the manifest from rendered text. A separately named rendered-output SHA may be
added later when output verification is required; it must not be conflated with
the source composition identity.

M.2 places `CompositionFingerprint` on `ExpandedTemplate` and carries it
through `ComposeResult`. The existing Python `ExpandedTemplate` and
`ComposeResult` adapters expose its `composition_sha256`; the standalone
`sc_sha` package exposes the complete two-operation dictionary contract.

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

## Test Fixtures
### Maturin/Python tests

The adapter must test:

- Rust/Python equality for every authoritative file vector;
- Rust/Python equality for every recursive composition fixture;
- exact `bytes` versus `str` input policy;
- lowercase hex output and manifest/result shape;
- malformed-manifest and invalid-text exception mapping;
- preservation of caller-supplied node order and edge occurrences;
- `maturin develop` in a clean virtual environment;
- wheel build and install on the supported platform matrix.

### Recursive sc-compose tests

Use checked-in fixtures covering:

1. root only;
2. one-level and multi-level includes;
3. changing exactly one nested child changes `CompositionSha256`;
4. touching an unrelated file outside the graph leaves `CompositionSha256`
   unchanged;
5. identical content at two distinct canonical paths remains two tagged nodes;
6. adding/removing a conditional candidate changes the manifest;
7. reordering or repeating an include changes ordered edges while preserving
   node deduplication;
8. a diamond dependency hashes one source once but retains both edges;
9. mixed directory nesting and canonical local-path representation;
10. CRLF/LF/BOM/final-newline behavior at the text-hash boundary;
11. missing include, cyclic include, depth-limit, and confinement/symlink
    escape errors owned by sc-compose;
12. dynamic/unresolved include becoming explicitly non-cacheable;
13. tagged URL/local-source non-collision fixtures for the forward-compatible
    manifest model;
14. legacy non-nested compatibility behavior.

Conditional-candidate coverage uses the native statically enumerable form
`@<{{ "item.md" if mode == "item" else "other-item.md" }}>`. Both branch
paths must enter the manifest while the expanded template preserves the Jinja
condition for render-time selection; arbitrary dynamic targets remain an
explicit unresolved-dependency error.

Every test must assert the composition fingerprint and enough manifest
node/edge evidence to explain why it changed or remained stable.

## This Sprint Does Not Close

atm-core or synaptic-canvas-dolt code changes,
their persistence/schema changes, MiniJinja recursive loading unless separately
approved, or issue #360 until external consumer acceptance is complete.
