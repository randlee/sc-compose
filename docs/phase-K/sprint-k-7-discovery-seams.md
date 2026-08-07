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

- `crates/sc-composer/src/discovery.rs`, especially `discover_tokens`,
  `discover_tokens_with_delimiters`, `walk_template`, delimiter helpers,
  `parse_for_loop_scope`, `parse_set_scope`, `collect_identifiers`,
  `mask_quoted_literals`, and `mask_filter_names`.
- If safe, create private scanner/scope/identifier modules behind unchanged `discover_tokens`, `discover_tokens_with_brace_count`, `discover_tokens_with_delimiters`, `discover_all_pass_tokens`, and `has_bare_for_loop_over` paths.
- Characterize custom delimiters, brace counts, whitespace markers, nested/shadowed loops, set locals, filters, quoted literals, loop built-ins, malformed/unclosed tags, and pass maps before moving code.

## Planned seam

Discovery's existing API remains stable while the scanner, scope tracking, and
identifier masking are made private seams:

```rust
pub fn discover_tokens_with_delimiters(
    text: &str,
    open_delimiter: &str,
    close_delimiter: &str,
) -> BTreeSet<VariableName>;
pub fn discover_all_pass_tokens(
    parsed: &ParsedTemplate,
) -> BTreeMap<usize, BTreeSet<VariableName>>;
```

The returned token sets and pass map are the contract. No discovery API or
source path is deleted or renamed, and no Jinja syntax is added.

## Acceptance criteria

- Token sets, scope filtering, loop built-ins, delimiter behavior, and pass maps are identical.
- The sprint either demonstrates a real ownership split or explicitly records why the move was abandoned after characterization; test strengthening alone is not claimed as a decomposition.
- No extraction, validation, or discovery semantics change.
- Characterization coverage is a prerequisite, not the closure claim: if no
  safe ownership split emerges, the sprint closes with strengthened tests and
  an explicit abandoned-move record rather than a fabricated decomposition.

## Required validation

Run `cargo test -p sc-composer discovery::tests` and
`cargo test -p sc-composer --test integration -- discovery` against the
baseline before the move and rerun the same commands after the move. Then run
`cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets
--all-features -- -D warnings`, and `cargo test --workspace`. Record token,
scope, and pass-map comparisons plus before/after production-NLOC evidence.

## Dependencies and non-closure

Independent, but should be reviewed after K.4 because discovery feeds diagnostics. No new Jinja syntax support is in scope.
