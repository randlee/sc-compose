---
id: FIX-374
status: complete
branch: fix/374-sc-sha-digest-error-code
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/fix/374-sc-sha-digest-error-code
target: integrate/phase-M
---

# Sprint FIX-374 — `parse_nodes` Hardcodes The Wrong Error Code For Malformed Manifest Digests

## Problem

Issue #374, found by adversarial fuzzing of the M.2 sc-sha-python FFI
boundary: `sc_sha.calculate_composition_hash()` given a node with a
malformed hex digest (e.g. `"not-hex"`) raises `ScShaError` with
`code == "SC_SHA_INVALID_MANIFEST"` instead of the correct
`"SC_SHA_INVALID_DIGEST"` per `docs/error-code-registry.md` and
`sc_sha::ShaError::InvalidDigestHex.code()`.

## Root cause

`parse_digest` in `bindings/sc-sha-python/src/lib.rs` (~lines 99-106)
discards `ShaError`'s `.code()` and returns a bare `&'static str`. Its
caller `parse_nodes` (~lines 133-134) then hardcodes the wrapper error as
`SC_SHA_INVALID_MANIFEST` instead of forwarding the real stable code —
unlike `calculate_hash_py`/`calculate_composition_hash_py`, which correctly
forward typed error codes elsewhere in the same file.

## Fix design

Change `parse_digest` to return the real `ShaError` (or its `.code()` plus
message) instead of discarding it, and have `parse_nodes` forward that code
through to the `ScShaError` it raises, instead of hardcoding
`SC_SHA_INVALID_MANIFEST`. Follow the same forwarding pattern already used
correctly by `calculate_hash_py`/`calculate_composition_hash_py`.

## Required tests (two-commit red/green)

1. Regression test: the issue's exact repro (a node with `sha256:
   "not-hex"`) — assert `ScShaError.code == "SC_SHA_INVALID_DIGEST"`.
2. Confirm other `parse_nodes` failure modes (e.g. genuinely malformed
   manifest shape) still correctly raise `SC_SHA_INVALID_MANIFEST` where
   that classification is actually correct (positive control — don't
   overcorrect and lose the legitimate use of that code).

## Out of scope

- Any change to `calculate_hash_py`/`calculate_composition_hash_py`'s
  existing (correct) error-forwarding pattern.
- FIX-375 (`ScShaError.__str__`) — separate worktree/branch, same file,
  do not bundle.

## Acceptance criteria

- `cargo test --workspace` (or the relevant Python-binding test suite)
  passes, including the new regression test.
- `cargo fmt --all --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Issue #374's exact repro now raises `SC_SHA_INVALID_DIGEST`.
- Closeout Evidence records the fix commit and confirms no other
  `parse_nodes` error path regressed.

## References

- Issue #374: https://github.com/randlee/sc-compose/issues/374
- `bindings/sc-sha-python/src/lib.rs` (`parse_digest`, `parse_nodes`)
- `docs/error-code-registry.md`
- `sc_sha::ShaError::InvalidDigestHex`

## Closeout Evidence

- Status: **complete**.
- Red regression baseline: `f0439e0` (`test: preserve sc-sha digest error
  code`) reproduced `SC_SHA_INVALID_MANIFEST` for a malformed `sha256`
  digest, while the malformed-manifest control remained correctly classified.
- Implementation: `6784f32` (`fix: forward malformed digest error code`)
  preserves the typed `sc_sha::ShaError` from `parse_digest` and forwards its
  stable code and message through `parse_nodes`.
- The exact malformed-hex repro now raises `SC_SHA_INVALID_DIGEST`;
  malformed manifest shape, invalid UTF-8, and unsupported schema controls
  retain their existing codes.
- Validation: `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `git diff --check` pass. The rebuilt maturin wheel passes all 8
  `bindings/sc-sha-python/tests` tests.
