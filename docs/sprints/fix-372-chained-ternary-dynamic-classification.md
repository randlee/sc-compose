---
id: FIX-372
status: in-progress
branch: fix/372-chained-ternary-dynamic-classification
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/372-chained-ternary-dynamic-classification
target: integrate/phase-M
---

# Sprint FIX-372 — Chained Ternary Inside `@<{{ ... }}>` Mis-Parses As A Garbage Candidate Path

## Problem

Issue #372, found by adversarial fuzzing of the M.2 include-graph resolver
(campaign `m2-include-fuzz-20260811-1`): a 3-way chained ternary such as

```
@<{{ "a.md" if x else "b.md" if y else "c.md" }}>
```

mis-parses into a garbage candidate path instead of being classified as
`IncludeDirective::Dynamic`, which M.2's conditional-candidate enumeration
otherwise handles correctly for simple (single) ternaries.

## Root cause

`directive.rs::quoted_literal()` accepts a de-quoted literal containing
embedded `" if "` / `" else "` substrings — it doesn't recognize that a
literal boundary has been crossed into a second conditional-expression arm,
so it treats the whole chained expression text as one flat literal.

## Fix design

Tighten `quoted_literal()`'s boundary detection so a candidate literal
containing conditional-expression keywords (`" if "`, `" else "`) is not
treated as a complete quoted string — either:

- reject it and fall through to `Dynamic` classification (simplest, matches
  existing single-ternary `Dynamic` handling), or
- recursively classify nested ternary arms as additional `Dynamic`
  candidates (more complete, matches the sprint's "exhaustive static
  candidates" requirement for conditional includes).

Prefer whichever keeps `Dynamic`'s existing candidate-enumeration contract
intact; document the choice in Closeout Evidence.

## Required tests (two-commit red/green)

1. Regression fixture: `@<{{ "a.md" if x else "b.md" if y else "c.md" }}>`
   — assert classification as `IncludeDirective::Dynamic`, not a garbage
   literal path.
2. Confirm existing single-ternary `@<{{ "a.md" if x else "b.md" }}>`
   classification is unchanged (positive control).
3. If nested-arm enumeration is chosen: assert all three candidates
   (`a.md`, `b.md`, `c.md`) are enumerated for the conditional-candidate
   graph.

## Out of scope

- Any change to simple (non-chained) ternary handling, already correct.
- General Jinja expression parsing beyond the literal-vs-conditional
  boundary needed here.

## Acceptance criteria

- `cargo test --workspace` passes, including the new regression test(s).
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #372's exact repro classifies as `Dynamic`, not a garbage path.
- Closeout Evidence records the fix commit and which of the two fix-design
  options was implemented and why.

## References

- Issue #372: https://github.com/randlee/sc-compose/issues/372
- `crates/sc-composer/src/include/directive.rs::quoted_literal()`
- Fuzz campaign `m2-include-fuzz-20260811-1`, report
  `site/reports/20260811-2-fuzz-report.html`
