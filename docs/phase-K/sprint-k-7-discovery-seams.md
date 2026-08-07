---
id: K.7
title: Template Discovery Seams
phase: K
status: planned
branch: sprint/k-7-discovery-seams
worktree: ../sc-compose-worktrees/sprint/k-7-discovery-seams
target: integrate/phase-k
---

# Sprint K.7 — Template Discovery Seams

## Purpose and evidence

Issue #311 ranks `crates/sc-composer/src/discovery.rs` at 4.06/10 with CCN 9 and 484 NLOC. The file combines delimiter walking, whitespace-control handling, loop/set scope parsing, identifier collection, quote/filter masking, and pass aggregation. Because loop-context behavior was recently stabilized, this sprint is characterization-first and may close with no move if the seam cannot be made behavior-preserving.

## Exact targets and deliverables

- `crates/sc-composer/src/discovery.rs:23-555`, especially `walk_template`, delimiter helpers, `parse_for_loop_scope`, `parse_set_scope`, `collect_identifiers`, `mask_quoted_literals`, and `mask_filter_names`.
- If safe, create private scanner/scope/identifier modules behind unchanged `discover_tokens`, `discover_tokens_with_brace_count`, `discover_tokens_with_delimiters`, `discover_all_pass_tokens`, and `has_bare_for_loop_over` paths.
- Characterize custom delimiters, brace counts, whitespace markers, nested/shadowed loops, set locals, filters, quoted literals, loop built-ins, malformed/unclosed tags, and pass maps before moving code.

## Acceptance criteria

- Token sets, scope filtering, loop built-ins, delimiter behavior, and pass maps are identical.
- The sprint either demonstrates a real ownership split or explicitly records why the move was abandoned after characterization; test strengthening alone is not claimed as a decomposition.
- No extraction, validation, or discovery semantics change.

## Required validation

Run the full discovery/validation characterization suite before and after any move, `cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

## Dependencies and non-closure

Independent, but should be reviewed after K.4 because discovery feeds diagnostics. No new Jinja syntax support is in scope.
