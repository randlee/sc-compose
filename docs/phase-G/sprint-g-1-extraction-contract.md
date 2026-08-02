---
id: G.1
title: Extraction Contract and Analysis Model
status: planned
branch: sprint/g-1-extraction-contract
worktree: ../sc-compose-worktrees/sprint/g-1-extraction-contract
target: develop
---

# Sprint G.1 — Extraction Contract and Analysis Model

## Goal

Define the production reverse-extraction contract and establish the pure
library data model that later XML matching and CLI work will consume. This is
an implementation sprint with a small, compile-tested model surface; it does
not claim that extraction works before G.2.

## Hard dependencies

- Phase G plan and the completed Phase-F baseline.
- `docs/requirements.md`, `docs/architecture.md`, existing diagnostics, and
  the `sc-composer` public-value model.
- The prototype findings recorded in [phase-G-plan.md](phase-G-plan.md).

## Exact targets

- `crates/sc-composer/src/extract/mod.rs`
- `crates/sc-composer/src/extract/error.rs`
- `crates/sc-composer/src/lib.rs`
- `crates/sc-composer/src/types.rs` only when required to reuse an existing
  public value/name type
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-compose/tests/repo_boundaries.rs`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/adrs/0011-reverse-extract-known-template-contract.md`
- `docs/adrs/README.md`

## Deliverables

- `G1-D1` — A versioned requirement and ADR define reverse extraction as a
  known-template, XML-first feature with rendered-string outputs and explicit
  non-goals for unknown-template identification, loops, branches, JSON,
  Markdown, and type reconstruction.
- `G1-D2` — `sc-composer` exposes pure request/report/occurrence/diagnostic
  types and an extraction entry point that accepts in-memory text only.
- `G1-D3` — The contract distinguishes successful extraction, malformed input,
  unsupported syntax, and ambiguous structure without conflating an
  intentional boundary with a missing value.
- `G1-D4` — Unit tests cover request validation, include/exclude selection,
  empty selections, stable diagnostic serialization, and the string-value
  limitation before the XML matcher is implemented.
- `G1-D5` — Extend `repo_boundaries.rs` with a machine-checked Phase-G
  boundary gate. It must inspect production Rust/Python source imports and
  Cargo manifests to prove that production bindings do not import
  `prototype/reverse_extract`, `bindings/python` depends on `sc-composer` but
  not `sc-compose`, `sc-observability`, or ATM crates, and `sc-composer` does
  not depend on `bindings/python`.

## Required work

- Reuse existing `VariableName`, `InputValue`, diagnostic, and error
  conventions where they fit; do not create a parallel render-context model.
- Keep file I/O, CLI argument parsing, JSON envelopes, and exit-code mapping
  out of `sc-composer`.
- Define occurrence provenance sufficient for G.2 to distinguish repeated
  sibling paths; a tag name alone is not an acceptable identity.
- If one variable name appears at more than one distinct structural
  occurrence, classify the result as `ambiguous` and emit no entry for that
  variable in `ExtractionReport.values`, even when the rendered strings happen
  to match. Never silently overwrite one occurrence with another; differing
  rendered strings must be preserved in the occurrence diagnostics/evidence.
- Define whether a report with warnings is successful and how a caller
  distinguishes `unsupported`, `ambiguous`, `not_observed`, and malformed XML.
- Add an ADR only because the prototype exposed a real product-contract gap;
  do not create separate ADRs for each implementation detail.

## Explicit code sample

```rust
pub enum ExtractFormat {
    Xml,
}

pub struct ExtractRequest<'a> {
    pub template: &'a str,
    pub rendered: &'a str,
    pub format: ExtractFormat,
    pub include: &'a [String],
    pub exclude: &'a [String],
}

pub struct ExtractionReport {
    pub values: BTreeMap<VariableName, String>,
    pub occurrences: Vec<ExtractionOccurrence>,
    pub confidence: f64,
    pub diagnostics: Vec<ExtractionDiagnostic>,
}
```

The names are illustrative; the accepted public contract must preserve the
same boundaries and semantics.

## This sprint does not close

- XML parsing or value matching beyond model-level tests;
- the `sc-compose extract` command;
- JSON/Markdown output adapters or unknown-template identification;
- typed-value inference, loop reconstruction, or automatic edits;
- production use of `prototype/reverse_extract`.

## Acceptance criteria

- The requirement and ADR agree on the supported XML subset, string-output
  semantics, error categories, and explicit non-goals.
- The library model compiles and is exported without adding CLI, filesystem,
  ATM, or network dependencies.
- Occurrence identity includes structural path/ordinal information and cannot
  represent only a tag-name lookup.
- Unit tests prove stable construction/serialization of the report and all
  contract-level error categories.
- The named same-variable multi-occurrence rule is covered by a contract test;
  it cannot silently overwrite the `BTreeMap` value.
- `repo_boundaries.rs` machine-checks the Phase-G crate/source dependency
  boundaries described by G1-D5.
- No existing render, validation, or diagnostic behavior changes.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `git diff --check`
