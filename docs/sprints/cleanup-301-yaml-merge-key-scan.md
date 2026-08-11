---
id: CLEANUP-301
title: Combine redundant per-line YAML merge-key scan passes
status: complete
branch: cleanup/yaml-merge-key-scan-consolidate
worktree: ../sc-compose-worktrees/cleanup/yaml-merge-key-scan-consolidate
target: develop
---

# Cleanup 301 — YAML merge-key scan consolidation

## Goal

Combine redundant per-line scans in the YAML merge-key detector without
changing quote, comment, or block-scalar handling.

## Required Fix

- Reuse one quote/comment scan for merge-key and block-scalar decisions.
- Keep the separate semantic results and security-sensitive tests.
- Close as no-change if consolidation increases complexity or risk.

## Decision

The quote/comment scan is now performed once per line and supplies both the
merge-key and block-scalar results. The semantic checks remain separate so the
security-sensitive cases stay explicit.

## Acceptance Criteria

- Real `<<: *defaults`, quoted `<<`, comments, and block-scalar text remain
  correctly distinguished.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace` pass.

## References

- Issue #301: https://github.com/randlee/sc-compose/issues/301
