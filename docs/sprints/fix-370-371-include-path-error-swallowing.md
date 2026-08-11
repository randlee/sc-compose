---
id: FIX-370-371
status: in-progress
branch: fix/370-371-include-path-error-swallowing
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/370-371-include-path-error-swallowing
target: develop
---

# Sprint FIX-370-371 — Include Resolver Swallows Non-NotFound Errors On The Relative-Candidate Attempt

## Problem

Two bugs, one root cause, found by adversarial fuzzing of the M.2
include-graph resolver (campaign `m2-include-fuzz-20260811-1`):

- **#370 (High, silent content substitution)**: a permission-denied nested
  include target (`chmod 000`) is silently replaced by an unrelated
  same-named root-relative decoy file's content, with exit 0 and no
  diagnostic at all.
- **#371 (Medium, misleading diagnostic)**: the same root cause, but when no
  root-relative decoy exists — the real permission-denied error is discarded
  in favor of a fabricated `ERR_INCLUDE_NOT_FOUND` naming a path never
  referenced in any template.

## Root cause

`path.rs::resolve_include_path()` discards *any* `Err` (not just
`NotFound`) returned by the relative-candidate resolution attempt before
falling back to root-relative resolution. Any non-`NotFound` error
(permission-denied is the fuzzed case, but this is a class of bug, not just
one errno) on the relative attempt is silently swallowed instead of
propagated.

## Fix design

In `path.rs::resolve_include_path()`, only fall through to root-relative
resolution when the relative-candidate attempt's error is `NotFound`.
Propagate any other error (permission-denied, etc.) immediately with a
diagnostic naming the actual relative-candidate path and the real OS error,
not a fabricated `ERR_INCLUDE_NOT_FOUND`.

## Required tests (two-commit red/green)

1. Regression fixture: nested include target `chmod 000`, no root-relative
   decoy present — assert the real permission-denied diagnostic is surfaced
   (not `ERR_INCLUDE_NOT_FOUND`), naming the correct relative path (#371).
2. Regression fixture: nested include target `chmod 000`, WITH a same-named
   root-relative decoy present — assert render fails with the
   permission-denied diagnostic, and assert the decoy's content does NOT
   appear anywhere in rendered output (#370).
3. Confirm the existing NotFound-triggers-root-relative-fallback behavior is
   unchanged (positive control — do not regress the legitimate fallback
   path).

## Out of scope

- Any change to root-relative resolution itself, only the error-classification
  gate before falling through to it.
- Windows ACL-based permission-denied simulation — `chmod 000` is POSIX-only;
  document if the regression test needs a `#[cfg(unix)]` guard.

## Acceptance criteria

- `cargo test --workspace` passes, including both new regression tests.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- #370's repro no longer substitutes decoy content; #371's repro no longer
  fabricates a not-found diagnostic for a path that was never referenced.
- Closeout Evidence records exact fix commit(s) and confirms both issues
  closed by the same code change.

## References

- Issue #370: https://github.com/randlee/sc-compose/issues/370
- Issue #371: https://github.com/randlee/sc-compose/issues/371
- `crates/sc-composer/src/include/path.rs::resolve_include_path()`
- Fuzz campaign `m2-include-fuzz-20260811-1`, report
  `site/reports/20260811-2-fuzz-report.html`
