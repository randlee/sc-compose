---
id: sprint-I.3
title: XML Block and Mixed-Content Extraction
phase: I
status: complete/accepted
branch: sprint/i-3-xml-block-mixed-content
worktree: ../sc-compose-worktrees/sprint/i-3-xml-block-mixed-content
target: develop
---

# Sprint I.3 — XML Block and Mixed-Content Extraction

## Purpose

Close GitHub issue #193 Gap 1. Extend the known-template XML adapter so a
placeholder occupying an element's content can recover a rendered block that
contains text and the contract-approved child markup. The implementation must
reuse I.2's raw matcher for candidate-value matching and retain XML
provenance.

## Dependencies and exact targets

- I.1 accepted XML block/mixed-content contract;
- I.2 accepted raw-text public matcher seam;
- `crates/sc-composer/src/extract/xml.rs`, XML path/source types, and shared
  extraction report code;
- CLI and Python extraction parity fixtures;
- `docs/requirements.md`, `docs/architecture.md`, ADR-0013 or the accepted
  ADR-0012 amendment, and the error registry.

## Deliverables

- Replace the current equal-child-count-only path for the approved full-content
  placeholder case with a bounded matcher over the rendered inner content.
- Preserve element occurrence paths, source kind, capture order, and
  ambiguity diagnostics for block captures.
- Use a deterministic serialization or text projection exactly as chosen by
  I.1; do not depend on incidental `quick_xml` formatting.
- Support realistic `<description>`, `<references>`, and `<workflow>` blocks,
  including multiline text and contract-approved inline child elements.
- Reject multiple placeholders in one block, dynamic names, unsupported
  control-flow output, mismatched static child structure, and captures that
  exceed existing input/occurrence limits.
- Add equivalent library, CLI, Python, positive corpus, and negative-boundary
  tests.

## Acceptance criteria

- The exact #193 Gap 1 shape recovers the block value without an unsupported
  child-structure error.
- Text-only block values, multiline values, escaped entities, repeated
  element names, and nested approved inline elements have stable evidence.
- A block placeholder cannot consume a sibling or silently hide a static
  mismatch; structural paths remain accurate.
- The public report and diagnostics are identical across Rust, CLI `--json`,
  and Python.
- Existing scalar XML behavior, malformed XML rejection, dotted-expression
  rejection, input limits, and ambiguity handling remain unchanged.

## Required validation

Use the [authoritative Phase I validation
checklist](phase-I-plan.md#authoritative-validation-checklist). Add a focused
corpus artifact containing the template, rendered input,
expected report, and the requirement/ADR trace for every case. Include a
review note explaining why each block is a realistic customer document. The
accepted I.3 corpus is
[`evidence/i-3-xml-block-mixed-content.json`](evidence/i-3-xml-block-mixed-content.json).

## Removal path

If block/mixed-content matching fails QA, remove the XML structural extension
and its fixtures while retaining I.2 raw mode and the pre-existing scalar XML
adapter. Do not leave a partial child-node fallback enabled.

## Out of scope

- dirty prefixes before the XML root (I.4);
- unmatched/truncated XML markup, multiple roots, post-root content, or
  unknown-template identification;
- loop/branch reconstruction;
- a second XML-only text matcher;
- treating every XML text node as a raw block when the template does not have
  the approved full-content placeholder form.
