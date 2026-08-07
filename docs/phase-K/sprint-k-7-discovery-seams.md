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

## Goal

Produce a production-ready private discovery scanner, scope, and masking split
only if characterization proves it safe; otherwise leave the sprint explicitly
non-closed with evidence explaining why no safe seam was found.

## Required work

- Record the baseline discovery and Python-wrapper characterization before
  moving implementation code.
- Implement only the seams listed under Exact targets and deliverables, retain
  every existing discovery path, and rerun the characterization suite after
  the move or before documenting an abandon decision.
- Record ownership and production-NLOC evidence and complete every command in
  Required validation before claiming closure.

## Hard dependencies

The hard dependencies are this sprint's plan-gate approval and
`integrate/phase-k` as the merge-forward target. There is no hard dependency on
another Phase K sprint; K.4 review is recommended because discovery feeds
diagnostics.

## Production-ready expectation

Every deliverable listed below must land at production-ready quality for this
sprint's behavior-preserving scope. Tests alone do not satisfy closure: if no
safe seam is found, the sprint remains non-closed with that evidence recorded.

## Exact targets and deliverables

- `crates/sc-composer/src/discovery.rs`, especially `discover_tokens`,
  `discover_tokens_with_delimiters`, `walk_template`, delimiter helpers,
  `parse_for_loop_scope`, `parse_set_scope`, `collect_identifiers`,
  `mask_quoted_literals`, and `mask_filter_names`.
- If safe, create private scanner/scope/identifier modules behind unchanged `discover_tokens`, `discover_tokens_with_brace_count`, `discover_tokens_with_delimiters`, `discover_all_pass_tokens`, and `has_bare_for_loop_over` paths.
- Add or strengthen characterization tests for custom delimiters, brace counts,
  whitespace markers, nested/shadowed loops, set locals, filters, quoted
  literals, loop built-ins, malformed/unclosed tags, and pass maps before
  moving code.

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
- The sprint must demonstrate a real ownership split. If characterization
  proves that no safe split exists, the sprint records the evidence as a
  failure/non-closure result and does not merge or count toward Phase K exit;
  test strengthening alone is never claimed as decomposition completion.
- No extraction, validation, or discovery semantics change.
- Characterization coverage is a prerequisite, not the closure claim: if no
  safe ownership split emerges, the sprint remains open (or is explicitly
  re-planned) with an abandoned-move record rather than a fabricated
  decomposition.

## Required validation

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-composer discovery::tests`
- `cargo test -p sc-composer --test integration -- discovery`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `maturin develop`
- `pytest bindings/python/tests`

Run the full focused list, including the Python commands, before the move and
again after the move (or before the abandon decision if characterization
proves the seam unsafe). Record token, scope, and pass-map comparisons plus
before/after production-NLOC evidence.

## Dependencies and non-closure

Independent, but should be reviewed after K.4 because discovery feeds diagnostics. No new Jinja syntax support is in scope.
