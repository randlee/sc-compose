---
id: M.1
title: sc-sha Core Crate and Compatibility Contract
phase: M
status: planned
branch: sprint/m-1-sc-sha-core-crate
worktree: ../sc-compose-worktrees/sprint/m-1-sc-sha-core-crate
target: integrate/phase-m
---

# Sprint M.1 — `sc-sha` core crate and compatibility contract

## Goal

Extract the verified file/composition hash contract into a publishable `sc-sha`
crate, expose the two typed public operations, and remove duplicate hash
ownership from sc-composer.

This is an implementation sprint, not a planning-only sprint; it produces the
core crate and compatibility implementation described by its non-doc targets.

**Traceability:** closes SHA-R1 (Rust definition), SHA-R2 (API definition),
SHA-R3 (manifest framing contract), SHA-R4 (composition consumer contract), SHA-N1 (normalization), SHA-N2
(boundary), SHA-N3 (production-readiness), and ADR-SHA-001. M.2 is responsible
for the Python consumption proof that completes SHA-R1.

## Hard Dependencies

None. The sprint starts from `integrate/phase-M`, whose selected parent commit
is recorded by team-lead after plan approval. It may run in parallel with
comp2's independent PR #358 CI fixes and QA, but neither branch may merge a
duplicate hash implementation.

**Target:** `develop` through `integrate/phase-M`.

The selected parent commit is recorded by team-lead after plan approval; M.2
does not block M.1.

The authoritative branch and worktree are in this document's frontmatter.

## Parallel Execution

M.1 may run in parallel with comp2's independent PR #358 CI fixes and QA. It
must not run in parallel with M.2 because M.2 consumes M.1's merged API. The
PR #358 branch may not merge a duplicate hash implementation while M.1 is
open.

## Exact Targets

- `Cargo.toml`
- `Cargo.lock`
- `crates/sc-sha/Cargo.toml`
- `crates/sc-sha/src/lib.rs`
- `crates/sc-sha/src/file.rs`
- `crates/sc-sha/src/manifest.rs`
- `crates/sc-sha/src/composition.rs`
- `crates/sc-sha/tests/compatibility_vectors.rs`
- `crates/sc-composer/Cargo.toml`
- `crates/sc-composer/src/lib.rs`
- `CLAUDE.md` (Boundary Rules amendment)
- `docs/adrs/0018-sc-sha-hash-ownership.md`

## Paths to Delete

`crates/sc-composer/src/template_hash.rs`, only after
all supported call sites use `sc-sha` and the compatibility facade/re-export
has passed its compile tests.

## Required Work

- Record the exact synaptic-canvas-dolt source commit and authoritative vector
  fixture before implementation changes the algorithm.
- Migrate PR #358's calculation as the starting point, correcting it to the
  one strict-UTF-8/universal-newline-normalized `TemplateSha256` contract.
- Implement exactly `calculate_hash` and `calculate_composition_hash`; keep any
  stream/file layering private and do not add resolver, filesystem, path-policy,
  cycle, or depth APIs to sc-sha.
- Implement the caller-supplied tagged manifest types, versioned injective
  framing, and structural manifest errors without discovering or reordering
  graph data.
- Add `ADR-SHA-001` and update `CLAUDE.md` Boundary Rules with new numbered
  rules covering `sc-sha` and `bindings/sc-sha-python` dependency constraints
  and the `ATM_HOME` prohibition. The amendment requires an explicit
  team-lead ruling and ADR sign-off before it is committed.
- Remove duplicate sc-composer hash ownership without introducing a second
  raw/text identity type.

## Explicit Code Samples

```rust
pub enum HashInput<'a> {
    TextFileBytes { utf8_file_bytes: &'a [u8] },
}

pub enum HashResult {
    Template(TemplateSha256),
}

pub fn calculate_hash(input: HashInput<'_>) -> Result<HashResult, ShaError>;
pub fn calculate_composition_hash(
    manifest: &ResolvedTemplateManifest,
) -> Result<CompositionSha256, CompositionError>;
```

## Deliverables

- A publishable workspace member `sc-sha` with no MiniJinja, CLI, ATM, Python,
  or filesystem-traversal dependency.
- Verified synaptic-canvas-dolt-compatible normalized file calculation and
  public Rust API shape; command hashing is not a claimed deliverable.
- The two-operation typed API and caller-supplied manifest framing required by
  M.2 and the atm-core consumer contract.
- A migrated sc-composer surface with no duplicate SHA implementation.
- `ADR-SHA-001`, the CLAUDE.md boundary amendment, and a reviewable
  validation-evidence package.

## Acceptance Criteria

- `[SHA-R1, SHA-N1]` All authoritative normalized-text vectors pass, including
  Unicode, non-BMP text, LF/CRLF, BOM, final-newline, empty, and invalid-UTF-8;
  output matches the recorded upstream commit and remains host-invariant.
- `[SHA-R2]` Compile tests prove there are exactly two public operations and
  that callers receive `TemplateSha256` and `CompositionSha256` through public
  accessors without private-field access; no resolver or filesystem surface is
  exported by sc-sha.
- `[SHA-N2, SHA-N3]` Manifest encoding tests prove version-tagged, framed,
  injective node/edge encoding and typed structural errors without graph
  discovery or policy enforcement in sc-sha.
- `[ADR-SHA-001]` The team-lead ruling and ADR sign-off approve the new
  `CLAUDE.md` Boundary Rules for `sc-sha` and `bindings/sc-sha-python`,
  including dependency constraints and the extended `ATM_HOME` prohibition.
- `[SHA-N3]` Missing/invalid inputs produce typed errors and no success digest;
  the compatibility fixture includes Rust output, expected lowercase hex, and
  representative `package_files.sha256` field mapping. M.2 adds Python output
  equality; M.1 does not claim that adapter evidence prematurely.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- compatibility-vector tests against the recorded upstream source commit
- `git diff --check`

## QA Handoff

Send team-lead the branch/worktree, parent integration commit,
exact targets, deleted-path evidence, requirement/ADR mapping, validation
output, compatibility fixtures, and any sc-lint findings. Team-lead opens the
PR and routes it to quality-mgr. M.1 is not complete until QA approval, merge,
and post-merge revalidation are recorded.

The phase plan's `sc-lint cleanup and QA routing` section is authoritative for
minor findings and dedicated `fix/` worktree routing; this sprint handoff must
include its finding evidence and parent commit.

## Test Fixtures
### `sc-sha` tests

The shared crate must test:

- all authoritative synaptic-canvas-dolt vectors;
- empty input, ordinary UTF-8, LF, CRLF, BOM, and no-final-newline cases when
  those are part of the verified contract;
- exact digest bytes and lowercase hex display;
- strict invalid-UTF-8 errors;
- manifest schema/domain tags;
- tagged local-path versus URL source encoding;
- length-delimited node/edge framing and adversarial injectivity cases for
  source values, node order, per-file hashes, edge order, and occurrences;
- duplicate-node, unknown-edge, unsupported-schema, and malformed-source errors;
- caller-supplied manifest composition vectors independent of resolver policy.

## This Sprint Does Not Close

Recursive sc-compose include discovery,
renderer loading changes, the Python wheel, or issue #360. It does not modify
atm-core or synaptic-canvas-dolt.
