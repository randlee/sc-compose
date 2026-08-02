---
id: G.7
title: Reject Dotted Extraction Expressions
status: planned
branch: sprint/g-7-dotted-expression-rejection
worktree: ../sc-compose-worktrees/sprint/g-7-dotted-expression-rejection
target: develop
---

# Sprint G.7 — Reject Dotted Extraction Expressions

## Goal

Close the G.6 adversarial finding `FUZZ-template-probe-dotted-expression`:
during known-template XML extraction, a dotted expression such as
`{{ user.name }}` is currently accepted as a literal variable identifier
(`"user.name"`) instead of being rejected. Team-lead/product-owner resolved
the underlying ambiguity: a dotted expression is object-field access, not a
literal variable name, and since Phase G's extraction feature supports only
the scalar (flat) variable subset with no object/nested-value extraction
capability, it must be rejected as unsupported syntax rather than accepted or
newly implemented as object-field extraction.

## Hard dependencies

- G.1 through G.6 are merged into the sprint baseline.
- `docs/phase-G/evidence/g-6-reverse-extract-campaign.json`'s
  `FUZZ-template-probe-dotted-expression` finding and its
  `requirement_follow_up` field are the originating evidence for this sprint.
- FR-16, ADR-0011, and this phase plan have been amended to state the
  dotted-expression contract decision before implementation begins.

## Exact targets

- `crates/sc-composer/src/extract/xml.rs` — `parse_value_segments`, the call
  site at the `VariableName::new(expression)` construction (around line 813
  as of commit `9c7ca1c`), is where the rejection check belongs. After a
  successful `VariableName::new` parse, reject any variable whose name
  contains `.` by returning `ExtractError::unsupported(...)`, which already
  maps to `DiagnosticCode::ErrExtractUnsupported`
  (`ERR_EXTRACT_UNSUPPORTED`) via `crates/sc-composer/src/extract/error.rs`.
  No new diagnostic code, error variant, or `ExtractError` constructor is
  needed.
- `crates/sc-composer/src/types.rs` — `VariableName::new` (lines 133-167) must
  **not** change. Its permissive grammar (`is_ascii_alphanumeric() ||
  matches!(ch, '_' | '-' | '.')`) is shared with
  `crates/sc-composer/src/validation.rs` for composition/rendering variable
  declaration and reference tracking, which is out of scope for this sprint
  and must not be affected.
- `crates/sc-composer/src/extract/tests.rs` — add the regression test using
  the exact fuzz-discovered minimized reproduction.
- `docs/phase-G/evidence/g-6-reverse-extract-campaign.json` — update the
  `FUZZ-template-probe-dotted-expression` finding's `status`,
  `classification`, and add a resolution note referencing this sprint, without
  altering the campaign's other findings, worker records, or validation
  history.

## What this sprint does not touch

- `bindings/python/src/functions.rs` (`extract_variables`) and
  `crates/sc-compose/src/commands/extract.rs` (the CLI `extract` command)
  both call into the shared `sc_composer::extract` entry point and inherit
  this fix automatically; neither needs an independent code change.
- `crates/sc-compose/src/commands/extract.rs`'s own `VariableName::new` calls
  (line 57) validate `--include`/`--exclude` filter names, not template
  expressions, and are unaffected by this sprint.
- Any other `VariableName::new` call site outside `crates/sc-composer/src/extract/`
  (var-file decoding, template-init, render-request variable parsing) is
  composition/rendering surface, not extraction, and is out of scope.

## Deliverables

- `G7-D1` — Reject any XML extraction expression that parses to a
  `VariableName` containing `.` with `ExtractError::UnsupportedSyntax`
  (`ERR_EXTRACT_UNSUPPORTED`), instead of accepting it as a literal variable
  name.
- `G7-D2` — Add a regression test in `crates/sc-composer/src/extract/tests.rs`
  using the G.6 minimized reproduction: template
  `<root><name>{{ user.name }}</name></root>`, rendered
  `<root><name>Ada</name></root>`, asserting extraction returns
  `ExtractError::UnsupportedSyntax` with code `ERR_EXTRACT_UNSUPPORTED`.
- `G7-D3` — Confirm (with a test or explicit note in the sprint report if a
  test already covers it) that ordinary scalar extraction, including
  variable names containing `_` and `-`, is unaffected by this change.
- `G7-D4` — Update `docs/phase-G/evidence/g-6-reverse-extract-campaign.json`'s
  `FUZZ-template-probe-dotted-expression` finding to record that the
  ambiguity was resolved by team-lead/product-owner decision and closed by
  this sprint, citing the amended FR-16/ADR-0011 language and this sprint doc.

## This sprint does not close

- object-field or nested-value extraction capability of any kind;
- any change to `VariableName`'s shared grammar or to composition/rendering
  behavior;
- new extraction output formats or Python/CLI-specific code changes beyond
  what the shared library fix already covers.

## Acceptance criteria

- `{{ user.name }}` (and any other dotted expression) in a known-template XML
  extraction request is rejected with `ExtractError::UnsupportedSyntax` /
  `ERR_EXTRACT_UNSUPPORTED`, deterministically, across Rust, Python, and CLI
  surfaces (verified once at the shared library level per the exact-targets
  scoping above).
- Non-dotted scalar variable extraction (including `_` and `-` in names)
  continues to work exactly as before.
- `VariableName::new` and `crates/sc-composer/src/validation.rs` are
  unmodified by this sprint's diff.
- The regression test reproduces the exact G.6 fuzz case and fails without the
  fix (verified by the developer before committing).
- The G.6 evidence artifact's `FUZZ-template-probe-dotted-expression` finding
  is updated to reflect resolution, and no other finding, worker record, or
  validation history in that artifact is altered.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
