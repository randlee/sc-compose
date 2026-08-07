---
id: K.1
title: XML Extraction Decomposition
phase: K
status: complete
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

## Completion evidence

- Baseline commit: `76d6c7f` (`origin/develop` before the implementation
  change). The baseline characterization passed: 51/51
  `extract_integration` tests, `cargo test -p sc-composer extract::xml`
  completed with zero failures (the filter matches no named tests), formatting
  and diff checks passed, and the baseline workspace/clippy gates passed.
  Baseline `maturin develop` succeeded in the isolated
  `/tmp/sc-compose-k1-venv`; the binding suite passed 52/52 when run with the
  baseline worktree explicitly on `PYTHONPATH`.
- Post-move characterization is unchanged: 51/51 extraction integration tests
  passed, the XML test filter completed with zero failures, and the XML cases
  covering malformed input, dirty-prefix recovery, missing occurrences,
  attributes/text/element content, repeated siblings, limits, and unsupported
  syntax remained green.
- Ownership evidence uses a simple nonblank, non-comment Rust-line count. The
  baseline `xml.rs` was 743 lines / 670 counted lines in one module. After the
  move, `xml.rs` is 368 lines / 321 counted lines; `xml_model.rs` is 261 / 238;
  and `xml_evidence.rs` is 138 / 126. The aggregate is 767 lines / 685 counted
  lines, while the largest owner fell from 670 to 321 counted lines. The new
  modules are private and keep parsing/model and occurrence/evidence ownership
  separate.
- Public-surface and forbidden-path review passed: `XmlPathSegment`,
  `XmlExtractionSource`, report aliases, and `extract_xml` remain in `xml.rs`;
  `xml_match.rs`, `xml_reject.rs`, and `xml_serialize.rs` were not modified;
  no XML source path was deleted or renamed.
- Post-move validation passed: `cargo test -p sc-composer --test
  extract_integration`, `cargo test -p sc-composer extract::xml`, `cargo fmt
  --all --check`, `git diff --check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test --workspace`, `maturin develop`,
  and `pytest bindings/python/tests` (52/52 with the rebuilt worktree binding
  selected explicitly).

## Dependencies and non-closure

Independent from K.2-K.8. This sprint does not add XML features, alter extraction semantics, or change the generic extraction contract.
