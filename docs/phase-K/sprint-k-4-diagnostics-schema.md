---
id: K.4
title: Diagnostic Schema and Envelope
phase: K
status: planned
branch: sprint/k-4-diagnostics-schema
worktree: ../sc-compose-worktrees/sprint/k-4-diagnostics-schema
target: integrate/phase-k
---

# Sprint K.4 — Diagnostic Schema and Envelope

## Purpose and evidence

Issue #311 ranks `crates/sc-composer/src/diagnostics.rs` at 3.90/10 and reports 38% duplication. The module combines the stable code enum/string mapping, filesystem error classification, diagnostic record construction, and generic serialized envelopes. These are distinct compatibility boundaries and should be separated without changing the wire schema.

## Exact targets and deliverables

- `crates/sc-composer/src/diagnostics.rs`, including `DiagnosticCode`,
  `as_str`, filesystem classification, `Diagnostic`, constructors, and
  `DiagnosticEnvelope<T>`.
- Create private schema, filesystem, record, and envelope modules behind the existing `crate::diagnostics` and crate-root re-exports.
- Characterize every serialized code/severity spelling, envelope shape, path/line/column behavior, include-chain ordering, and filesystem classification before moving code.

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

Run `cargo test -p sc-composer diagnostics::tests`, `cargo test -p sc-compose
--test json_cli -- diagnostic`, and `cargo test -p sc-compose --test cli --
diagnostic` against the baseline before the move and rerun the same commands
after the move. Then run `cargo fmt --all --check`, `git diff --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test
--workspace`. Record serialized fixtures, classification results, and
before/after production-NLOC evidence.

## Dependencies and non-closure

Recommended predecessor for K.5 and K.6. No new diagnostics or changes to error policy are in scope.
