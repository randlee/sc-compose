---
id: K.1
title: XML Extraction Decomposition
phase: K
status: planned
branch: sprint/k-1-xml-extraction-decomposition
worktree: ../sc-compose-worktrees/sprint/k-1-xml-extraction-decomposition
target: integrate/phase-k
---

# Sprint K.1 — XML Extraction Decomposition

## Purpose and evidence

Issue #311 ranks `crates/sc-composer/src/extract/xml.rs` at 2.35/10 with CCN 13 and 672 NLOC. The file already has `xml_match`, `xml_reject`, and `xml_serialize` seams, but still owns document parsing/model lifetime handling, occurrence/evidence collection, and top-level extraction orchestration. This sprint makes those ownership boundaries explicit without changing the supported XML subset.

## Goal

Produce a production-ready private decomposition of the XML parser/model and
occurrence/evidence ownership while preserving the existing extraction
contract.

## Required work

- Record the focused characterization result against the Phase K baseline
  before moving implementation code.
- Implement only the seams listed under Exact targets and deliverables, retain
  the existing public/crate-visible paths, and rerun the characterization
  suite after the move.
- Record ownership and production-NLOC evidence and complete every command in
  Required validation before claiming closure.

## Hard dependencies

The hard dependencies are this sprint's plan-gate approval and
`integrate/phase-k` as the merge-forward target. There is no hard dependency on
another Phase K sprint.

## Production-ready expectation

Every deliverable listed below must land at production-ready quality for this
sprint's behavior-preserving scope. Partial module movement, test-only work,
or an unmeasured ownership split cannot satisfy the acceptance criteria.

## Exact targets and deliverables

- `crates/sc-composer/src/extract/xml.rs`, specifically `XmlElement`/
  `XmlNode`/`XmlDocument`, `parse_xml`, `decode_*`, `collect_expected_*`,
  `collect_template_occurrences`, `path_exists`, and `extract_xml`.
- Add private modules for document parsing/model utilities and occurrence/evidence collection; preserve existing `XmlPathSegment`, `XmlExtractionSource`, report aliases, and `extract_xml` behavior.
- Add characterization tests for malformed XML, dirty-prefix recovery, missing occurrences, attributes/text/element content, repeated siblings, limits, and unsupported syntax before moving code.

## Planned seam

The existing extraction entry point remains the only caller-facing boundary;
the proposed private helpers must preserve these signatures while ownership
is split:

```rust
pub(crate) fn extract_xml(
    request: &ExtractRequest<'_>,
) -> Result<XmlExtractionReport, ExtractError>;
fn parse_xml(source: &str) -> Result<XmlDocument, ExtractError>;
fn collect_expected_evidence(
    root: &XmlElement,
    evidence: &mut Evidence,
) -> Result<(), ExtractError>;
```

The exact private module names may differ, but `XmlPathSegment`,
`XmlExtractionSource`, the report aliases, and `extract_xml` remain in their
existing public/crate-visible paths. No path is deleted.

## Acceptance criteria

- All existing XML extraction values, paths, sources, confidence values, diagnostics, and error codes are identical.
- `xml_match.rs`, `xml_reject.rs`, and `xml_serialize.rs` remain compatible and are not rewritten as part of this sprint.
- Production-NLOC and largest-module evidence show reduced ownership concentration; no new public XML API is introduced.
- No existing XML source path is deleted or renamed; any new module is private
  and is removed from the sprint diff if characterization does not support the
  split.

## Required validation

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-composer --test extract_integration`
- `cargo test -p sc-composer extract::xml`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `maturin develop`
- `pytest bindings/python/tests`

Run the full focused list, including the Python commands, before the move and
again after the move. Record the unchanged public surface/diff review and
before/after production-NLOC evidence.

## Dependencies and non-closure

Independent from K.2-K.8. This sprint does not add XML features, alter extraction semantics, or change the generic extraction contract.
