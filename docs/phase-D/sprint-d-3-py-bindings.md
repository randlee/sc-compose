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

- Audit the Python adapter against the actual D.3 Rust landing and document the
  correct outcome: D.3 is primarily a `sc-compose` CLI sprint, so Python does
  **not** mirror its `--all`, `--pass N`, `--var`, `--var-file`,
  `--brace-count`, or `--variable-delimiters` grammar.
- Confirm that the D.1-py and D.2-py wrappers continue to bind against the
  consolidated D.3 `sc_composer` re-exports without import-path churn or
  symbol loss.
- Make the "nothing new to wrap at the Python public API layer" result
  explicit and testable, rather than leaving D.3-py as a silent stub.
- Preserve the repo boundary rule that `bindings/python` wraps library-owned
  `sc-composer` APIs only; it does not expose `sc-compose` CLI parsers or
  shell out to the CLI as a surrogate binding layer.

This sprint is intentionally smaller than D.2-py or D.4-py, but it is still a
real sprint. The tandem policy in
[ADR-0009](../adrs/0009-phase-d-python-binding-parity-sequencing.md) commits to
an explicit Python companion sprint after each Rust sprint, even when the
result is "parity audit plus regression coverage" rather than a large new
binding surface.

The current `bindings/python/src/` state confirms the gap this sprint closes:
the adapter already exports library-level wrappers such as `parse_template_document`,
`discover_tokens`, and the Phase C/Phase D foundational types, but it has no
business exposing CLI-only pass-group parsing helpers from `sc-compose`.

## Scope

This sprint is a parity/audit sprint, not a stub and not a new binding-surface
expansion. Work remains confined to `bindings/python` plus the sprint
documentation and planning index needed to record the deliberate boundary.

No `sc-compose` CLI modules, parsers, subprocess facades, or command grammar
belong in the Python package.

## Hard Dependencies

- [Sprint D.3 — Multi-Pass CLI Surface](sprint-d-3-cli-surface.md) — actual
  Rust landing being audited
- [Sprint D.1-py — Python Bindings — Multi-Pass Library Foundation](sprint-d-1-py-bindings.md)
  — foundational wrapper conventions and symbols D.3-py must preserve
- [Sprint D.2-py — Python Bindings — Multi-Pass Composition Pipeline](sprint-d-2-py-bindings.md)
  — `render_all()` and `ComposePolicy.passes` are the main Python-exposed
  surfaces whose import stability D.3-py must protect
- [Phase D README](./README.md) — [Python Binding Parity](./README.md#python-binding-parity)
- [docs/architecture.md](../architecture.md) — `bindings/python` remains a
  library adapter, not a CLI facade
- [CLAUDE.md](../../CLAUDE.md) — boundary rules 3–5 (`bindings/python` may
  depend on `sc-composer` only)

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

## Explicit Code Samples

### Public-path parity inside the adapter

```rust
use sc_composer::{
    ComposePolicy,
    ParsedTemplate,
    PassConfig,
    VariableName,
    discover_all_pass_tokens,
    discover_tokens_with_brace_count,
    render_all,
};
```

The point of D.3-py is not to create a second Python-visible symbol family.
It is to ensure the adapter keeps targeting the stable public Rust surface
after D.3's re-export consolidation.

### Negative boundary test (Python smoke)

```python
import sc_compose


def test_python_surface_remains_library_only() -> None:
    assert not hasattr(sc_compose, "parse_pass_inputs")
    assert not hasattr(sc_compose, "filtered_args_for_clap")
    assert not hasattr(sc_compose, "run_template_init")
```

### Positive parity test

```python
from sc_compose import ComposePolicy, PassConfig, render_all


def test_d2_py_symbols_remain_importable_after_d3() -> None:
    policy = ComposePolicy(passes=[PassConfig(2), PassConfig(1)])
    assert len(policy.passes) == 2
    assert callable(render_all)
```

## Backward Compatibility

- No new Python public symbol is added unless D.3 introduced a genuinely new
  library-owned `sc_composer` API worth binding.
- Existing D.1-py and D.2-py exports remain importable and behaviorally
  unchanged.
- Python continues to wrap `sc-composer` only; it does not become a CLI facade.

## This Sprint Does Not Close

- Any Python binding that mirrors the `sc-compose` CLI argument grammar
- Any Python-side wrapper over `--all`, `--pass N`, `--var`, `--var-file`,
  `--brace-count`, or `--variable-delimiters`
- `verify()` bindings — deferred to D.4-py
- Multi-pass `template-init` bindings — deferred to D.4-py, subject to the
  library/CLI boundary documented there
- Any new `sc-composer` implementation work
- Any change to the repo rule that `bindings/python` may depend on
  `sc-composer` only

## Acceptance Criteria

- The adapter compiles and tests cleanly against the stabilized D.3
  `sc_composer` public surface.
- The Python package does not expose CLI-only parser helpers or pass-group
  grammar APIs.
- Smoke tests prove D.1-py and D.2-py multi-pass symbols remain usable after
  D.3.
- This sprint is documented as a deliberate parity/audit sprint with explicit
  success criteria.
- The sprint doc matches the same full-rigor section structure as D.1-py,
  D.2-py, and D.4-py.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
