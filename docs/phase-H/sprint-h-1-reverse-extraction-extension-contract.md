---
id: H.1
title: Reverse Extraction Format Contract
status: complete
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
- ADR-0011 remains authoritative for Phase-G behavior; the accepted H.1
  amendments govern the Phase-H extensions.

## Exact Targets

- `docs/requirements.md`
- `docs/architecture.md`
- `docs/adrs/0012-phase-h-reverse-extraction-extension-gates.md`
- `docs/phase-H/phase-H-plan.md`
- `docs/project-plan.md`
- `docs/error-code-registry.md`

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
  ADR-0012, the Phase-H plan, the project index, and the error-code registry so
  they agree with the six-sprint sequence, dependencies, non-goals, and exit
  gates.
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

## Accepted Format Decisions

The following decisions are normative for H.2, H.3, H.4, H.5, and H.6. A
dependent sprint may add tests and implementation detail, but may not reopen
these semantics.

### JSON

- The rendered input is exactly one JSON value. Objects and arrays are
  structurally matched; object key order is irrelevant, while array length and
  index order must match.
- Placeholders are supported only in JSON string values, either as a complete
  string or embedded with static prefix/suffix text. Captures are always
  rendered strings. Placeholders in object keys, numbers, booleans, `null`,
  array/object structure, or a complete non-string JSON value are rejected.
- Static non-string JSON values are valid structural anchors and are compared
  by parsed JSON value; they are never recovered as typed variables.
- A missing path, shape mismatch, malformed document, duplicate object key,
  unsupported placeholder shape, or repeated variable at distinct paths is a
  diagnostic failure. Repeated occurrences remain reviewable in the report,
  but the ambiguous variable is omitted from the recovered value map.
- Object paths use `JsonPathSegment::ObjectKey { key }`; array paths use
  `JsonPathSegment::ArrayIndex { index }`. The root has an empty path.

### YAML

- The rendered input is exactly one YAML document body. A template's own YAML
  frontmatter is skipped before matching; rendered YAML frontmatter is not
  treated as extraction input. YAML document streams containing more than one
  document are rejected.
- Mapping keys must be static scalar strings. Placeholders are supported only
  in string scalar values, either as a complete scalar or embedded with static
  prefix/suffix text. Captures remain rendered strings; placeholders in keys,
  typed scalars, null, aliases, anchors, tags, or collection structure are
  rejected.
- Static YAML scalars and collections may be used as structural anchors, but
  no source type is reconstructed from a captured spelling. Duplicate keys,
  aliases/anchors, unsupported tags, multiple documents, malformed YAML,
  missing paths, shape mismatches, and repeated-variable ambiguity are
  diagnostic failures.
- Mapping paths use `YamlPathSegment::MappingKey { key }`; sequence paths use
  `YamlPathSegment::SequenceIndex { index }`. The root has an empty path.

### TOML

- The rendered input is exactly one TOML document. Tables, nested tables,
  arrays, inline tables, and arrays of tables are structurally matched; table
  key order is irrelevant and array order/index is significant.
- Placeholders are supported only in TOML basic/literal string values, either
  as a complete value or embedded with static prefix/suffix text. Captures are
  rendered strings. Placeholders in keys, table names, integers, floats,
  booleans, datetimes, arrays/inline-table structure, or unsupported value
  syntax are rejected. TOML has no null value; a null-equivalent placeholder
  is therefore rejected as an unsupported value shape.
- Duplicate keys, malformed TOML, missing paths, shape mismatches, and
  repeated-variable ambiguity are diagnostic failures. Static non-string TOML
  values may anchor structure but are not recovered as typed variables.
- Table/key paths use `TomlPathSegment::TableKey { key }`; array and
  array-of-table occurrences use `TomlPathSegment::ArrayIndex { index }`. The
  root has an empty path.

## Accepted Parser and Public-Surface Decisions

- Rust uses the already-approved workspace libraries `serde_json` 1.x for
  JSON, `serde_yaml` 0.9 for YAML, and `toml` 0.8 for TOML. Adapters must wrap
  these parsers to enforce the duplicate-key, alias/tag, document-count, and
  rendered-string policies above; default parser behavior is not itself the
  contract.
- Rust owns parsing and matching through the typed `ExtractFormat` request.
  The CLI accepts `--format xml|json|yaml|toml` and passes the typed value to
  Rust; omitting the flag retains XML compatibility. Python exposes
  `extract_variables(..., format="xml|json|yaml|toml")` and delegates to the
  same Rust entry point. Neither adapter parses or matches independently.
- The canonical report aliases are:

  ```rust
  pub enum JsonPathSegment { ObjectKey { key: String }, ArrayIndex { index: usize } }
  pub enum JsonExtractionSource { StringValue }
  pub type JsonExtractionReport =
      ExtractionReport<JsonPathSegment, JsonExtractionSource>;

  pub enum YamlPathSegment { MappingKey { key: String }, SequenceIndex { index: usize } }
  pub enum YamlExtractionSource { StringScalar }
  pub type YamlExtractionReport =
      ExtractionReport<YamlPathSegment, YamlExtractionSource>;

  pub enum TomlPathSegment { TableKey { key: String }, ArrayIndex { index: usize } }
  pub enum TomlExtractionSource { StringValue }
  pub type TomlExtractionReport =
      ExtractionReport<TomlPathSegment, TomlExtractionSource>;
  ```

## Accepted Cross-Format Diagnostic Inventory

This inventory is normative and is mirrored in
[`docs/error-code-registry.md`](../error-code-registry.md). Every diagnostic
serializes through the existing `diagnostics[]` envelope with `severity`,
`code`, `message`, and optional `location`; recovery hints are serialized as
the existing diagnostic detail/recovery-hint field. The owning sprint is the
first implementation surface responsible for emitting and testing the code.

| Code | Category | Severity | Trigger and recovery hint | Owner | Serialized representation |
| --- | --- | --- | --- | --- | --- |
| `ERR_EXTRACT_FORMAT_UNSUPPORTED` | format-selection | error | Requested format is not enabled; select `xml`, `json`, `yaml`, or `toml`. | H.3 | `diagnostics[]` code plus request location |
| `ERR_EXTRACT_TEMPLATE_UNSUPPORTED` | template-syntax | error | Loop, branch, dynamic key, typed placeholder, or other unsupported expression; use a known-template scalar expression. | H.2/H.4/H.5 | `diagnostics[]` code plus template location |
| `ERR_EXTRACT_INPUT_LIMIT` | input-policy | error | Size/depth/occurrence limit exceeded; reduce input or split the request. | H.2/H.4/H.5 | `diagnostics[]` code plus input location |
| `ERR_EXTRACT_SECURITY_POLICY` | input-policy | error | Alias/tag or other disallowed parser feature encountered; remove the feature. | H.4 | `diagnostics[]` code plus source location |
| `ERR_EXTRACT_JSON_MALFORMED` | json-parse | error | JSON cannot be parsed as one value; correct the rendered JSON. | H.2 | `diagnostics[]` code plus parser location |
| `ERR_EXTRACT_JSON_DUPLICATE_KEY` | json-parse | error | An object repeats a key; remove the duplicate key. | H.2 | `diagnostics[]` code plus object-key location |
| `ERR_EXTRACT_JSON_PATH_MISSING` | json-structure | error | Known-template path is absent; render the expected object/array shape. | H.2 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_JSON_SHAPE_MISMATCH` | json-structure | error | Object/array or static value differs from the known template; restore the expected shape. | H.2 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_JSON_VALUE_UNSUPPORTED` | json-value-policy | error | Placeholder occurs in a key, non-string value, or structural position; use a string value. | H.2 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_JSON_AMBIGUOUS` | json-ambiguity | error | Variable occurs at multiple distinct JSON paths; disambiguate the template or use occurrences. | H.2 | `diagnostics[]` code plus occurrence location |
| `ERR_EXTRACT_YAML_MALFORMED` | yaml-parse | error | YAML cannot be parsed as one document; correct the rendered YAML body. | H.4 | `diagnostics[]` code plus parser location |
| `ERR_EXTRACT_YAML_DUPLICATE_KEY` | yaml-parse | error | A mapping repeats a key; remove the duplicate key. | H.4 | `diagnostics[]` code plus mapping location |
| `ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED` | yaml-policy | error | Alias or anchor is present; expand it into explicit content. | H.4 | `diagnostics[]` code plus node location |
| `ERR_EXTRACT_YAML_DOCUMENT_STREAM` | yaml-policy | error | More than one YAML document is present; provide one rendered document body. | H.4 | `diagnostics[]` code plus document location |
| `ERR_EXTRACT_YAML_PATH_MISSING` | yaml-structure | error | Known-template path is absent; render the expected mapping/sequence shape. | H.4 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_YAML_SHAPE_MISMATCH` | yaml-structure | error | Mapping/sequence or static scalar differs from the known template; restore the expected shape. | H.4 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_YAML_VALUE_UNSUPPORTED` | yaml-value-policy | error | Placeholder occurs in a key, typed scalar, null, tag, alias, or structure; use a string scalar. | H.4 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_YAML_AMBIGUOUS` | yaml-ambiguity | error | Variable occurs at multiple distinct YAML paths; disambiguate the template or use occurrences. | H.4 | `diagnostics[]` code plus occurrence location |
| `ERR_EXTRACT_TOML_MALFORMED` | toml-parse | error | TOML cannot be parsed as one document; correct the rendered TOML. | H.5 | `diagnostics[]` code plus parser location |
| `ERR_EXTRACT_TOML_DUPLICATE_KEY` | toml-parse | error | A table or document repeats a key; remove the duplicate key. | H.5 | `diagnostics[]` code plus key location |
| `ERR_EXTRACT_TOML_PATH_MISSING` | toml-structure | error | Known-template path is absent; render the expected table/array shape. | H.5 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_TOML_SHAPE_MISMATCH` | toml-structure | error | Table/array or static value differs from the known template; restore the expected shape. | H.5 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_TOML_VALUE_UNSUPPORTED` | toml-value-policy | error | Placeholder occurs in a key, non-string value, null-equivalent, or structure; use a string value. | H.5 | `diagnostics[]` code plus path location |
| `ERR_EXTRACT_TOML_AMBIGUOUS` | toml-ambiguity | error | Variable occurs at multiple distinct TOML paths; disambiguate the template or use occurrences. | H.5 | `diagnostics[]` code plus occurrence location |

`ERR_EXTRACT_INVALID_REQUEST`, `ERR_EXTRACT_MALFORMED`,
`ERR_EXTRACT_UNSUPPORTED`, and `ERR_EXTRACT_AMBIGUOUS` remain the existing
Phase-G XML/general report codes. They are not silently reused for the new
format-specific conditions above. H.2, H.4, and H.5 must add each accepted
code to the Rust diagnostic enum and binding/CLI serialization before using it.

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
    /// Byte span relative to `rendered_candidate`, when the ambiguity can be
    /// localized to the candidate value.
    pub span: Option<Range<usize>>,
    pub message: String,
}

pub(crate) struct RawTextMatch {
    pub captures: Vec<RawTextCapture>,
    pub ambiguity: Option<RawTextAmbiguity>,
}

pub(crate) enum RawTextMatchError {
    /// Request-scoped: the template contract is broken, so no partial
    /// recovery is possible for this extraction request.
    InvalidTemplate {
        span: Option<Range<usize>>,
        message: String,
    },
    /// Occurrence-scoped: the adapter may record the per-occurrence diagnostic
    /// and continue processing other candidate occurrences.
    StaticMismatch {
        span: Option<Range<usize>>,
        message: String,
    },
    /// Occurrence-scoped: the adapter may record the per-occurrence diagnostic
    /// and continue processing other candidate occurrences.
    AmbiguousDelimiter {
        span: Option<Range<usize>>,
        message: String,
    },
}

pub(crate) enum RawTextErrorScope {
    Request,
    Occurrence,
}

impl RawTextMatchError {
    pub(crate) const fn scope(&self) -> RawTextErrorScope {
        match self {
            Self::InvalidTemplate { .. } => RawTextErrorScope::Request,
            Self::StaticMismatch { .. } | Self::AmbiguousDelimiter { .. } => {
                RawTextErrorScope::Occurrence
            }
        }
    }
}

pub(crate) fn match_raw_text(
    input: RawTextMatchInput<'_>,
) -> Result<RawTextMatch, RawTextMatchError>;
```

Adapters map `RawTextMatchError` to the stable format-neutral extraction
diagnostics from H.1. A returned `ambiguity` is never silently converted into
a value; the adapter preserves the signal in the report or returns the
contracted ambiguity diagnostic. `InvalidTemplate` is request-scoped: the
template contract is broken and the adapter must fail the request without
partial recovery. `StaticMismatch` and `AmbiguousDelimiter` are
occurrence-scoped: the adapter maps each to its per-occurrence diagnostic and
may continue processing other candidate occurrences according to H.1's
malformed-input and ambiguity policy. Every `span` is relative to the
`rendered_candidate`; it is `None` when the failure cannot be localized to a
candidate byte range, such as a template-side invalid expression.
The `scope()` method is the programmatic propagation marker adapters must use;
the prose above explains the same invariant for non-Rust serializers.

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

- FR-16, architecture, ADR-0012, the Phase-H plan, the project index, and the
  error-code registry are
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
- `RawTextMatchError` and `RawTextAmbiguity` carry an optional structured
  candidate-relative span, and the contract explicitly distinguishes
  request-scoped template errors from occurrence-scoped match errors through
  the `RawTextMatchError::scope()` marker.
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
