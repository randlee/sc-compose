---
id: K.5
title: Error-Family Modules
phase: K
status: planned
branch: sprint/k-5-error-family-modules
worktree: ../sc-compose-worktrees/sprint/k-5-error-family-modules
target: integrate/phase-k
---

# Sprint K.5 — Error-Family Modules

## Purpose and evidence

Issue #311 ranks `crates/sc-composer/src/error.rs` at 3.99/10 and reports 55% duplication. The file defines six error families, repeated constructors/accessors, display/source/backtrace behavior, recovery hints, and `ComposeError` conversions. This sprint isolates family ownership while retaining the crate's error API.

## Goal

Produce a production-ready private decomposition of the error families while
preserving the existing API and user-visible semantics.

## Required work

- Record the baseline constructor, formatting, source, backtrace, hint, and
  conversion characterization before moving implementation code.
- Implement only the seams listed under Exact targets and deliverables, retain
  the compatibility root and Python consumers, and rerun the characterization
  suite after the move.
- Record ownership and production-NLOC evidence and complete every command in
  Required validation before claiming closure.

## Hard dependencies

The hard dependencies are this sprint's plan-gate approval and
`integrate/phase-k` as the merge-forward target. K.4 is recommended first, but
is not a hard dependency when the existing exports remain stable.

## Production-ready expectation

Every deliverable listed below must land at production-ready quality for this
sprint's behavior-preserving scope. Partial module movement, test-only work,
or an unmeasured ownership split cannot satisfy the acceptance criteria.

## Exact targets and deliverables

- `crates/sc-composer/src/error.rs`, including `RecoveryHint*`,
  `ResolveError`, `IncludeError`, `ValidationError`, `RenderError`,
  `ConfigError`, `ComposeError`, and shared display/source helpers.
- Create private family modules and a compatibility root that re-exports all existing public types and conversion implementations.
- Add or strengthen characterization tests for constructors, accessors,
  `Display`, `Error::source`, backtrace presence, recovery hints, diagnostic
  formatting, and every `From<FamilyError> for ComposeError` conversion before
  moving code.

## Planned seam

Family modules may own their concrete fields and constructors, but the root
module remains the compatibility surface and shared formatting may only be
deduplicated when output is identical:

```rust
pub enum ComposeError {
    Resolve(ResolveError),
    Include(IncludeError),
    Validation(Box<ValidationError>),
    Render(RenderError),
    Config(ConfigError),
}

impl ComposeError {
    pub const fn code(&self) -> Option<DiagnosticCode>;
}
```

All existing family types and `From` conversions remain available from
`crate::error`; no error family or public source path is deleted or renamed.

## Acceptance criteria

- Public type paths, visibility, constructors, accessors, codes, messages, source chains, and conversion precedence are unchanged.
- Shared helpers are deduplicated only where their output is demonstrably identical; family-specific text remains family-owned.
- No error code, recovery hint, or user-facing message changes as a side effect.
- No family is replaced by a generic error type; if a shared helper cannot be
  proven output-equivalent, it remains family-specific.

## Required validation

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-composer error::tests`
- `cargo test -p sc-composer --test integration -- error`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `maturin develop`
- `pytest bindings/python/tests`

Run the full focused list, including the Python commands, before the move and
again after the move. Record display, source-chain, conversion, and
before/after production-NLOC evidence.

## Dependencies and non-closure

Recommended after K.4. No error-policy redesign, message rewrite, or new public error family is in scope.
