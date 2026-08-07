---
id: K.4
title: Diagnostic Schema and Envelope
phase: K
status: complete
branch: sprint/k-4-diagnostics-schema
worktree: ../sc-compose-worktrees/sprint/k-4-diagnostics-schema
target: integrate/phase-k
---

# Sprint K.4 — Diagnostic Schema and Envelope

## Purpose and evidence

Issue #311 ranks `crates/sc-composer/src/diagnostics.rs` at 3.90/10 and reports 38% duplication. The module combines the stable code enum/string mapping, filesystem error classification, diagnostic record construction, and generic serialized envelopes. These are distinct compatibility boundaries and should be separated without changing the wire schema.

## Goal

Produce a production-ready private decomposition of diagnostic code/schema,
filesystem classification, records, and envelopes without changing the
serialized protocol.

## Required work

- Record the baseline serialization and classification characterization before
  moving implementation code.
- Implement only the seams listed under Exact targets and deliverables, retain
  existing exports and Python consumers, and rerun the characterization suite
  after the move.
- Record ownership and production-NLOC evidence and complete every command in
  Required validation before claiming closure.

## Hard dependencies

The hard dependencies are this sprint's plan-gate approval and
`integrate/phase-k` as the merge-forward target. K.5 and K.6 should follow K.4
when practical, but are not hard source-level dependencies when exports remain
stable.

## Production-ready expectation

Every deliverable listed below must land at production-ready quality for this
sprint's behavior-preserving scope. Partial module movement, test-only work,
or an unmeasured ownership split cannot satisfy the acceptance criteria.

## Exact targets and deliverables

- `crates/sc-composer/src/diagnostics.rs`, including `DiagnosticCode`,
  `as_str`, filesystem classification, `Diagnostic`, constructors, and
  `DiagnosticEnvelope<T>`.
- Create private schema, filesystem, record, and envelope modules behind the existing `crate::diagnostics` and crate-root re-exports.
- Add or strengthen direct unit and JSON-CLI characterization tests for every
  serialized code/severity spelling, envelope shape, path/line/column
  behavior, include-chain ordering, and filesystem classification before
  moving code.

## Planned seam

The stable diagnostic types remain at their current crate paths. The split may
move implementation behind those paths, but must preserve these signatures:

```rust
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: DiagnosticCode,
    pub message: String,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub include_chain: Vec<PathBuf>,
}

pub struct DiagnosticEnvelope<T> {
    pub schema_version: String,
    pub payload: T,
    pub diagnostics: Vec<Diagnostic>,
}
```

`DiagnosticCode`, `DiagnosticSeverity`, `Diagnostic`, and
`DiagnosticEnvelope<T>` remain re-exported from `crate::diagnostics`; no
serialized field, enum spelling, or source path is deleted or renamed.

## Acceptance criteria

- `DiagnosticCode::as_str`, serde names, schema version, JSON envelope fields, constructor defaults, and filesystem classifications are byte-for-byte compatible.
- No diagnostic code is added, removed, renamed, or repurposed; no caller changes are required.
- Duplication evidence is recorded by category, not by speculative line-count reduction alone.
- No diagnostic type is made private and no new schema version is introduced.

## Required validation

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-composer diagnostics::tests`
- `cargo test -p sc-compose --test json_cli -- diagnostic`
- `cargo test -p sc-compose --test cli -- diagnostic`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `maturin develop`
- `pytest bindings/python/tests`

Run the full focused list, including the Python commands, before the move and
again after the move. Record serialized fixtures, classification results, and
before/after production-NLOC evidence.

## Completion evidence

- Baseline target was `d6ed03e` (`origin/integrate/phase-k`, including the
  merged K.1 and K.3 work). Before the move, the focused diagnostics suite
  passed 18 tests, JSON-CLI diagnostics passed 26 tests, the CLI diagnostic
  filter completed with zero matched tests and no failures, and the workspace,
  clippy, format, diff, `maturin develop`, and Python binding gates passed.
  The baseline binding suite passed 52/52 in the isolated
  `/tmp/sc-compose-k4-venv` environment.
- The stable schema characterization now directly covers all 72
  `DiagnosticCode` variants for both `as_str()` and serde spellings, all three
  severity spellings, the envelope schema version/field shape, record defaults
  and location/include-chain ordering, and each filesystem classification
  boundary. The post-move diagnostics characterization passed 26 tests.
- The decomposition keeps `DiagnosticCode`, `DiagnosticSeverity`,
  `Diagnostic`, `DiagnosticEnvelope<T>`, and
  `DIAGNOSTIC_SCHEMA_VERSION` at their existing crate paths and preserves the
  exact public field layout and serialized names. No code was added, removed,
  renamed, or repurposed; the crate-root re-exports and Python consumers are
  unchanged.
- Ownership evidence uses nonblank, non-comment Rust lines outside test
  modules. The baseline `diagnostics.rs` contained 294 production lines in one
  mixed owner. After the move, the private owners are: `schema.rs` 174 lines
  (enum, severity, and string/serde schema), `filesystem.rs` 59 lines
  (classification and platform loop mapping), `record.rs` 47 lines
  (record fields and constructors), `envelope.rs` 18 lines (schema envelope),
  and the `diagnostics.rs` facade 9 lines. The largest owner fell from 294 to
  174 lines.
- Duplication evidence by category: schema has one enum and one `as_str`
  mapping; filesystem has one classifier and one platform-specific loop
  helper; record construction has one field layout and one constructor chain;
  envelope serialization has one generic struct and one schema-version source.
  The facade contains only module declarations, the schema-version constant,
  and re-exports, so no wire fields or code mappings are duplicated across
  modules.
- Post-move validation passed the focused commands, workspace tests (266 unit,
  51 extraction integration, and 16 integration tests), clippy, formatting,
  diff checks, `maturin develop --manifest-path bindings/python/Cargo.toml`,
  and the Python binding suite (52/52).

## Dependencies and non-closure

Recommended predecessor for K.5 and K.6. No new diagnostics or changes to error policy are in scope.
