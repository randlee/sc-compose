---
id: FIX-SIMP-001-FOLLOWUP
title: "docs/architecture.md Section 4 module list omits template_ext"
status: complete
branch: fix/simp-001-followup-architecture-doc
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/simp-001-followup-architecture-doc
target: docs/architecture.md
---

## Root Cause

Flagged as ATM-QA-003 (req-qa, Minor, non-blocking) during FIX-SIMP-001 QA
(PR #392): `docs/architecture.md` Section 4's module list did not mention the
new `template_ext` module (added by FIX-SIMP-001 to hold the shared
`strip_template_suffix` helper).

## Fix

Added a `template_ext` bullet to `docs/architecture.md` Section 4, between
`render` and `validate`, describing its single responsibility
(`strip_template_suffix`, shared by the renderer's auto-escape callback and
`sc-compose`'s template-init JSON detection).

## Acceptance Criteria

1. `docs/architecture.md` Section 4 lists `template_ext`.
2. `docs/project-plan.md` gets a Follow-on Fix Sprint entry for this
   docs-only follow-up.

## References

- FIX-SIMP-001 QA verdict (PR #392), finding ATM-QA-003.

## Priority

Minor, docs-only, non-blocking.

## Closeout Evidence

- Docs-only change to `docs/architecture.md` Section 4.
