---
status: in_progress
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

Formula TOML templates now select a caller-supplied, generic TOML escape mode.
The shared renderer only consumes that mode and has no knowledge of Beads
filenames; `sc-composer-beads` selects it for `.formula.toml.j2` templates.
The pinned regression test parses both single-line and triple-quoted multiline
rendered values with the TOML parser and asserts the original hostile string
round-trips unchanged.

## Acceptance Criteria

- [x] Formula values containing quotes and/or backslashes render as valid TOML.
- [x] The pinned test parses single-line and multiline rendered output and
  verifies round-tripping.
- [x] TOML mode is caller-selected and contains no Beads filename logic in
  `sc-composer`.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [x] `cargo fmt --all --check` passes.

## Validation

- `cargo test -p sc-composer-beads toml_formula_templates_embed_unescaped_quotes_and_backslashes`
- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `git diff --check`

## FIX-PR566-ARCH-001-TOML-BOUNDARY follow-up

The original implementation selected TOML escaping by matching
`.formula.toml` inside `crates/sc-composer/src/renderer.rs`. That violated the
shared-library boundary. The corrected implementation exposes the generic
`TemplateEscapeMode::Toml` policy and leaves the filename decision in
`sc-composer-beads/src/render.rs`, matching the existing caller-decides JSON
pattern.
