---
id: FIX-375
status: complete
branch: fix/375-scshaerror-str
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/375-scshaerror-str
target: integrate/phase-M
---

# Sprint FIX-375 — `ScShaError` Has No `__str__`, `str(e)` Prints The Raw Args-Tuple Repr

## Problem

Issue #375, found by adversarial fuzzing of the M.2 sc-sha-python FFI
boundary: `str(e)` on a raised `ScShaError` prints the raw args-tuple repr
(e.g. `"('SC_SHA_INVALID_INPUT', 'utf8_file_bytes must be bytes; encode
text as UTF-8 explicitly')"`) instead of a readable message. Cosmetic/DX
issue — doesn't affect error-code-based dispatch, but makes default
logging/tracebacks unreadable.

## Root cause

`ScShaError` in `bindings/sc-sha-python/src/lib.rs` (~lines 12-27) extends
`PyException` and passes `(code, message)` as `args` with no custom
`__str__` override, so Python's default `BaseException.__str__` renders the
tuple repr.

## Fix design

Add a `__str__` override on `ScShaError` that renders just the
human-readable message (keep `.code` available as a separate attribute for
dispatch, unaffected by this change) — or equivalently set
`args = (message,)` and expose `code` as a separate attribute. Either way,
`.code`-based dispatch (used by other call sites/tests) must continue to
work unchanged.

## Required tests (two-commit red/green)

1. Regression test: the issue's exact repro — assert `str(e)` equals just
   the human-readable message, not the tuple repr.
2. Confirm `e.code` (or however code access is currently exposed) is
   unchanged and still dispatchable (positive control).

## Out of scope

- FIX-374 (`parse_nodes` error-code forwarding) — separate worktree/branch,
  same file, do not bundle.
- Any change to which exceptions carry which `.code` values.

## Acceptance criteria

- `cargo test --workspace` (or the relevant Python-binding test suite)
  passes, including the new regression test.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #375's exact repro now prints just the human-readable message via
  `str(e)`.
- Closeout Evidence records the fix commit and confirms `.code`-based
  dispatch is unaffected.

## References

- Issue #375: https://github.com/randlee/sc-compose/issues/375
- `bindings/sc-sha-python/src/lib.rs` (`ScShaError`)

## Closeout Evidence

- Status: **complete**.
- Red regression baseline: `1356f8f` (`test: require readable sc-sha
  exception strings`) reproduced Python's raw `(code, message)` tuple repr
  from `str(e)` while confirming `.code` remained dispatchable.
- Implementation: `ef0a601` (`fix: render sc-sha errors with their message`)
  adds `ScShaError.__str__`, returning only the human-readable message while
  preserving the separate `.code` attribute.
- The exact input-error repro now renders the message directly via `str(e)`;
  `.code == "SC_SHA_INVALID_INPUT"` remains unchanged.
- Validation: `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` pass. The rebuilt maturin wheel passes all 8
  `bindings/sc-sha-python/tests` tests.
