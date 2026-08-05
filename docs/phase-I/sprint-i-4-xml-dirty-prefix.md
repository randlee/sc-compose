---
id: sprint-I.4
title: XML Dirty-Prefix Normalization
phase: I
status: complete
branch: sprint/i-4-xml-dirty-prefix
worktree: ../sc-compose-worktrees/sprint/i-4-xml-dirty-prefix
target: integrate/phase-i
---

# Sprint I.4 — XML Dirty-Prefix Normalization

## Purpose

Close GitHub issue #193 Gap 5. Allow a narrow, observable class of rendered
XML responses with non-XML text before the document root while preserving the
fail-closed malformed-input boundary.

## Dependencies and exact targets

- I.1 accepted preamble algorithm, warning policy, and diagnostic code;
- XML parser and extraction entry point in
  `crates/sc-composer/src/extract/xml.rs`;
- extraction diagnostics in `crates/sc-composer/src/diagnostics.rs`, exports,
  CLI/Python adapters, registry, and docs;
- #193 reproduction fixtures.

## Deliverables

- Add one rendered-input normalization step before XML parsing. It must
  identify a single valid root candidate and strip only the contract-approved
  leading preamble.
- Preserve XML declarations/comments/processing instructions according to
  I.1 and retain line/offset information needed by diagnostics.
- Emit the approved warning or recovery metadata whenever bytes are removed;
  do not make a successful repair invisible.
- Reject a malformed suffix, multiple roots, a second document, an unmatched
  or truncated declaration/comment/processing instruction, an ambiguous
  markup prefix, or XML content after the selected root. The ordinary
  malformed XML code remains the fallback.
- Add library, CLI, and Python parity tests and a fixture corpus with accepted
  and rejected prefixes.

## Acceptance criteria

- The exact #193 dirty-prefix reproduction extracts successfully after the
  approved preamble is removed.
- Plain text preambles, XML declarations, comments, whitespace, and permitted
  processing instructions have deterministic documented behavior.
- A prefix containing a second `<root>`, malformed markup, or an invalid
  suffix cannot be silently dropped.
- The report records the recovery warning/detail and still reports XML paths
  relative to the actual root.
- Clean XML output is byte-for-byte unaffected at the report/diagnostic level
  except for no recovery warning.

## Required validation

Use the [authoritative Phase I validation
checklist](phase-I-plan.md#authoritative-validation-checklist). Run the
focused cases through Rust, CLI, and Python and retain the input,
normalization decision, report, and diagnostic in reviewable evidence.
I.4 owns a combined regression fixture in which a rendered document has an
accepted dirty prefix and an I.3 full-content block/mixed-content placeholder;
the case must prove both normalizations happen in the documented order. If I.3
has not yet landed when I.4 is otherwise ready, record the fixture as a
phase-level integration test deferred until both sprint changes are present;
Phase I cannot close until it passes.

## Removal path

If the recovery policy cannot preserve the malformed-input boundary, remove
the normalization step, warning, and fixtures as one unit. Clean XML parsing
and the existing malformed diagnostic remain the fallback.

## Out of scope

- arbitrary HTML/XML repair, truncation recovery, or multiple-document merge;
- changing template parsing or accepting dirty template input;
- implementing XML block/mixed-content matching (I.3); the combined I.3
  regression is exercised here only because I.3 is now present on the target;
- unknown-template identification or raw-text mode design (I.1/I.2).
