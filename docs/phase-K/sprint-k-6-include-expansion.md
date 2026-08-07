---
id: K.6
title: Include Expansion Seams
phase: K
status: planned
branch: sprint/k-6-include-expansion
worktree: ../sc-compose-worktrees/sprint/k-6-include-expansion
target: integrate/phase-k
---

# Sprint K.6 — Include Expansion Seams

## Purpose and evidence

Issue #311 ranks `crates/sc-composer/src/include.rs` at 4.05/10, reports 54% duplication, and gives CCN 10. The module combines expansion state/source caching, recursive traversal, include directive recognition, path resolution, confinement, and depth/cycle errors. The path-containment behavior shipped in CLEANUP-298 is a hard contract for this sprint.

## Goal

Produce a production-ready private decomposition of include state, directives,
and paths while preserving containment, expansion, and graph behavior.

## Required work

- Record the baseline include graph, confinement, filesystem, and depth
  characterization before moving implementation code.
- Implement only the seams listed under Exact targets and deliverables, retain
  `canonicalize_within_roots`, and rerun the characterization suite after the
  move.
- Record ownership and production-NLOC evidence and complete every command in
  Required validation before claiming closure.

## Hard dependencies

The hard dependencies are this sprint's plan-gate approval,
`integrate/phase-k` as the merge-forward target, and the existing CLEANUP-298
containment contract. K.4 is recommended first, but is not a hard dependency
when the existing exports remain stable.

## Production-ready expectation

Every deliverable listed below must land at production-ready quality for this
sprint's behavior-preserving scope. Partial module movement, test-only work,
or an unmeasured ownership split cannot satisfy the acceptance criteria.

## Exact targets and deliverables

- `crates/sc-composer/src/include.rs`, especially `ExpandedTemplate`,
  `ExpansionState`, `expand_includes`, `expand_file`,
  `resolve_include_path`, `canonicalize_include`, and
  `parse_include_directive`.
- Create private expansion, path, and directive modules while preserving `expand_includes`, `ExpandedTemplate`, `IncludeDepth`, cached source text, and all error behavior.
- Add or strengthen characterization tests for nested includes, duplicate
  includes/cache reuse, missing files, confinement escapes, symlink loops,
  cycles, depth ceilings, frontmatter capture, and custom delimiters before
  moving code.

## Planned seam

The public include API and path-containment call remain unchanged; private
state, path, and directive helpers may move behind it:

```rust
pub fn expand_includes(
    template_path: impl AsRef<Path>,
    root: &ConfiningRoot,
    policy: &ComposePolicy,
) -> Result<ExpandedTemplate, ComposeError>;
fn parse_include_directive(line: &str) -> Option<&str>;
```

`ExpandedTemplate` stays at `crate::include::ExpandedTemplate`, and
`canonicalize_within_roots` remains the single containment implementation.
No include source path or confinement helper is deleted or renamed.

## Acceptance criteria

- Expanded text, resolved-file ordering, source/frontmatter maps, include chains, depth behavior, path normalization, and diagnostic codes/messages are unchanged.
- Path-containment helpers remain single-source-of-truth; no second containment implementation is introduced.
- Production-NLOC and ownership evidence demonstrate a smaller orchestration surface without changing recursion or I/O policy.
- No new path-containment implementation, resolver policy, or include syntax is
  introduced; a failed seam characterization leaves the existing recursion
  intact and records why.

## Required validation

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-composer include::tests`
- `cargo test -p sc-composer --test integration -- include`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `maturin develop`
- `pytest bindings/python/tests`

Run the full focused list, including the Python commands, before the move and
again after the move. Record path containment, graph ordering, and
before/after production-NLOC evidence.

## Dependencies and non-closure

Recommended after K.4; depends on the existing CLEANUP-298 containment contract. No include syntax, depth policy, or resolver behavior changes are in scope.
