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

- `crates/sc-composer/src/include.rs:14-811`, especially `ExpandedTemplate`, `ExpansionState`, `expand_file`, `resolve_include_path`, `canonicalize_include`, and `parse_include_directive`.
- Create private expansion, path, and directive modules while preserving `expand_includes`, `ExpandedTemplate`, `IncludeDepth`, cached source text, and all error behavior.
- Characterize nested includes, duplicate includes/cache reuse, missing files, confinement escapes, symlink loops, cycles, depth ceilings, frontmatter capture, and custom delimiters before moving code.

## Acceptance criteria

- Expanded text, resolved-file ordering, source/frontmatter maps, include chains, depth behavior, path normalization, and diagnostic codes/messages are unchanged.
- Path-containment helpers remain single-source-of-truth; no second containment implementation is introduced.
- Production-NLOC and ownership evidence demonstrate a smaller orchestration surface without changing recursion or I/O policy.

## Required validation

Run include/path-containment characterization tests before and after, `cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

## Dependencies and non-closure

Recommended after K.4; depends on the existing CLEANUP-298 containment contract. No include syntax, depth policy, or resolver behavior changes are in scope.
