---
id: FIX-373
status: complete
branch: fix/373-diamond-frontmatter-dedup
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/373-diamond-frontmatter-dedup
target: integrate/phase-M
---

# Sprint FIX-373 — Diamond-Shared Frontmatter-less Leaf Emits Duplicate `ERR_VAL_MISSING_FRONTMATTER`

## Problem

Issue #373, found by adversarial fuzzing of the M.2 include-graph resolver
(campaign `m2-include-fuzz-20260811-1`): a frontmatter-less leaf file
referenced via two or more distinct include edges (a "diamond" shape) emits
`ERR_VAL_MISSING_FRONTMATTER` once per occurrence/edge instead of once per
canonical file — inconsistent with the sprint's own "each canonical source
hashed/counted once" dedup requirement (SHA-R3), which is already correctly
applied to sibling state elsewhere in the same function.

## Root cause

`expansion.rs::expand_file()` pushes to `state.frontmatters`
unconditionally on every visit, not gated by the same `is_new` check that
guards the sibling `resolved_files`/`source_texts` pushes in the same
function.

## Fix design

Gate the `state.frontmatters` push behind the existing `is_new` check
already used for `resolved_files`/`source_texts` in `expand_file()`, so a
diamond-shared file's missing-frontmatter diagnostic is recorded exactly
once per canonical file regardless of how many edges reference it.

## Required tests (two-commit red/green)

1. Regression fixture: a frontmatter-less leaf file referenced via two
   distinct parent templates (diamond shape) — assert exactly one
   `ERR_VAL_MISSING_FRONTMATTER` diagnostic is emitted, not one per edge.
2. Confirm a non-diamond (single-reference) frontmatter-less leaf still
   emits exactly one diagnostic (positive control, no regression).
3. Confirm `resolved_files`/`source_texts` dedup behavior for the same
   diamond fixture is unaffected (already correct — just a control check).

## Out of scope

- Any change to `resolved_files`/`source_texts` dedup logic, already correct.
- Frontmatter validation logic itself, only the emission-count gating.

## Acceptance criteria

- `cargo test --workspace` passes, including the new regression test.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #373's exact repro (diamond-shared frontmatter-less leaf) emits the
  diagnostic exactly once.
- Closeout Evidence records the fix commit and confirms the `is_new` gate
  now covers all three state pushes (`resolved_files`, `source_texts`,
  `frontmatters`) identically.

## References

- Issue #373: https://github.com/randlee/sc-compose/issues/373
- `crates/sc-composer/src/include/expansion.rs::expand_file()`
- Fuzz campaign `m2-include-fuzz-20260811-1`, report
  `site/reports/20260811-2-fuzz-report.html`

## Closeout Evidence

- Status: **complete**.
- Red regression baseline: `769b834` (`test: cover diamond frontmatter
  warning dedup`) reproduced two missing-frontmatter diagnostics for the
  shared leaf; the single-reference control and resolved-file/source-text
  dedup controls passed.
- Implementation: `4c498ab` (`fix: deduplicate frontmatter expansion state`)
  moves the `frontmatters` push under the existing `is_new` gate, matching
  `resolved_files` and `source_texts`. Follow-up `3ea9def` updates the
  duplicate-include characterization assertion from three frontmatter
  entries to the intended two canonical files.
- The diamond regression now emits exactly one
  `ERR_VAL_MISSING_FRONTMATTER`; the single-reference control emits exactly
  one, while the include graph retains two edges and two unique nodes.
- Validation: `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` all pass.
