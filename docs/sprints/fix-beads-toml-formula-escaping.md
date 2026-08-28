---
status: complete
branch: fix/beads-toml-formula-escaping
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/beads-toml-formula-escaping
---

# FIX-BEADS-FUZZ-TEMPLATE-001: TOML formula escaping gap

## Source

PR #564 (`fuzz/beads-integration-campaign`), Phase 1 stock adversarial-fuzz
campaign, `template-probe` finding, pinned as
`toml_formula_templates_embed_unescaped_quotes_and_backslashes` in
`crates/sc-composer-beads/src/render.rs`.

## Problem

`.formula.toml.j2` templates produced invalid TOML when a rendered value
contained a double quote or backslash. Auto escaping handled HTML, XML, and
JSON extensions but had no TOML-aware path.

## Fix

Formula TOML templates now select a dedicated auto-escape path that emits
TOML basic-string escapes for quotes, backslashes, and control characters.
The pinned regression test parses the rendered formula with the TOML parser and
asserts the original hostile string round-trips unchanged.

## Acceptance Criteria

- [x] Formula values containing quotes and/or backslashes render as valid TOML.
- [x] The pinned test parses the rendered output and verifies round-tripping.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [x] `cargo fmt --all --check` passes.

## Validation

- `cargo test -p sc-composer-beads toml_formula_templates_embed_unescaped_quotes_and_backslashes`
- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `git diff --check`
