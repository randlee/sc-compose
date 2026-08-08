---
id: CLEANUP-298
title: Centralize include/resolver path containment logic
status: complete
branch: cleanup/include-resolver-root-containment
worktree: ../sc-compose-worktrees/cleanup/include-resolver-root-containment
target: develop
---

# Cleanup 298 — centralize include/resolver path containment

## Goal

Centralize duplicated allowed-root containment and canonicalization while
preserving caller-specific errors.

## Acceptance Criteria

- Existing, missing, sibling-prefix, symlink, and boundary escape cases pass
  for both callers.
- Existing distinct error codes/messages remain unchanged.
- Format, clippy, and workspace tests pass.

## References

- Issue #298: https://github.com/randlee/sc-compose/issues/298
