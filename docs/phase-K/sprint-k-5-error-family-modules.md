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

## Exact targets and deliverables

- `crates/sc-composer/src/error.rs:1-770`, including `RecoveryHint*`, `ResolveError`, `IncludeError`, `ValidationError`, `RenderError`, `ConfigError`, `ComposeError`, and shared display/source helpers.
- Create private family modules and a compatibility root that re-exports all existing public types and conversion implementations.
- Characterize constructors, accessors, `Display`, `Error::source`, backtrace presence, recovery hints, diagnostic formatting, and every `From<FamilyError> for ComposeError` conversion before moving code.

## Acceptance criteria

- Public type paths, visibility, constructors, accessors, codes, messages, source chains, and conversion precedence are unchanged.
- Shared helpers are deduplicated only where their output is demonstrably identical; family-specific text remains family-owned.
- No error code, recovery hint, or user-facing message changes as a side effect.

## Required validation

Run error unit/integration characterization tests before and after, `cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

## Dependencies and non-closure

Recommended after K.4. No error-policy redesign, message rewrite, or new public error family is in scope.
