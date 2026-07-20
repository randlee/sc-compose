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
- Wrap `sc_composer::render_all(parsed, contexts)` using the canonical
  per-pass `contexts` contract from
  [ADR-0009](../adrs/0009-phase-d-python-binding-parity-sequencing.md):
  `&[(u8, BTreeMap<VariableName, InputValue>)]`, adapted for Python as
  `list[tuple[int, dict[str, Any]]]`.
- Extend `ComposePolicy` so Python callers can supply `passes:
  list[PassConfig]`, reusing the `PassConfig` wrapper introduced by D.1-py.
- Prove that the existing `compose()` Python binding auto-detects stacked
  headers end to end once D.2's Rust compose loop is present; this sprint does
  not introduce a second high-level compose entry point.
- Preserve backward compatibility of the shipped Phase C Python API: existing
  single-pass `compose()`, `validate()`, and `parse_template_document()`
  behavior remains unchanged.

All work is confined to `bindings/python`. This sprint depends on D.2's Rust
surface having passed QA and merged to `integrate/phase-d`. Per repo boundary
rules, `bindings/python` may depend on **`sc-composer` only** — never on
`sc-compose` (the CLI crate) or ATM-specific crates.

The current `bindings/python/src/` state confirms the gap this sprint closes:
the adapter exports `compose()`, `validate()`, `parse_template_document()`,
and single-pass `ComposePolicy`, but it does **not** yet expose `render_all()`
or `ComposePolicy.passes`.

## Scope

All implementation work is confined to `bindings/python` plus the sprint
documentation and planning index needed to record the shipped surface.

This sprint is adapter-only. It does not change `sc-composer` or `sc-compose`
behavior, and it does not expose CLI-only multi-pass flags.

## Hard Dependencies

- [Sprint D.2 — Multi-Pass Composition Pipeline](sprint-d-2-composition-pipeline.md)
  — `render_all()`, multi-pass `compose()`, `protect_higher_braces()`
- [Sprint D.1-py — Python Bindings — Multi-Pass Library Foundation](sprint-d-1-py-bindings.md)
  — `PassConfig`, multi-pass parsing wrappers, wrapper conventions
- [Phase D README](./README.md) — [Python Binding Parity](./README.md#python-binding-parity)
- [ADR-0009: Phase D Python-Binding Parity Sequencing](../adrs/0009-phase-d-python-binding-parity-sequencing.md)
  — canonical `contexts` contract and tandem dispatch rules
- [docs/architecture.md](../architecture.md) — `bindings/python` adapter
  responsibilities and crate-boundary rules
- [CLAUDE.md](../../CLAUDE.md) — boundary rules 3–5 (`bindings/python` may
  depend on `sc-composer` only)

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

## This Sprint Does Not Close

- Any `sc-composer` or `sc-compose` Rust implementation work — this sprint is
  adapter-only and assumes D.2 has already passed QA and merged
- Any Python wrapper over CLI-only D.3 surface such as `--all`, `--pass N`,
  `--var-file`, `--brace-count`, or `--variable-delimiters`
- `verify()` Python exposure — deferred to D.4-py
- Multi-pass `template-init` Python exposure — deferred to D.4-py, and only
  the library-owned portion of that sprint is even eligible for binding
- A higher-level Pythonic multi-pass convenience object beyond thin wrappers
  over `ComposePolicy`, `compose()`, and `render_all()`
- Async or streaming multi-pass rendering helpers

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
