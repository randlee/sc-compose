---
id: sprint-I.1
title: Contract, Raw-Text Semantics, and Traceability
phase: I
status: complete
branch: sprint/i-1-contract-and-traceability
worktree: ../sc-compose-worktrees/sprint/i-1-contract-and-traceability
target: integrate/phase-i
---

# Sprint I.1 — Contract, Raw-Text Semantics, and Traceability

## Purpose

Freeze the product and architecture contract before any runtime change. This
is a planning/design sprint: it creates no executable implementation and must
not claim that raw text, XML mixed content, dirty-prefix recovery, loop
built-ins, or YAML merge-key support already exists.

## Inputs and dependencies

- Phase G FR-16 and ADR-0011;
- Phase H FR-16 boundary, ADR-0012, shared `match_raw_text` design, and H.8
  closure evidence;
- current XML matcher in `crates/sc-composer/src/extract/xml.rs`;
- current token discovery in `crates/sc-composer/src/validation.rs`;
- current var-file decoder in `crates/sc-compose/src/var_file.rs` and YAML
  conversion in `crates/sc-composer/src/types.rs`;
- GitHub issues #193, #167, and #166, including their reproductions.

The decisions below are accepted by I.1. I.2 through I.6 may implement them;
I.4, I.5, and I.6 remain independent of I.2 after this gate.

## Decisions to record

### Raw-text mode

Define one format-neutral known-template operation over in-memory text. The
public selector is `format="raw"` for Rust, CLI, and Python, with a stable
report shape shared with the existing extraction formats. It:

- matches literal template text and `{{ variable }}` segments using the H
  shared matcher;
- supports Markdown and arbitrary text documents without a structural parser;
- retains occurrence order, variable names, rendered values, and confidence;
- rejects unsupported statements, ambiguous adjacent captures, and malformed
  template delimiters with stable diagnostics;
- never identifies an unknown template, executes Jinja, or infers source types.

Raw occurrences use `RawPathSegment` with zero-based byte offsets into the
rendered source and one-based line/column coordinates at the start of the
captured span. `include` selects only named variables when non-empty; `exclude`
removes named variables from values and occurrences; overlap or duplicate
filters fail with `ERR_EXTRACT_INVALID_REQUEST`. Excluded variables still
participate in static matching so they cannot change the interpretation of
neighboring captures.

The frozen interface must be expressed against the existing Rust report model,
not as an informal new parallel model. The minimum shape is:

```rust
pub enum ExtractFormat { Xml, Json, Yaml, Toml, Raw }

pub enum ExtractionPathSegment {
    Xml(XmlPathSegment), Json(JsonPathSegment), Yaml(YamlPathSegment),
    Toml(TomlPathSegment), Raw(RawPathSegment),
}

pub enum ExtractionSource {
    Xml(XmlExtractionSource), Json(JsonExtractionSource),
    Yaml(YamlExtractionSource), Toml(TomlExtractionSource),
    Raw(RawExtractionSource),
}

pub struct RawPathSegment {
    pub byte_start: usize, pub byte_end: usize,
    pub line: usize, pub column: usize,
}

pub enum RawExtractionSource { TextSpan }

pub fn extract(
    request: &ExtractRequest<'_>,
) -> Result<ExtractionReport<ExtractionPathSegment, ExtractionSource>, ExtractError>;
```

The CLI maps `ExtractFormatArg::Raw` to `ExtractFormat::Raw` and serializes
`raw` in help, text output, and JSON. Python accepts `format="raw"` and extends
the generated `Literal["xml", "json", "yaml", "toml", "raw"]` annotation;
both wrappers call the Rust entry point above. The stable raw-mode diagnostic
set is this typed list, with no adapter-local substitutes:

```rust
const RAW_MODE_CODES: &[DiagnosticCode] = &[
    DiagnosticCode::ErrExtractInvalidRequest,
    DiagnosticCode::ErrExtractTemplateUnsupported,
    DiagnosticCode::ErrExtractAmbiguous,
    DiagnosticCode::WarnExtractLowConfidence,
];
```

`ERR_EXTRACT_INVALID_REQUEST` covers empty/contradictory filters,
`ERR_EXTRACT_TEMPLATE_UNSUPPORTED` covers statements and malformed delimiter
forms outside the raw subset, `ERR_EXTRACT_AMBIGUOUS` covers non-unique
captures, and `WARN_EXTRACT_LOW_CONFIDENCE` covers a report that is returned
with degraded evidence. I.1 must either retain these existing codes or record
an explicit ADR/registry amendment before I.2 changes the list.

### XML block and mixed content

The accepted subset for I.3 is a full element-content placeholder that may
capture rendered text and a deterministic serialization of allowed child
markup. Multiple variables, dynamic element names, control-flow
reconstruction, unmatched/truncated markup, multiple roots, and post-root
content remain unsupported. The matcher uses canonical child serialization
with stable element names, attributes, text, and child order; incidental
parser formatting is not part of the value. Description, references, and
workflow blocks from #193 are the required realistic examples.

### Dirty-prefix recovery

The accepted rendered-only preamble consists of UTF-8 text and whitespace
before one XML document, plus complete XML comments and processing
instructions in the retained prolog. An XML declaration is retained only when
it is the first construct in that retained prolog. The normalizer removes only
bytes before the selected root and emits `WARN_EXTRACT_DIRTY_PREFIX_STRIPPED`
with the removed byte span. It rejects unmatched/truncated markup in the
discarded prefix, malformed suffixes, multiple roots, second documents,
post-root content, and DTDs; it is not an XML repair parser.

### Loop context

The implicit names available only while an active Jinja `for` scope is being
scanned are `loop`, `loop.index`, `loop.index0`, `loop.revindex`,
`loop.revindex0`, `loop.first`, `loop.last`, `loop.length`, `loop.depth`,
`loop.depth0`, and the `loop.cycle(...)` call form. Nested scopes push and pop
independently. A caller variable named `loop` outside a `for` remains subject
to ordinary undeclared-variable policy; arbitrary dotted names are never
implicit.

### YAML merge keys

The policy for `<<` in JSON/YAML var-files is fail-closed rejection with
`ERR_CONFIG_VARFILE` and an actionable message. I.6 must detect merge-key
syntax before tagged-value unwrapping; it must not partially expand, silently
drop inherited fields, or change JSON behavior. Explicit mappings remain the
portable recovery. Merge expansion is deferred until a separate requirement
specifies precedence, alias cycles, and limits.

## Required documentation changes

- Amend `docs/requirements.md` with new FR numbers for raw text, XML
  block/prefix behavior, loop built-ins, and the var-file merge-key policy.
- Amend `docs/architecture.md` with ownership, report/path semantics, the
  raw-text seam, prefix normalization boundary, validation scope model, and
  var-file policy.
- Create and accept [ADR-0013](../adrs/0013-phase-i-raw-text-and-input-safety.md)
  for the Phase-I extraction and input-safety decisions.
- Add any new stable diagnostic to `docs/error-code-registry.md`, including
  trigger, recovery, owner, and serialized shape.
- Update `docs/project-plan.md`, the Phase I plan, and the Phase H future-work
  references so no document claims these features are still unplanned once
  the contract is accepted.

## Acceptance criteria

- The four issue gaps map one-to-one to I.3-I.6, and the extra raw-text mode
  is explicitly labeled product-directed scope.
- Rust, CLI, and Python ownership is unambiguous; no adapter owns a second
  matcher.
- Every accepted case has a positive example and every rejection boundary has
  a diagnostic and recovery expectation.
- The plan states that independent sprint QA may run in parallel and contains
  no undocumented sequential QA gate.
- Requirements, architecture, ADR, registry, project plan, and sprint docs
  use the same names, selectors, codes, and scope limits.
- `git diff --check` passes and the documentation review can be performed
  without reading implementation patches.

## Decision record

The accepted decisions are recorded in
[ADR-0013](../adrs/0013-phase-i-raw-text-and-input-safety.md) and the linked
FR/architecture/registry amendments. The sprint closes the contract gate; it
does not add runtime support.

## Required validation

This sprint is documentation/design only. The diff-scope gate is explicit:

- `git diff --name-only origin/integrate/phase-i...HEAD` must contain only
  `docs/` files; the sprint branch is cut from and targets `integrate/phase-i`
  (including any requirements, architecture, ADR, registry, project-plan, or
  Phase-I files required by this sprint);
- the same file list must contain no `crates/`, `bindings/`, `tests/`,
  `site/`, generated artifact, or executable-source path;
- `git diff --check` must pass; and
- the plan QA record must show that no Rust, CLI, or Python implementation was
  introduced while I.1's contract remains unaccepted.

## Removal path

If the contract is rejected, remove this sprint's proposed FR/ADR/registry
amendments and keep the Phase-H future-work boundary authoritative. No runtime
artifact is created by this sprint.

## Explicit non-goals

- writing extraction or validation code;
- implementing YAML merge expansion without the contract decision;
- changing the Phase-H shipped behavior before a later implementation sprint;
- unknown-template identification, arbitrary Jinja execution, or source-type
  reconstruction.
