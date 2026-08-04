# ADR-0013: Phase-I Raw Text and Input-Safety Contract

## Status

Accepted

## Context

Phase H extracted a shared format-neutral raw-text matcher but deliberately
did not expose it as a customer-facing mode. Issue #193's XML block and
dirty-prefix findings, issue #167's loop-context validation finding, and issue
#166's YAML merge-key var-file finding require an explicit contract before
runtime work begins. The product's first practical raw-text use case is known
template extraction from Markdown and other text documents.

## Decision

Phase I.1 accepts the following contract.

### Raw-text extraction

Raw mode is a known-template, in-memory extraction format selected as
`ExtractFormat::Raw`, CLI `--format raw`, or Python `format="raw"`. It reuses
the H shared matcher for literal text and `{{ variable }}` segments. It does
not identify unknown templates, execute Jinja, reconstruct loops, infer source
types, or parse a structured document.

The public Rust bridge extends the existing generic report model:

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

Raw byte offsets are zero-based and half-open into the rendered source; line
and column are one-based at the span start. Include/exclude filters retain the
existing request semantics: include selects, exclude removes, overlap and
duplicates are invalid, and filtered variables still participate in matching.
The stable raw diagnostic set is:

```rust
const RAW_MODE_CODES: &[DiagnosticCode] = &[
    DiagnosticCode::ErrExtractInvalidRequest,
    DiagnosticCode::ErrExtractTemplateUnsupported,
    DiagnosticCode::ErrExtractAmbiguous,
    DiagnosticCode::WarnExtractLowConfidence,
];
```

The CLI and Python adapters map to this Rust entry point and do not implement
independent matching.

### XML block and mixed content

A known XML template may use one full element-content placeholder. The value
may contain text and approved child markup. Matching uses deterministic
canonical child serialization (element names, attributes, text, and child
order), not incidental parser formatting. Multiple placeholders in one block,
dynamic names, control-flow reconstruction, unmatched/truncated markup,
multiple roots, post-root content, and unknown-template identification remain
unsupported.
I.3 reports `ERR_EXTRACT_XML_CHILD_STRUCTURE_MISMATCH` for rendered child markup
outside the approved structure, `ERR_EXTRACT_XML_CONTROL_FLOW_UNSUPPORTED` for
unsupported control-flow reconstruction, and `ERR_EXTRACT_XML_DYNAMIC_ELEMENT_NAME`
for dynamic element names.

### XML dirty-prefix recovery

Only rendered XML is normalized. A leading UTF-8 text/whitespace preamble is
accepted before one XML document. Complete comments and processing
instructions in the retained prolog are allowed; an XML declaration is
retained only when first in that retained prolog. The normalizer removes only
bytes before the selected root and emits
`WARN_EXTRACT_DIRTY_PREFIX_STRIPPED` with the removed span.

Unmatched or truncated markup in the discarded prefix, malformed suffixes,
multiple roots, second documents, post-root content, and DTDs are rejected;
the feature is not a general XML repair parser.

### Jinja loop context

During strict token discovery, the following names are implicit only inside an
active `for` scope: `loop`, `loop.index`, `loop.index0`, `loop.revindex`,
`loop.revindex0`, `loop.first`, `loop.last`, `loop.length`, `loop.depth`,
`loop.depth0`, and `loop.cycle(...)`. Nested scopes are independent. A
reference to `loop` outside a `for` remains subject to ordinary undeclared
variable policy, and arbitrary dotted names are not implicit.

### YAML merge keys in var-files

YAML merge keys (`<<`) in JSON/YAML var-files are rejected at the var-file
boundary with `ERR_CONFIG_VARFILE` before tagged-value unwrapping. The
diagnostic identifies the source line and column and directs callers to write
explicit mappings. Partial expansion and silent loss of inherited fields are
forbidden. A future merge-expansion feature must first specify precedence,
nested aliases, cycles, and resource limits.

## Consequences

- I.2 can implement raw mode against one frozen Rust/report/wrapper shape.
- I.3 and I.4 have explicit XML boundaries and a combined integration case.
- I.5 can scope exemptions without making `loop` globally implicit.
- I.6 can fail closed without changing valid JSON/YAML object behavior.
- `WARN_EXTRACT_DIRTY_PREFIX_STRIPPED` is reserved in the diagnostic registry;
  I.4 owns its runtime enum and cross-surface emission.
- This ADR adds no runtime implementation; the I.1 diff-scope gate remains
  docs-only.

## References

- [Phase I plan](../phase-I/phase-I-plan.md)
- [Sprint I.1](../phase-I/sprint-i-1-contract-and-traceability.md)
- [ADR-0012](0012-phase-h-reverse-extraction-extension-gates.md)
- [GitHub issue #193](https://github.com/randlee/sc-compose/issues/193)
- [GitHub issue #167](https://github.com/randlee/sc-compose/issues/167)
- [GitHub issue #166](https://github.com/randlee/sc-compose/issues/166)
