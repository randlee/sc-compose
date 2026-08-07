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

Run `cargo test -p sc-composer include::tests` and
`cargo test -p sc-composer --test integration -- include` against the
baseline before the move and rerun the same commands after the move. Then run
`cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets
--all-features -- -D warnings`, and `cargo test --workspace`. Record path
containment, graph ordering, and before/after production-NLOC evidence.

## Dependencies and non-closure

Recommended after K.4; depends on the existing CLEANUP-298 containment contract. No include syntax, depth policy, or resolver behavior changes are in scope.
