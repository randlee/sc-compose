---
status: complete
branch: fix/beads-render-error-message
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/beads-render-error-message
---

# FIX-BEADS-FUZZ-4177-BOUNDARY-01: opaque render-failure error message

## Source

PR #564 (`fuzz/beads-integration-campaign`), Phase 1 stock adversarial-fuzz
campaign, `boundary-probe` finding, pinned as
`render_failed_message_is_identical_for_distinct_failure_causes` in
`crates/sc-composer-beads/src/render.rs`.

## Problem

`BeadComposeError::RenderFailed.message` was byte-identical
(`"template rendering failed"`) for every distinct underlying failure cause,
because `render.rs` called `error.to_string()` on the underlying render error
instead of `RenderError::message()`, discarding the specific diagnostic.

## Resolution

The renderer now preserves `RenderError::message()`, and the pinned regression
test asserts that distinct failures retain distinct, non-opaque messages.

## Validation

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`

## References

- PR #564: https://github.com/randlee/sc-compose/pull/564
- `crates/sc-composer-beads/src/render.rs`
