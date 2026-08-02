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
- The prior research findings recorded in [phase-G-plan.md](phase-G-plan.md).

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
  types, an explicit `ExtractError` contract, and an extraction entry point
  that accepts in-memory text only.
- `G1-D3` — The contract distinguishes successful extraction, malformed input,
  unsupported syntax, and ambiguous structure without conflating an
  intentional boundary with a missing value.
- `G1-D4` — Unit tests cover request validation, include/exclude selection,
  empty selections, stable diagnostic serialization, and the string-value
  limitation before the XML matcher is implemented.
- `G1-D5` — Extend `repo_boundaries.rs` with a machine-checked Phase-G
  boundary gate. Its file-discovery walker must be extended—not merely
  layered with new assertions—to enumerate `crates/**/*.rs`,
  `bindings/python/src/**/*.rs`, and `bindings/python/python/**/*.py`; the
  existing crates-only/Rust-only scope is insufficient. The gate must inspect
  those production Rust/Python source imports and Cargo manifests to prove
  that production bindings do not import research-only extraction artifacts,
  `bindings/python` depends on `sc-composer` but not `sc-compose`,
  `sc-observability`, or ATM crates, and `sc-composer` does not depend on
  `bindings/python`.

## Required work

- Reuse existing `VariableName`, `InputValue`, diagnostic, and error
  conventions where they fit; do not create a parallel render-context model.
- Keep file I/O, CLI argument parsing, JSON envelopes, and exit-code mapping
  out of `sc-composer`.
- Define occurrence provenance sufficient for G.2 to distinguish repeated
  sibling paths; a tag name alone is not an acceptable identity.
- Define the generic `ExtractionOccurrence` and `ExtractionDiagnostic` shapes
  before G.2 specializes them for XML. G.2 must extend those shapes with XML
  path/source detail rather than introduce an incompatible second report
  contract.
- If one variable name appears at more than one distinct structural
  occurrence, classify the result as `ambiguous` and emit no entry for that
  variable in `ExtractionReport.values`, even when the rendered strings happen
  to match. Never silently overwrite one occurrence with another; differing
  rendered strings must be preserved in the occurrence diagnostics/evidence.
- Define whether a report with warnings is successful and how a caller
  distinguishes `unsupported`, `ambiguous`, `not_observed`, and malformed XML.
- Validate report confidence at construction: reject NaN, infinity, and values
  outside the closed `0.0..=1.0` range.
- Add an ADR only because prior research exposed a real product-contract gap;
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
    pub include: &'a [VariableName],
    pub exclude: &'a [VariableName],
}

pub struct ExtractionReport<P = OccurrencePathSegment, S = OccurrenceSource> {
    pub values: BTreeMap<VariableName, String>,
    pub occurrences: Vec<ExtractionOccurrence<P, S>>,
    pub confidence: f64,
    pub diagnostics: Vec<ExtractionDiagnostic>,
}

pub enum ExtractError {
    InvalidRequest { message: String },
    MalformedXml { diagnostic: ExtractionDiagnostic },
    UnsupportedSyntax { diagnostic: ExtractionDiagnostic },
    AmbiguousStructure { diagnostic: ExtractionDiagnostic },
}

pub struct ExtractionOccurrence<
    P = OccurrencePathSegment,
    S = OccurrenceSource,
> {
    pub variable: VariableName,
    pub path: Vec<P>,
    pub source: S,
    pub rendered_text: Option<String>,
}

pub enum OccurrencePathSegment {
    Node { label: String, ordinal: usize },
    Value { label: Option<String>, ordinal: usize },
}

pub enum OccurrenceSource {
    Named { kind: String, label: Option<String> },
}

pub struct ExtractionDiagnostic {
    pub code: String,
    pub kind: ExtractionDiagnosticKind,
    pub message: String,
    pub occurrence: Option<OccurrenceIndex>,
}

pub struct OccurrenceIndex(pub usize);

pub enum ExtractionDiagnosticKind {
    Unsupported,
    Ambiguous,
    NotObserved,
    Malformed,
}
```

The names are illustrative; the accepted public contract must preserve the
same boundaries and semantics. `confidence` is a report-level `f64` in the
closed range `0.0..=1.0`; it is not a per-occurrence score. Occurrence-level
trust is represented by path/source evidence and diagnostics. For the
same-variable conflicting-occurrence rule, each conflicting rendered string is
retained in its own `ExtractionOccurrence<P, S>::rendered_text`; the
`ExtractionDiagnostic::occurrence` index identifies the relevant entries,
while the variable is omitted from `values`.

## This sprint does not close

- XML parsing or value matching beyond model-level tests;
- the `sc-compose extract` command;
- JSON/Markdown output adapters or unknown-template identification;
- typed-value inference, loop reconstruction, or automatic edits;
- production use of prior research artifacts.

## Acceptance criteria

- The requirement and ADR agree on the supported XML subset, string-output
  semantics, error categories, and explicit non-goals.
- The library model compiles and is exported without adding CLI, filesystem,
  ATM, or network dependencies.
- Occurrence identity includes structural path/ordinal information and cannot
  represent only a tag-name lookup.
- Unit tests prove stable construction/serialization of the report and all
  contract-level error categories, including invalid confidence values.
- The named same-variable multi-occurrence rule is covered by a contract test;
  it cannot silently overwrite the `BTreeMap` value.
- `ExtractionOccurrence<P, S>` and `ExtractionDiagnostic` have explicit,
  compile-testable fields and category variants before G.2 begins. Their
  default parameters preserve the generic G.1 contract, and G.2's XML aliases
  instantiate the same types with XML path/source detail.
- The boundary walker itself enumerates `crates/**/*.rs`,
  `bindings/python/src/**/*.rs`, and `bindings/python/python/**/*.py`; the
  acceptance test fails if any of those roots disappear from discovery.
- `repo_boundaries.rs` machine-checks the Phase-G crate/source dependency
  boundaries described by G1-D5.
- No existing render, validation, or diagnostic behavior changes.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `git diff --check`
