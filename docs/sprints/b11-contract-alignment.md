---
id: B11
title: Contract-Alignment
status: complete
branch: feat/b11-contract-alignment
worktree: ../sc-compose-worktrees/feat/b11-contract-alignment
target: integrate/phase-B
---

# Sprint B11 — Contract-Alignment

## Goal

- Align `docs/requirements.md` and `docs/architecture.md` to the shipped `sc-composer` public API.
- Remove the production-readiness doc/API drift around `Renderer`, `validate()`, and ownership boundaries.
- Keep the sprint docs-only at the contract level; no library behavior changes are in scope.

## Hard Dependencies

- `integrate/phase-B` at the current merged Phase B tip.
- Production-readiness review findings that identified the mismatches in `Renderer::new()`, `validate()`, and the renderer ownership narrative.

## Exact Targets

- `docs/requirements.md`
- `docs/architecture.md`
- `crates/sc-composer/src/lib.rs`
- `crates/sc-composer/src/renderer.rs`
- `crates/sc-composer/src/validate.rs`

Phase B branch note:

- Exact Targets are verified against `integrate/phase-B`, which is the target
  branch for this cleanup work.
- `crates/sc-composer/src/validate.rs` is the public validation entrypoint on
  that Phase B line and is the authoritative source for the shipped
  `validate()` signature.

## Deliverables

- `docs/requirements.md` describes `Renderer::new()` with no config argument.
- `docs/architecture.md` describes `validate()` as returning `Result<ValidationReport, ComposeError>`.
- Ownership language is corrected so include resolution, variable expansion, and validation live in `compose()` / `validate()` flow rather than in `Renderer`.
- All public-facing code samples and signatures match the exports in `crates/sc-composer/src/lib.rs` and the implementations in `renderer.rs` / `validate.rs`.

## Required Work

- Audit every normative reference to `Renderer::new(...)` and replace the stale constructor shape with the shipped zero-argument API.
- Correct the `validate()` signature and surrounding prose anywhere the docs still imply a bare `ValidationReport` return.
- Rewrite the architecture narrative so `Renderer` remains a pure render wrapper and request orchestration stays in `compose()` / `validate()` entrypoints.
- Cross-check the final docs against `crates/sc-composer/src/lib.rs`, `renderer.rs`, and `validate.rs` before closing the sprint.

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
pub use composer::{compose, compose_with_observer};
pub use renderer::{render_template, Renderer};
pub use validate::{validate, validate_with_observer};

impl Renderer {
    pub fn new() -> Self;
}

pub fn validate(request: &ComposeRequest) -> Result<ValidationReport, ComposeError>;
```

## This Sprint Does Not Close

- No changes to shipped rendering behavior.
- No CLI command-surface changes.
- No new diagnostics or observer behavior.

## Acceptance Criteria

- All described API signatures match `crates/sc-composer/src/renderer.rs` and `crates/sc-composer/src/lib.rs`.
- The docs no longer claim that `Renderer` owns include resolution, variable expansion, or validation.
- No contradictory constructor or return-type samples remain in the normative docs.
- `cargo test --workspace` passes on the implementation branch that lands the doc corrections.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
