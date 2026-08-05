---
id: D.1-py
title: Python Bindings — Multi-Pass Library Foundation
status: complete
branch: sprint/d-1-py-bindings
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/d-1-py-bindings
target: integrate/phase-d
---

# Sprint D.1-py — Python Bindings — Multi-Pass Library Foundation

## Goal

- Expose the D.1 stacked-header library surface to Python via the existing
  PyO3 `bindings/python` crate.
- Wrap `ParsedTemplate.passes`, `Frontmatter.pass_number`, and `PassConfig`.
- Wrap `discover_tokens_with_brace_count` and
  `discover_all_pass_tokens`.
- Preserve full backward compatibility for the Phase C Python API.

## Scope

All work is confined to `bindings/python` plus the sprint documentation needed
to record the shipped adapter surface.

This sprint wraps D.1 library functionality only. It does not add Python
compose-loop wiring, reporting extensions, or new CLI behavior.

## Exact Targets

- `bindings/python/src/types.rs`
- `bindings/python/src/functions.rs`
- `bindings/python/src/convert.rs`
- `bindings/python/python/sc_compose/__init__.py`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`
- `docs/project-plan.md`

## Deliverables

- `ParsedTemplate.passes -> list[Frontmatter]` in outer-to-inner order.
- `Frontmatter.pass_number -> int`.
- `PassConfig` Python wrapper with constructor and getters for
  `pass_number`, `required_variables`, `defaults`, and `metadata`.
- `discover_tokens_with_brace_count(text, brace_count)`.
- `discover_all_pass_tokens(parsed)`.
- Updated import surface, stubs, and smoke coverage.

## Backward Compatibility

- `ParsedTemplate.frontmatter` remains unchanged and continues to expose only
  the outermost frontmatter block.
- `discover_tokens(text)` keeps the existing double-brace behavior.
- Headerless templates continue returning `frontmatter is None` and
  `passes == []`.
- No existing Phase C export changes behavior.

## Acceptance Criteria

- Parsing stacked headers exposes both `frontmatter` and full `passes`.
- `PassConfig` round-trips `required_variables`, `defaults`, and `metadata`.
- Brace-count token discovery distinguishes `{{ ... }}` from `{{{ ... }}}`.
- All new names are importable from `sc_compose` and present in `__all__`.
- The existing smoke suite still passes with additive tests only.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
