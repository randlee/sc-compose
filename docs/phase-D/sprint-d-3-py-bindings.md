---
id: D.3-py
title: Python Bindings — Multi-Pass CLI Surface Parity Check
status: planned
branch: sprint/d-3-py-bindings
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
  library adapter, not a CLI façade
- [CLAUDE.md](../../CLAUDE.md) — boundary rules 3–5 (`bindings/python` may
  depend on `sc-composer` only)

## Exact Targets

- `bindings/python/src/functions.rs` — audit call-sites against D.3's
  consolidated `sc_composer` public re-exports; update import paths only if the
  wrappers still reference pre-re-export internal paths
- `bindings/python/src/types.rs` — same audit for wrapper-owned type imports
  (`PassConfig`, `ParsedTemplate`, discovery helpers, etc.)
- `bindings/python/python/sc_compose/__init__.py` — no net-new CLI symbols;
  update only if the documented Python import surface needs clarification
- `bindings/python/python/sc_compose/_native.pyi` — no CLI-only additions;
  adjust type stubs only if D.3's library re-export consolidation changes the
  surfaced names or signatures D.1-py / D.2-py rely on
- `bindings/python/tests/test_smoke.py` — add parity regression tests proving
  the documented Python surface remains library-only and that D.1-py/D.2-py
  symbols still import and behave as expected
- `docs/phase-D/sprint-d-3-py-bindings.md` — this document

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `D1` — Re-export parity audit against D.3
  - Confirm the Python adapter uses the stable `sc_composer` public API
    exposed after D.3's GAP-11 re-export consolidation
  - Update binding internals only if D.3's re-export shape makes any existing
    import path stale
  - No net-new Python symbol is required merely because D.3 consolidated Rust
    re-exports

- `D2` — Explicit library-vs-CLI boundary lock
  - Record and test that Python does **not** expose the `sc-compose` CLI's
    pass-grouping grammar (`--all`, `--pass`, `--var`, `--var-file`) or
    delimiter flags
  - Python's multi-pass story remains library-shaped:
    `ComposePolicy.passes`, `render_all()`, `compose()`, and later `verify()`
  - The adapter remains free of dependencies on `sc-compose` command modules

- `D3` — Import surface and smoke regression coverage
  - Add smoke tests exercising the D.1-py / D.2-py symbols most likely to be
    impacted by D.3 re-export churn:
    `PassConfig`, `ParsedTemplate`, `discover_tokens_with_brace_count`,
    `discover_all_pass_tokens`, `render_all`, and `ComposePolicy.passes`
  - Add a negative regression check that the Python package still does not
    export CLI parser helpers or CLI-only pass-group interfaces

- `D4` — Documentation-level closure of the "stub" state
  - D.3-py is documented as a deliberate parity/audit sprint with explicit
    acceptance criteria
  - Future reviews can evaluate D.3-py on a concrete contract instead of
    guessing whether "no new symbol" was intentional or accidental

## Required Work

- Audit the actual D.3 Rust landing and confirm which items are library-owned
  versus CLI-only
- Inspect `bindings/python/src/functions.rs` and `types.rs` for any direct
  references that should instead flow through the stabilized `sc_composer`
  public surface
- Update `__init__.py` / `_native.pyi` only if needed for consistency with the
  true library surface; do **not** add CLI-only names
- Add smoke tests that:
  - import and exercise the D.1-py / D.2-py multi-pass bindings
  - assert the package does not expose CLI pass-parser helpers or a Python-side
    clone of the `sc-compose` CLI grammar
- Keep the adapter crate boundary clean: no `sc-compose` dependency, no CLI
  command-module imports, no subprocess façade

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
    assert not hasattr(sc_compose, "template_init_cli")
```

### Positive parity test

```python
from sc_compose import ComposePolicy, PassConfig, render_all


def test_d2_py_symbols_remain_importable_after_d3() -> None:
    policy = ComposePolicy(passes=[PassConfig(2), PassConfig(1)])
    assert len(policy.passes) == 2
    assert callable(render_all)
```

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

- `AC1` for `D1`
  - The Python adapter compiles and tests cleanly against the stabilized D.3
    `sc_composer` public surface
  - No wrapper remains coupled to an obsolete pre-D.3 internal import path

- `AC2` for `D2`
  - The Python package exposes no CLI-only parser or flag-grammar API
  - D.3-py's documentation states that this omission is intentional, not a
    gap or oversight

- `AC3` for `D3`
  - Smoke tests prove the D.1-py / D.2-py multi-pass library symbols remain
    importable and usable after D.3
  - No net-new Python public symbol is added unless the D.3 Rust landing
    genuinely introduced a new library-owned `sc_composer` API worth binding

- `AC4` for `D4`
  - The sprint doc matches the same full-rigor section structure as D.1-py,
    D.2-py, and D.4-py
  - Future reviewers can evaluate D.3-py on explicit success/failure criteria

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
