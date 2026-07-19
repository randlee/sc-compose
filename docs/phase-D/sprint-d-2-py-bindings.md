---
id: D.2-py
title: Python Bindings — Multi-Pass Composition Pipeline
status: planned
branch: sprint/d-2-py-bindings
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

- `bindings/python/src/convert.rs` — add `extract_pass_contexts` helper for
  `list[tuple[int, dict[str, Any]]] ->
  Vec<(u8, BTreeMap<VariableName, InputValue>)>`
- `bindings/python/src/types.rs` — extend `PyComposePolicy` with `passes`
  constructor support, getter, and repr updates
- `bindings/python/src/functions.rs` — add `render_all(parsed, contexts)`
  wrapper and any supporting registration updates
- `bindings/python/src/lib.rs` — no structural change expected; function/type
  registration still flows through `types::register` / `functions::register`
- `bindings/python/python/sc_compose/__init__.py` — re-export `render_all`
  and any newly surfaced names in the import block and `__all__`
- `bindings/python/python/sc_compose/_native.pyi` — type stubs for
  `ComposePolicy.passes` and `render_all`
- `bindings/python/tests/test_smoke.py` — multi-pass render-all and
  `ComposePolicy.passes` smoke coverage
- `docs/phase-D/sprint-d-2-py-bindings.md` — this document

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `D1` — Programmatic `render_all()` Python wrapper (wraps D.2)
  - New `render_all(parsed: ParsedTemplate, contexts: list[tuple[int, dict[str, Any]]]) -> str`
    wrapper in `bindings/python/src/functions.rs`
  - Delegates directly to `sc_composer::render_all`
  - Reuses the existing error-mapping hierarchy (`ScComposeError`,
    `ScConfigError`, `ScValidationError`) with no new Python-only exception
    types
  - Preserves the Rust pass ordering contract: callers provide contexts
    outer-to-inner, and mismatched counts or pass numbers raise the same
    underlying config errors as the Rust API

- `D2` — `ComposePolicy.passes` Python exposure (wraps D.2 + D.1-py)
  - `ComposePolicy.__init__(..., passes=None)` accepts
    `list[PassConfig] | None`
  - `ComposePolicy.passes -> list[PassConfig]` getter returns cloned wrapper
    values in the same order stored in the Rust policy
  - `ComposePolicy.__repr__` includes pass-count information so multi-pass
    policy objects remain inspectable in Python debugging sessions
  - No change to existing `strict_undeclared_variables`,
    `unknown_variable_policy`, `max_include_depth`, `allowed_roots`, or
    `resolver_policy` semantics

- `D3` — Existing `compose()` stacked-header parity proof
  - The existing `compose(request)` wrapper remains the high-level Python entry
    point
  - New smoke coverage demonstrates that a stacked-header template composed via
    `ComposeRequest(..., policy=ComposePolicy(passes=[...]))` renders end to
    end without introducing a second convenience API
  - This sprint records that Python's multi-pass high-level story is
    `compose()` + `ComposePolicy.passes`, not a parallel "CLI-like" wrapper

- `D4` — Import surface, stubs, and tests
  - `__init__.py` re-exports `render_all`
  - `_native.pyi` reflects `ComposePolicy.passes` and the new `render_all`
    function with the documented Python signatures
  - `test_smoke.py` covers:
    - `render_all()` success on a 2-pass template
    - context-count mismatch failure
    - `ComposePolicy.passes` round-trip
    - high-level `compose()` stacked-header success using policy passes

## Required Work

- Add `extract_pass_contexts` to `convert.rs`, accepting Python values of shape
  `[(2, {"team": "wyvern"}), (1, {"task": "test"})]` and converting them into
  the Rust `Vec<(u8, BTreeMap<VariableName, InputValue>)>` contract
- Validate every dict key as a `VariableName` and every dict value through
  `validate_input_value()`, matching existing `extract_var_map()` behavior
- Extend `PyComposePolicy` in `types.rs`:
  - add `passes` to the constructor signature
  - map Python `PassConfig` wrappers into `ComposePolicy.passes`
  - add a `passes` getter and update `__repr__`
- Add `render_all` wrapper to `functions.rs`
- Register the new function in `functions::register`
- Update `__init__.py` and `_native.pyi` in lockstep
- Add `test_smoke.py` coverage for both low-level `render_all()` and the
  existing high-level `compose()` path
- Run the standard Rust and Python validation set

## Explicit Code Samples

### `render_all()` wrapper (functions.rs)

```rust
#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn render_all(
    parsed: PyRef<'_, PyParsedTemplate>,
    contexts: &Bound<'_, PyAny>,
) -> PyResult<String> {
    let contexts = extract_pass_contexts(contexts)?;
    sc_composer::render_all(&parsed.inner, &contexts).map_err(compose_error_to_pyerr)
}
```

### `ComposePolicy(passes=...)` constructor extension (types.rs)

```rust
#[pymethods]
impl PyComposePolicy {
    #[new]
    #[pyo3(signature = (
        strict_undeclared_variables=false,
        unknown_variable_policy="ignore",
        max_include_depth=32,
        allowed_roots=None,
        passes=None
    ))]
    fn new(
        strict_undeclared_variables: bool,
        unknown_variable_policy: &str,
        max_include_depth: u16,
        allowed_roots: Option<&Bound<'_, PyAny>>,
        passes: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: ComposePolicy {
                strict_undeclared_variables,
                unknown_variable_policy: parse_unknown_variable_policy(unknown_variable_policy)?,
                max_include_depth: sc_composer::IncludeDepth::new(max_include_depth),
                allowed_roots: extract_allowed_roots(allowed_roots)?,
                resolver_policy: ResolverPolicy::default(),
                passes: extract_pass_configs(passes)?,
            },
        })
    }
}
```

### `extract_pass_contexts` helper (convert.rs)

```rust
pub(crate) fn extract_pass_contexts(
    value: &Bound<'_, PyAny>,
) -> PyResult<Vec<(u8, BTreeMap<VariableName, InputValue>)>> {
    let mut contexts = Vec::new();
    for item in value.try_iter()? {
        let item = item?;
        let (pass_number, mapping): (u8, Bound<'_, PyAny>) = item.extract()?;
        let variables = extract_var_map(Some(&mapping))?;
        contexts.push((pass_number, variables));
    }
    Ok(contexts)
}
```

### `_native.pyi` additions

```python
class ComposePolicy:
    def __init__(
        self,
        strict_undeclared_variables: bool = False,
        unknown_variable_policy: str = "ignore",
        max_include_depth: int = 32,
        allowed_roots: list[str | PathLike[str] | ConfiningRoot] | None = None,
        passes: list[PassConfig] | None = None,
    ) -> None: ...
    @property
    def passes(self) -> list[PassConfig]: ...


def render_all(
    parsed: ParsedTemplate,
    contexts: list[tuple[int, dict[str, Any]]],
) -> str: ...
```

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

- `AC1` for `D1`
  - `render_all(parsed, [(2, {"team": "wyvern"}), (1, {"task": "test"})])`
    returns the correct fully rendered output for a 2-pass template
  - `render_all()` raises the mapped config error when the number of supplied
    contexts does not match `parsed.passes`
  - `render_all()` raises the mapped config error when a provided context pass
    number does not match the corresponding header pass number

- `AC2` for `D2`
  - `ComposePolicy(passes=[PassConfig(2), PassConfig(1)]).passes` round-trips
    two `PassConfig` wrappers in order
  - Existing `ComposePolicy` constructor call sites without `passes=` behave
    exactly as before

- `AC3` for `D3`
  - The existing `compose()` wrapper successfully renders a stacked-header
    template when given a `ComposeRequest` whose policy includes per-pass
    `PassConfig` entries
  - No second high-level compose function is introduced

- `AC4` for `D4`
  - `render_all` is importable from `sc_compose` and present in `__all__`
  - `_native.pyi` includes the final `render_all` and `ComposePolicy.passes`
    signatures
  - Smoke tests exercise each new public symbol at least once

- `AC5` backward-compat guard
  - All pre-existing Phase C Python exports keep their current names and
    single-pass behavior
  - The existing smoke suite continues to pass without behavioral changes to
    unrelated bindings

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
