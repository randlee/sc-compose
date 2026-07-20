---
id: D.3-py
title: Python Bindings — Multi-Pass CLI Surface Parity Check
status: complete
branch: sprint/d-3-py-bindings
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/d-3-py-bindings
target: integrate/phase-d
---

# Sprint D.3-py — Python Bindings — Multi-Pass CLI Surface Parity Check

## Goal

- Audit the Python adapter against the actual D.3 Rust landing and confirm the
  right outcome: D.3 is primarily a `sc-compose` CLI sprint, so Python does
  not mirror its CLI grammar or flags.
- Verify the D.1-py and D.2-py library bindings remain stable and importable
  after D.3's `sc_composer` public-surface consolidation.
- Make the “no new Python symbol” result explicit, intentional, and tested.

## Scope

This sprint is a parity/audit sprint, not a stub and not a new binding-surface
expansion. Work remains confined to `bindings/python` plus the sprint
documentation and planning index needed to record the deliberate boundary.

No `sc-compose` CLI modules, parsers, subprocess facades, or command grammar
belong in the Python package.

## Exact Targets

- `bindings/python/src/functions.rs`
- `bindings/python/src/types.rs`
- `bindings/python/python/sc_compose/__init__.py`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`
- `docs/project-plan.md`

## Deliverables

- Re-export parity audit confirming the adapter still targets stable
  `sc_composer` public APIs.
- Explicit regression coverage proving the Python package remains library-only.
- Positive smoke coverage for the existing multi-pass Python symbols most
  exposed to D.3 re-export churn:
  `PassConfig`, `ParsedTemplate`, `discover_tokens_with_brace_count`,
  `discover_all_pass_tokens`, `render_all`, and `ComposePolicy.passes`.
- Documentation stating that omission of CLI grammar from Python is
  intentional, not a gap.

## Backward Compatibility

- No new Python public symbol is added unless D.3 introduced a genuinely new
  library-owned `sc_composer` API worth binding.
- Existing D.1-py and D.2-py exports remain importable and behaviorally
  unchanged.
- Python continues to wrap `sc-composer` only; it does not become a CLI facade.

## Acceptance Criteria

- The adapter compiles and tests cleanly against the stabilized D.3
  `sc_composer` public surface.
- The Python package does not expose CLI-only parser helpers or pass-group
  grammar APIs.
- Smoke tests prove D.1-py and D.2-py multi-pass symbols remain usable after
  D.3.
- This sprint is documented as a deliberate parity/audit sprint with explicit
  success criteria.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
