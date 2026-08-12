---
id: CLEANUP-293
title: Fix redundant frontmatter_safe|yaml_safe filter chain
status: complete
branch: cleanup/template-frontmatter-yaml-versioning
worktree: ../sc-compose-worktrees/cleanup/template-frontmatter-yaml-versioning
target: develop
---

# Cleanup 293 — redundant frontmatter_safe|yaml_safe filter chain

## Goal

`.claude/skills/codex-orchestration/sprint-plan.md.j2` applies `title`/`worktree`
through `frontmatter_safe|yaml_safe`. `frontmatter_safe` rewrites exact delimiter
lines before `yaml_safe` quotes the result, so chaining both filters corrupts
values.

## Reproducer

Render the sprint-plan template with `title: "Release\n---\nNotes"`.

- Current: literal backslash artifacts appear after parsing.
- Expected: `yaml_safe` alone preserves the original value.

## Required Fix

- Use `yaml_safe` only for YAML `title`/`worktree` fields.
- Sweep `.claude/skills/**/*.j2` for the same redundant chain.
- Keep `frontmatter_safe` available for raw delimiter-sensitive output.

## Acceptance Criteria

- The reproducer round-trips without backslash artifacts.
- No other template behavior changes.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace` pass.

## References

- Issue #293: https://github.com/randlee/sc-compose/issues/293
