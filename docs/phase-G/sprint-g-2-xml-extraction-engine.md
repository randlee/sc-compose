---
id: G.2
title: Deterministic XML Extraction Engine
status: planned
branch: sprint/g-2-xml-extraction-engine
worktree: ../sc-compose-worktrees/sprint/g-2-xml-extraction-engine
target: develop
---

# Sprint G.2 — Deterministic XML Extraction Engine

## Goal

Implement the known-template XML extraction engine against the G.1 contract.
Replace the prototype's regex-plus-first-tag behavior with structural
occurrence matching that either returns the uniquely supported value or emits
an explicit unsupported/ambiguous result.

## Hard dependencies

- G.1's public report, occurrence, and diagnostic contract must be available.
- Existing `sc-composer` frontmatter and template-token semantics remain the
  source of truth; this sprint must not create a second renderer.

## Exact targets

- `crates/sc-composer/src/extract/xml.rs`
- `crates/sc-composer/src/extract/mod.rs`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `crates/sc-composer/tests/fixtures/reverse-extract/**`

## Deliverables

- `G2-D1` — Parse the declared reversible XML subset and associate each
  placeholder with a structural occurrence path, including sibling ordinal,
  attribute name, or text-node position.
- `G2-D2` — Extract scalar rendered strings from attributes and text nodes,
  preserving XML entity decoding and meaningful whitespace according to the
  documented contract.
- `G2-D3` — Correctly distinguish repeated sibling occurrences; the second
  occurrence must never alias the first merely because the tag names match.
- `G2-D4` — Detect and report unsupported filters, expressions, loops,
  conditionals, ambiguous namespaces, missing occurrences, and malformed XML
  without fabricating a value.
- `G2-D5` — Compute confidence only from matched structural/static evidence and
  expose warnings when confidence is insufficient; confidence must not turn an
  ambiguous extraction into success.
- `G2-D6` — Add deterministic unit and integration fixtures for attributes,
  text, static prefix/suffix, repeated siblings, XML entities, empty values,
  malformed XML, wrong structure, unsupported Jinja constructs, and the named
  `same-variable-conflicting-occurrences` case.

## Required work

- Prefer a parser/tokenizer and validated structural walk over regular
  expressions that cannot model nesting or occurrence identity.
- Reuse the library's existing frontmatter and variable-name rules.
- Treat rendered output as a string observation. Do not infer numbers,
  booleans, nulls, arrays, or objects from spelling alone.
- Make whitespace, XML namespaces, comments, declarations, and entity
  handling explicit in tests and diagnostics rather than relying on incidental
  `ElementTree` behavior.
- Keep extraction in `sc-composer`; no filesystem reads or CLI formatting.

## Explicit matching contract

```rust
pub enum ExtractionSource {
    Attribute { name: String },
    TextNode,
}

pub enum XmlPathSegment {
    Element { name: String, ordinal: usize },
    Attribute { name: String },
}

pub type XmlExtractionOccurrence =
    ExtractionOccurrence<XmlPathSegment, ExtractionSource>;

pub type XmlExtractionReport =
    ExtractionReport<XmlPathSegment, ExtractionSource>;
```

G.2 uses the generic `ExtractionOccurrence<P, S>` and
`ExtractionReport<P, S>` types from G.1 through the concrete
`XmlExtractionOccurrence` and `XmlExtractionReport` aliases above. It does not
declare a second `ExtractionOccurrence` or replace the report with an
incompatible XML-only type.
An occurrence is successful only when the template skeleton and rendered XML
identify exactly one path. A repeated tag without a stable ordinal/path is an
error, not permission to use the first element. If one variable name maps to
multiple distinct paths, the result is `ambiguous` and that variable is omitted
from `ExtractionReport.values`; no occurrence may silently overwrite another.

## This sprint does not close

- CLI arguments, JSON envelopes, or process exit codes;
- unknown-template directory scanning or confidence-based identification;
- loop/conditional reconstruction or typed-value recovery;
- JSON and Markdown rendered-output formats;
- changes to Minijinja rendering semantics.

## Acceptance criteria

- Every supported fixture returns the expected value and occurrence path.
- The repeated-sibling regression reproduces the prototype defect and passes
  with distinct values for distinct occurrences.
- The `same-variable-conflicting-occurrences` fixture returns an `ambiguous`
  diagnostic and no value for the conflicting variable; it never keeps only
  the first or last rendered string.
- Every unsupported or malformed fixture yields a stable diagnostic and no
  fabricated variable value.
- Entity decoding, empty scalar behavior, whitespace policy, and namespace
  policy are covered by tests.
- The engine is callable from `sc-composer` without file I/O or CLI imports,
  and existing workspace behavior remains unchanged.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo test -p sc-composer --test extract_integration`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `git diff --check`
