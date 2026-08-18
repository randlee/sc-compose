---
id: P.1
title: Generated sc-sha Go Adapter Contract
phase: P
status: planned
branch: sprint/p-1-generated-sc-sha-go-adapter
worktree: ../sc-compose-worktrees/sprint/p-1-generated-sc-sha-go-adapter
target: integrate/phase-p
---

# Sprint P.1 — Generated `sc-sha` Go adapter contract

## Goal

Create a generated UniFFI Go adapter for the two existing `sc-sha` operations,
with a pinned generator/toolchain, committed generated Go source, typed public
records/errors, and cross-language conformance tests. The adapter is the first
reusable Go binding foundation; it is not handwritten CGo glue.

**Traceability:** P-R1, P-R2, P-R3, P-N1, and P-N2.

## Dependencies and parallelism

Before source work begins, ADR-0020 must be accepted and team-lead must record
approval of the ADR-0018 / `CLAUDE.md` / sc-boundary amendment. The adapter
starts from the Phase P parent selected by team-lead. P.1 blocks P.2. It may
run in parallel only with unrelated work that does not touch `sc-sha`,
`bindings/sc-sha-go`, ADR-0018, `CLAUDE.md`, workspace membership, or the
named boundary inventories.

## Exact targets

- `Cargo.toml`
- `Cargo.lock`
- `.gitignore`
- `deny.toml`
- `bindings/sc-sha-go/Cargo.toml`
- `bindings/sc-sha-go/src/lib.rs`
- `bindings/sc-sha-go/src/sc_sha_go.udl`
- `bindings/sc-sha-go/uniffi.toml`
- `bindings/sc-sha-go/go.mod`
- `bindings/sc-sha-go/go/` (generated Go package; committed output)
- `bindings/sc-sha-go/tests/` (Go and Rust adapter fixtures)
- `bindings/sc-sha-go/testdata/conformance-v1.json`
- `bindings/sc-sha-go/README.md`
- `bindings/sc-sha-python/src/lib.rs`
- `bindings/sc-sha-python/tests/test_compatibility.py`
- `Justfile`
- `.github/workflows/ci.yml`
- `boundaries/sc-sha/shared-library.toml`
- `boundaries/sc-sha-go/go-adapter.toml` (new)
- `tests/fixtures/sc-lint/sc-boundary/sc-sha-go-forbidden-edge/Cargo.toml`
- `crates/sc-compose/tests/sc_lint_sc_boundary.rs`
- `crates/sc-compose/tests/support/mod.rs`
- `CLAUDE.md`
- `docs/adrs/0018-sc-sha-hash-ownership.md`
- `docs/adrs/0020-generated-go-binding-strategy.md`

## Required work

1. Complete the approval gate before authoring the adapter: accept ADR-0020;
   amend ADR-0018 to list `bindings/sc-sha-go`; amend `CLAUDE.md` and the
   boundary inventories so the new allowed dependency direction is machine
   checked.
2. Add `bindings/sc-sha-go` as a workspace member. It may depend on `sc-sha`,
   the pinned UniFFI runtime, and narrowly required adapter serialization/error
   dependencies only. It must not add those dependencies to `sc-sha`.
3. Define the complete source interface in `sc_sha_go.udl`: two functions,
   template/composition hash values, `CanonicalSource`, nodes, edges, V1
   manifest, and stable typed error variants. The Rust adapter validates and
   converts these values, then calls the existing two `sc-sha` public
   functions. It owns no hashing implementation.
4. Pin UniFFI `0.31.0` and `uniffi-bindgen-go` `v0.7.1+v0.31.0` in committed
   tool configuration. Add one deterministic generation command and CI check
   that regenerates `bindings/sc-sha-go/go/` and fails on a diff. Generated Go
   output is reviewed and committed; no developer edits it by hand. An upgrade
   is a deliberate compatibility change with regenerated vectors and review,
   not a floating tool installation.
5. Generate the Go package and prove it builds and tests with CGo enabled
   under normal Go pointer checking. Neither command nor documentation may set
   `GODEBUG=cgocheck=0` or `GODEBUG=invalidptr=0`.
6. Add shared vectors covering ordinary Unicode, LF/CRLF/bare-CR normalization,
   BOM, no final newline, empty input, invalid UTF-8, invalid digest, invalid
   source, unsupported schema, duplicate node, and unknown edge. Rust, Python,
   and Go must assert the same lowercase digest or stable error code.
7. Add the named negative sc-boundary fixture, proving that a Go adapter edge
   to `sc-compose`, `sc-composer`, ATM code, a resolver/filesystem package, or
   another adapter fails the boundary check.

## Explicit interface contract

The UDL is the source of the generated binding. It must be semantically
equivalent to this contract; exact UniFFI syntax follows the pinned version:

```text
enum CanonicalSource { LocalPath(string), Url(string) }
record TemplateHash { string sha256; }
record CompositionHash { string sha256; }
record ResolvedTemplateNode { CanonicalSource source; string sha256; }
record ResolvedIncludeEdge {
  CanonicalSource parent;
  CanonicalSource child;
  u32 occurrence;
}
record ResolvedTemplateManifest {
  string schema;
  sequence<ResolvedTemplateNode> nodes;
  sequence<ResolvedIncludeEdge> edges;
}
error ScShaError {
  InvalidUtf8;
  InvalidDigest;
  InvalidManifest(string);
  UnsupportedManifestSchema;
  DuplicateSource;
  UnknownEdgeEndpoint;
}
TemplateHash calculate_hash(bytes utf8_file_bytes) throws ScShaError;
CompositionHash calculate_composition_hash(
  ResolvedTemplateManifest manifest
) throws ScShaError;
```

The generated Go surface must expose one typed error path carrying the stable
`SC_SHA_*` code and a human-readable message. It must not expose JSON blobs,
`map[string]any`, caller-owned raw pointers, an async operation, or any third
hash calculation.

## Deliverables

- a pinned, generated UniFFI Go adapter for `sc-sha`;
- committed Go output plus deterministic regeneration/drift CI;
- typed manifest/source/error contract equivalent to the Rust/Python adapter;
- cross-language conformance vectors and normal-safety CGo build proof;
- ADR-0018/ADR-0020/`CLAUDE.md`/sc-boundary evidence for the new adapter.

## Acceptance criteria

- [ ] Exactly `CalculateHash` and `CalculateCompositionHash` are public
      domain operations; both delegate to `sc-sha` and no duplicate hashing
      code exists in the adapter or generated package.
- [ ] The generated Go package represents source/node/edge/manifest values and
      stable errors as types, not untyped JSON or maps.
- [ ] Pinned generation reproduces the committed Go output byte-for-byte and
      CI fails on drift.
- [ ] The Go package passes all vectors and agrees with Rust/Python on every
      successful digest and stable error code.
- [ ] CGo tests execute under normal pointer checking; no committed command,
      workflow, or documentation weakens Go FFI safety checks.
- [ ] `sc-sha` retains no adapter/runtime dependency, while the new boundary
      inventory and negative fixture reject forbidden edges.
- [ ] ADR-0020 and the ADR-0018 / `CLAUDE.md` amendments are accepted before
      the adapter source is merged.

## Required validation

```text
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
just lint-ci-consumer
just generate-sc-sha-go check
(cd bindings/sc-sha-go && go test ./...)
git diff --check
```

`just lint-ci-consumer` is the provisioned, CI-authoritative sc-lint profile.
Bare `just lint` remains blocked by the tracked sc-lint external-binary
bootstrap defect (`O5-SC-LINT-BOOTSTRAP-001`); no suppression is permitted in
this sprint.

## QA handoff

Send team-lead the adapter branch/worktree, selected parent, pinned generator
versions, generated-output drift evidence, vector results for Rust/Python/Go,
boundary negative-fixture result, and all validation output. Team-lead opens
the PR and routes it to quality-mgr. Apply the Phase P sc-lint cleanup and
fix-worktree routing before declaring P.1 complete.

## This sprint does not close

External Go-module publication, prebuilt native-library artifacts, consumer
repository adoption, a rust2go adapter, or Go bindings for `sc-composer`.
