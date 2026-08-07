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

- `crates/sc-composer/src/diagnostics.rs:1-417`, including `DiagnosticCode`, `as_str`, filesystem classification, `Diagnostic`, constructors, and `DiagnosticEnvelope<T>`.
- Create private schema, filesystem, record, and envelope modules behind the existing `crate::diagnostics` and crate-root re-exports.
- Characterize every serialized code/severity spelling, envelope shape, path/line/column behavior, include-chain ordering, and filesystem classification before moving code.

## Acceptance criteria

- `DiagnosticCode::as_str`, serde names, schema version, JSON envelope fields, constructor defaults, and filesystem classifications are byte-for-byte compatible.
- No diagnostic code is added, removed, renamed, or repurposed; no caller changes are required.
- Duplication evidence is recorded by category, not by speculative line-count reduction alone.

## Required validation

Run diagnostic unit tests and JSON CLI tests before and after, `cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

## Dependencies and non-closure

Recommended predecessor for K.5 and K.6. No new diagnostics or changes to error policy are in scope.
