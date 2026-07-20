---
id: D.2-py
title: Python Bindings — Multi-Pass Composition Pipeline
status: complete
branch: sprint/d-2-py-bindings
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/d-2-py-bindings
target: integrate/phase-d
---

# Sprint D.2-py — Python Bindings — Multi-Pass Composition Pipeline

## Goal

- Expose D.2's library-owned multi-pass composition surface to Python through
  the existing PyO3 `bindings/python` crate.
- Wrap `render_all(parsed, contexts)` using the documented per-pass context
  contract `list[tuple[int, dict[str, Any]]]`.
- Extend `ComposePolicy` so Python callers can supply and inspect
  `passes: list[PassConfig]`.
- Preserve backward compatibility for the shipped Phase C Python API and keep
  `compose()` as the only high-level Python composition entry point.

## Scope

All implementation work is confined to `bindings/python` plus the sprint
documentation and planning index needed to record the shipped surface.

This sprint is adapter-only. It does not change `sc-composer` or `sc-compose`
behavior, and it does not expose CLI-only multi-pass flags.

## Exact Targets

- `bindings/python/src/convert.rs`
- `bindings/python/src/types.rs`
- `bindings/python/src/functions.rs`
- `bindings/python/python/sc_compose/__init__.py`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`
- `docs/project-plan.md`

## Deliverables

- `render_all(parsed, contexts) -> str`
- `ComposePolicy(..., passes=None)` constructor support
- `ComposePolicy.passes -> list[PassConfig]`
- Updated `ComposePolicy.__repr__` with pass-count visibility
- Smoke coverage proving:
  - `render_all()` success
  - mapped config failures for context-count and pass-number mismatch
  - `ComposePolicy.passes` round-trip
  - stacked-header success through existing `compose()`

## Backward Compatibility

- Existing `compose()`, `validate()`, and `parse_template_document()` behavior
  remains unchanged for single-pass callers.
- No second high-level compose wrapper is introduced.
- Pre-existing Python exports keep their current names and semantics.

## Acceptance Criteria

- `render_all()` renders a 2-pass template correctly from ordered pass
  contexts.
- `render_all()` maps context-count and pass-number mismatches through the
  existing config error hierarchy.
- `ComposePolicy(passes=[PassConfig(2), PassConfig(1)]).passes` round-trips in
  order.
- `compose()` renders a stacked-header template when given
  `ComposePolicy(passes=[...])`.
- `render_all` is exported from `sc_compose`, stubbed in `_native.pyi`, and
  exercised by smoke tests.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
