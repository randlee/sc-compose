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

## Exact targets and deliverables

- `crates/sc-composer/src/extract/xml.rs:1-743`, especially `XmlElement`/`XmlNode`/`XmlDocument`, `parse_xml`, `decode_*`, `collect_expected_*`, `collect_template_occurrences`, `path_exists`, and `extract_xml`.
- Add private modules for document parsing/model utilities and occurrence/evidence collection; preserve existing `XmlPathSegment`, `XmlExtractionSource`, report aliases, and `extract_xml` behavior.
- Add characterization tests for malformed XML, dirty-prefix recovery, missing occurrences, attributes/text/element content, repeated siblings, limits, and unsupported syntax before moving code.

## Acceptance criteria

- All existing XML extraction values, paths, sources, confidence values, diagnostics, and error codes are identical.
- `xml_match.rs`, `xml_reject.rs`, and `xml_serialize.rs` remain compatible and are not rewritten as part of this sprint.
- Production-NLOC and largest-module evidence show reduced ownership concentration; no new public XML API is introduced.

## Required validation

Run the Phase K checklist: focused XML tests before and after the move, `cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo test --test extract_integration`.

## Dependencies and non-closure

Independent from K.2-K.8. This sprint does not add XML features, alter extraction semantics, or change the generic extraction contract.
