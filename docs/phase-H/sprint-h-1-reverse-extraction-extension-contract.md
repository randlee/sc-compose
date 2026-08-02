---
id: H.1
title: Reverse Extraction Format Contract
status: planned
branch: sprint/h-1-reverse-extraction-extension-contract
worktree: ../sc-compose-worktrees/sprint/h-1-reverse-extraction-extension-contract
target: develop
---

# Sprint H.1 — Reverse Extraction Format Contract

## Goal

- This is a planning/design sprint, not a runtime delivery sprint.
- Convert the three in-scope issue #193 format gaps into one accepted,
  implementable Phase-H contract before any code sprint starts.
- Amend FR-16, the extraction architecture, and ADR-0012 without changing
  Phase-G runtime behavior.

## Hard Dependencies

- Phase G.1 through G.7 are complete on the `develop` baseline.
- The read-only issue #193 gap review is available as the source analysis.
- ADR-0011 remains authoritative until this sprint is accepted.

## Exact Targets

- `docs/requirements.md`
- `docs/architecture.md`
- `docs/adrs/0012-phase-h-reverse-extraction-extension-gates.md`
- `docs/phase-H/phase-H-plan.md`
- `docs/project-plan.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H1-D1 — Accept a normative disposition for JSON, YAML, and TOML, including
  the supported input subset and explicit rejection cases for each.
- H1-D2 — Define the generic report/path/source extension strategy and the
  public Rust, Python, and CLI format-selection shape without implementing it.
- H1-D3 — Define the malformed-input, duplicate-key, null/type, ambiguity,
  provenance, size-limit, security, and cross-format recovery-hint policies
  needed by H.2 through H.5, including the current XML-specific constructors
  in `crates/sc-composer/src/extract/error.rs`.
- H1-D4 — Amend H.4 and H.5 Exact Targets to include
  `crates/sc-composer/src/extract/error.rs`, and update FR-16, architecture,
  ADR-0012, the Phase-H plan, and the project index so they agree with the
  six-sprint sequence, dependencies, non-goals, and exit gates.
- H1-D5 — Define and confirm one shared raw-text matching core for value
  matching. It must reuse the established Exact-Match Delimiter Scanning,
  Longest-Match-First Template-Init Replacement, and Multi-Pass Brace-Count
  Delimiter Scheme decisions; H.2, H.4, and H.5 must delegate to this core
  rather than implement independent format-specific text matchers. The core
  must be structured so a future customer-facing raw-text or best-effort mode
  can reuse it without redesign. The design must identify which
  format-neutral matching logic moves out of the current XML implementation
  (including the `parse_value_segments`/capture path in `extract/xml.rs`) and
  which structural traversal, provenance, and diagnostics remain format-owned.

## Required Work

- Review each in-scope issue #193 reproduction and map it to a requirement, ADR rule,
  owning module, and planned sprint.
- Decide whether JSON placeholders are restricted to string values or may
  represent complete JSON values, and define object/array occurrence paths.
- Define YAML and TOML parser boundaries, including duplicate and typed-value
  behavior, while retaining rendered-string output.
- Define a format-neutral extraction error taxonomy and recovery-hint policy;
  existing XML-specific recovery text must not be copied into YAML or TOML
  diagnostics without an explicit contract decision.
- Define the shared raw-text matching boundary and its handoff from structured
  JSON/YAML/TOML parsing to value matching; do not design a customer-facing
  degraded or raw-text operating mode in this sprint.
- Inventory the current XML matcher and its regression tests, specifying the
  extraction seam that becomes shared. XML structural traversal and
  format-specific provenance stay in `xml.rs`; delimiter scanning, template
  segment parsing, static-prefix/suffix matching, capture boundaries, and
  adjacent-variable ambiguity handling become reusable raw-text operations.
- Build an error inventory covering every new JSON/YAML/TOML failure mode with
  its stable code, category, severity, recovery hint, owning surface, and
  serialized representation. H.4 and H.5 may not rely on an unlisted or
  provisional failure code.
- Record explicit non-closure for XML mixed-content extraction, XML dirty-prefix
  tolerance, best-effort/degraded parsing, customer-facing raw-text mode,
  unknown-template identification, loops, branches, typed recovery, and
  Jinja evaluation.

## Explicit Code Samples

The accepted design must make the eventual API shape unambiguous. The sample
is illustrative contract text, not an executable artifact:

```rust
pub enum ExtractFormat {
    Xml,
    Json,
    Yaml,
    Toml,
}

pub struct ExtractRequest<'a> {
    pub template: &'a str,
    pub rendered: &'a str,
    pub format: ExtractFormat,
    pub include: &'a [VariableName],
    pub exclude: &'a [VariableName],
}
```

The sprint must also name the format-specific path/source variants that
instantiate the existing generic `ExtractionReport` without creating a second
report model, and must identify the shared raw-text matching core used by all
three format adapters.

The shared-core contract must include a concrete Rust entry point. The
following signature is normative for the boundary; names may change only if
the same input/output/error guarantees remain explicit:

```rust
use std::ops::Range;

pub(crate) enum RawTextSegment<'a> {
    Static(&'a str),
    Variable(VariableName),
}

pub(crate) struct RawTextMatchInput<'a> {
    /// Segments produced from the template-side static prefixes/suffixes and
    /// variable expressions.
    pub segments: &'a [RawTextSegment<'a>],
    /// The candidate value slice identified by the format adapter.
    pub rendered_candidate: &'a str,
}

pub(crate) struct RawTextCapture {
    pub variable: VariableName,
    /// Byte span relative to `rendered_candidate`.
    pub span: Range<usize>,
    pub rendered_text: String,
}

pub(crate) struct RawTextAmbiguity {
    pub message: String,
}

pub(crate) struct RawTextMatch {
    pub captures: Vec<RawTextCapture>,
    pub ambiguity: Option<RawTextAmbiguity>,
}

pub(crate) enum RawTextMatchError {
    InvalidTemplate { message: String },
    StaticMismatch { message: String },
    AmbiguousDelimiter { message: String },
}

pub(crate) fn match_raw_text(
    input: RawTextMatchInput<'_>,
) -> Result<RawTextMatch, RawTextMatchError>;
```

Adapters map `RawTextMatchError` to the stable format-neutral extraction
diagnostics from H.1. A returned `ambiguity` is never silently converted into
a value; the adapter preserves the signal in the report or returns the
contracted ambiguity diagnostic.

The design must name the migration seam explicitly: the first implementation
sprint extracts the format-neutral operations from the current XML path into a
shared internal module, keeps XML behavior covered by its existing tests, and
then builds JSON/YAML/TOML adapters on that same seam. H.1 does not expose the
future raw-text mode or implement the refactoring itself.

## This Sprint Does Not Close

- No Rust, Python, CLI, parser, or test implementation.
- No change to Phase-G XML scalar behavior or shared delimiter decisions.
- No customer-facing best-effort/degraded-parse mode or cross-format raw-text
  mode; those are future-phase features.
- No claim that JSON, YAML, or TOML is supported before the relevant
  implementation sprint passes.
- No claim that XML mixed-content or dirty-prefix input is supported in Phase
  H; those are future-phase scope.

## Acceptance Criteria

- FR-16, architecture, ADR-0012, the Phase-H plan, and the project index are
  mutually consistent and link to the same six contiguous sprints.
- Every in-scope issue #193 gap has one disposition, one owner sprint, explicit
  supported/rejected behavior, and a testable contract.
- The accepted contract preserves rendered-string output, ambiguity safety,
  library/adapter boundaries, and fail-closed malformed-input handling.
- The shared raw-text matching core is explicitly defined as a reusable
  foundational layer, and H.2/H.4/H.5 can use it without inventing
  format-specific matching logic.
- The design identifies the XML-to-shared-core migration seam and preserves
  XML's existing behavior and regression coverage while moving only
  format-neutral matching operations.
- The concrete `match_raw_text` signature settles the candidate-slice input,
  capture-span output, ambiguity signal, and diagnostic error boundary before
  H.2 begins.
- Best-effort/degraded parsing and customer-facing raw-text mode are named
  future-phase non-goals, not Phase-H features or sprint work.
- The error inventory is complete for every planned JSON/YAML/TOML failure
  mode; each mode has a stable documented code and recovery mapping.
- H.2 through H.5 can implement their scope without inventing semantics in
  code or reopening H.1 decisions.
- The document explicitly states that this sprint produces no executable
  artifact.

## Required Validation

- `git diff --check`
- `rg -n "H\.[1-6]|ADR-0012|Phase-H" docs/requirements.md docs/architecture.md docs/adrs/0012-phase-h-reverse-extraction-extension-gates.md docs/phase-H docs/project-plan.md`
