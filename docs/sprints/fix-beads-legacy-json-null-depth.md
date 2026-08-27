---
status: complete
branch: fix/beads-legacy-json-null-depth
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/beads-legacy-json-null-depth
---

# FIX-BEADS-FUZZ-SHAPE-001: legacy JSON mode null-depth inconsistency

## Source

PR #564 (`fuzz/beads-integration-campaign`), Phase 1 stock adversarial-fuzz
campaign, `shape-probe`/`differential-probe` finding, pinned as
`renderer_json_legacy_mode_null_representation_diverges_by_nesting_depth` in
`crates/sc-composer/src/renderer.rs`.

## Problem

Under `JsonEscapeMode::Legacy`, a top-level `null` value rendered as an empty
string, but the identical `null` value nested inside an object or array rendered
as the literal, unquoted text `none`. The representation of `null` must not
depend on its nesting depth.

## Fix

Legacy JSON interpolation now serializes non-string values through
`serde_json` before applying string-content escaping. This preserves the
canonical JSON `null` token for top-level, array, and object values while
retaining the existing content-only escaping for string values. The pinned
renderer test asserts the corrected output at all three shapes.

## Acceptance Criteria

- [x] `null` renders identically at every nesting depth under Legacy mode.
- [x] The pinned renderer test asserts the fixed behavior rather than the
      former `none` output.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [x] `cargo fmt --all --check` passes.

## Validation

- `cargo test -p sc-composer renderer_json_legacy_mode_null_representation_diverges_by_nesting_depth`
- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `git diff --check`
