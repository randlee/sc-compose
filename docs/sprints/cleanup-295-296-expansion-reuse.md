---
id: CLEANUP-295-296
title: Reuse include expansion for preflight, render, and diagnostics
status: complete
branch: cleanup/expansion-reuse-diagnostics
worktree: ../sc-compose-worktrees/cleanup/expansion-reuse-diagnostics
target: develop
---

# Cleanup 295/296 — reuse include expansion instead of re-resolving

## Goal

Reuse the preflight expansion for rendering and use
`ExpandedTemplate.source_texts` for missing-frontmatter diagnostics.

## Required Fix

- Ensure custom-delimiter rendering expands includes once.
- Stop diagnostics from rereading files already captured in the expansion.

## Acceptance Criteria

- A one-expansion regression test passes.
- A mutation-after-expansion test proves diagnostics use cached source.
- Format, clippy, and workspace tests pass.

## References

- Issue #295: https://github.com/randlee/sc-compose/issues/295
- Issue #296: https://github.com/randlee/sc-compose/issues/296
