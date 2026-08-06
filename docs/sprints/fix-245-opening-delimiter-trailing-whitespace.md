---
id: FIX-245
title: Opening frontmatter delimiter with trailing whitespace silently bypasses required_variables enforcement
status: complete
branch: fix/245-opening-delimiter-trailing-whitespace
worktree: ../sc-compose-worktrees/fix/245-opening-delimiter-trailing-whitespace
target: develop
---

# Sprint FIX-245 — Opening delimiter trailing whitespace bypasses `required_variables`

## Goal

Fix GitHub issue #245: a `---` opening delimiter followed by trailing
whitespace before the newline (`"---   \n"`) causes `split_frontmatter` to
return `None` for the whole document — the entire frontmatter block
(including `required_variables`) is silently treated as ordinary body
content. `required_variables` is never parsed or enforced, and the apparent
frontmatter leaks into rendered output verbatim, with no diagnostic.

## Hard Dependencies

- `develop` branch at HEAD (no blocked sprints)

## Root Cause

`opening_delimiter_len` (`crates/sc-composer/src/frontmatter/parser.rs`)
only matches an exact `---\n` / `---\r\n` / `---`(EOF) first line. Trailing
whitespace after `---` fails all three branches, so `split_frontmatter`
returns `None`, meaning "no frontmatter in this document" — silently, with
no diagnostic.

The symmetric closing-delimiter case (`"--- \n"` as a candidate closing
line) is handled correctly today and **must stay exactly as-is**: closing
matching uses `trimmed = line.trim_end_matches(['\n', '\r'])` against exact
`"---"`/`"..."`, so a closing line with trailing whitespace does NOT match,
and — because the block was already opened — this correctly surfaces
`ERR_CONFIG_PARSE: no closing delimiter found` (a loud, correct failure).
Do not touch this closing-side logic.

The bug is specifically that the *opening* side's equally-strict rejection
has a silent (not loud) consequence, because before a block is opened there
is no "unclosed block" error path to fall into — it just looks like there
was never any frontmatter at all.

## Exact Targets

- `crates/sc-composer/src/frontmatter/parser.rs` (`opening_delimiter_len`
  only — do not modify the closing-delimiter matching logic)

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint,
the sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- Root cause fixed: `opening_delimiter_len` must recognize `---` followed by
  trailing horizontal whitespace (spaces and/or tabs only — not other
  whitespace) before the line ending (`\n`, `\r\n`, or EOF) as a valid
  opening delimiter, so the frontmatter block is correctly parsed and
  `required_variables` is enforced exactly as if there were no trailing
  whitespace.
- Closing-delimiter matching logic is unchanged — a closing line with
  trailing whitespace continues to correctly surface
  `ERR_CONFIG_PARSE: no closing delimiter found`. Do not make closing
  matching more lenient; do not make opening matching lenient beyond
  trailing horizontal whitespace (e.g. leading whitespace before `---`
  remains unmatched — out of scope, not part of this bug).
- Regression test: **land a new, dedicated commit first, before the fix
  commit**, containing the failing `#[ignore]`d regression test
  `opening_delimiter_with_trailing_whitespace_does_not_silently_bypass_required_variables`
  in `crates/sc-compose/tests/fuzz_regressions.rs`, confirmed failing against
  pre-fix code. Then land the fix in a second commit that also removes the
  `#[ignore]` attribute so the test passes. This ordering is now mandatory
  for every fix in the fuzz-campaign queue (process correction from
  quality-mgr after FIX-246/FIX-243 both had closeout narratives claiming a
  git-verifiable red→green trail that didn't actually exist — see
  SC-QA-255-001 and SC-QA-256-001). Do not describe the test as "promoted
  from an ignored test on another branch" — it is new to this branch; the
  closeout narrative must say so plainly and instead point at the two
  commits on *this* branch as the red→green evidence.
- No regression to existing frontmatter parsing/validation behavior for
  documents without trailing whitespace on the opening delimiter — verify by
  re-running the existing frontmatter module test suite and CLI integration
  tests clean.

## Required Work

- First commit: add the failing regression test (see below), confirm it
  fails against current `develop` code, commit and push it alone.
- Second commit: fix `opening_delimiter_len` to tolerate trailing horizontal
  whitespace on the opening line, remove `#[ignore]` from the regression
  test, confirm it now passes, commit and push.
- Add a unit test directly in `crates/sc-composer/src/frontmatter/`
  confirming `opening_delimiter_len`/`split_frontmatter` behavior for: (a) a
  normal `---\n` opening (unchanged), (b) `---   \n` opening with trailing
  spaces (now recognized), (c) `---\t\n` opening with a trailing tab (now
  recognized), (d) a closing line with trailing whitespace still correctly
  fails to close and surfaces `ERR_CONFIG_PARSE` (unchanged, confirms no
  closing-side regression).

## Explicit Code Sample

Repro (must go from silently bypassing `required_variables` to enforcing
it):

```
---   
required_variables:
  - name
---
Hi {{ name }}
```

`sc-compose render --file t.j2 --root <ROOT>` (no `--var` supplied)

Before fix: exits 0 — the whole document, including the apparent
frontmatter, renders verbatim as body; `required_variables` is never
enforced.

After fix: fails with a missing-required-variable diagnostic for `name`,
matching the behavior of the same template without the trailing whitespace.

## This Sprint Does Not Close

- No change to closing-delimiter matching behavior
- No change to leading-whitespace-before-`---` handling (out of scope)
- No broader frontmatter schema changes beyond opening-delimiter whitespace
  tolerance

## Acceptance Criteria

- `cargo test --workspace` passes, including the new regression test
  (landed `#[ignore]`d first in its own commit, then un-ignored in the fix
  commit)
- The explicit code sample above behaves exactly as described after the fix
- The closing-delimiter trailing-whitespace case continues to correctly
  surface `ERR_CONFIG_PARSE` (no regression)
- Existing frontmatter/CLI test suites remain green
- GitHub issue #245 can be closed referencing the merged PR
- Sprint doc closeout narrative accurately describes the two-commit
  red→green trail on this branch (no fabricated cross-branch provenance
  claims)

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`

## Closeout Evidence

This branch contains the complete red-to-green trail for the new regression
test; the test was created on this branch and was not promoted from another
worktree.

- `45e571e` — added the ignored regression test. Running it with `--ignored`
  against the pre-fix code failed because rendering exited 0 instead of
  returning exit code 2 with `ERR_VAL_MISSING_REQUIRED`.
- `61aadec` — updated `opening_delimiter_len`, added the four parser unit
  tests, and removed `#[ignore]`. The regression then passed.
- Parser delimiter unit tests: PASS (4/4).
- Fuzz regression tests: PASS (4/4).
- Workspace tests: PASS (`cargo test --workspace`).
- Clippy: PASS (`cargo clippy --all-targets --all-features -- -D warnings`).
- Formatting and whitespace checks: PASS (`cargo fmt --all --check` and
  `git diff --check`).
- Closing delimiters remain strict: a trailing-whitespace closing line still
  produces `ERR_CONFIG_PARSE`.
