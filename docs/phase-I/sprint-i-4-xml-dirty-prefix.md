---
id: sprint-I.4
title: XML Dirty-Prefix Normalization
phase: I
status: planned
branch: feature/phase-i-4-xml-dirty-prefix
worktree: ../sc-compose-worktrees/feature/phase-i-4-xml-dirty-prefix
target: develop
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
- Reject a malformed suffix, multiple roots, a second document, an ambiguous
  markup prefix, or a prefix that would require general repair. The ordinary
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

```text
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
pytest -q bindings/python/tests
git diff --check
```

Run the focused cases through Rust, CLI, and Python and retain the input,
normalization decision, report, and diagnostic in reviewable evidence.

## Removal path

If the recovery policy cannot preserve the malformed-input boundary, remove
the normalization step, warning, and fixtures as one unit. Clean XML parsing
and the existing malformed diagnostic remain the fallback.

## Out of scope

- arbitrary HTML/XML repair, truncation recovery, or multiple-document merge;
- changing template parsing or accepting dirty template input;
- XML block/mixed-content matching (I.3);
- unknown-template identification or raw-text mode design (I.1/I.2).
