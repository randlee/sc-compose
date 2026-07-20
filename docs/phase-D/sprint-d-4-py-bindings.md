---
id: D.4-py
title: Python Bindings — template-init + verify
status: complete
branch: sprint/d-4-py-bindings
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/d-4-py-bindings
target: integrate/phase-d
---

# Sprint D.4-py — Python Bindings — template-init + verify

## Goal

- Expose D.4's library-owned drift-verification surface to Python through the
  existing PyO3 `bindings/python` crate.
- Wrap `verify(request, deployed_path)` and the structured `VerifyResult`
  returned by `sc_composer`.
- Preserve the actual D.4 ownership split: `verify` is library-owned, while
  multi-pass `template-init` remains CLI-owned and therefore out of scope for
  Python bindings.

## Scope

This sprint adds only the library-owned verification surface. It deliberately
does not wrap multi-pass `template-init`, because that logic lives in
`sc-compose` CLI code rather than `sc-composer`.

All work remains confined to `bindings/python` plus the sprint documentation
and planning index needed to record the boundary decision.

## Exact Targets

- `bindings/python/src/types.rs`
- `bindings/python/src/functions.rs`
- `bindings/python/python/sc_compose/__init__.py`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`
- `docs/project-plan.md`

## Deliverables

- `verify(request, deployed_path) -> VerifyResult`
- `VerifyResult` getters for clean/path/text/diff/warnings fields
- Builtin override parity through existing `ComposeRequest.vars_input`
- Import/stub/smoke coverage for verification behavior

## Backward Compatibility

- Verification reuses the existing `ComposeRequest`/`ComposePolicy` shape,
  including any `ComposePolicy.passes` from D.2-py.
- No verify-specific override model is introduced.
- No Python wrapper around template-init is added or implied.

## Acceptance Criteria

- `verify()` returns `VerifyResult` and reports `clean=True` when rendered and
  deployed content match.
- Drifted deployed output yields `clean=False` with unified diff text.
- `RENDER_DATE` / `RENDER_TIMESTAMP` override through `vars_input` makes
  verification deterministic.
- `verify` and `VerifyResult` are exported from `sc_compose`, stubbed in
  `_native.pyi`, and exercised in smoke tests.
- This sprint explicitly records template-init as out of scope because it is
  CLI-owned.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
