---
id: D.1-py
title: Python Bindings — Multi-Pass Library Foundation
status: planned
branch: sprint/d-1-py-bindings
target: integrate/phase-d
---

# Sprint D.1-py — Python Bindings — Multi-Pass Library Foundation

## Goal

- Expose the D.1 stacked-header library surface to Python via the existing
  PyO3 `bindings/python` crate, extending the current wrapper conventions
  rather than introducing a new binding pattern.
- Wrap `ParsedTemplate.passes`, `Frontmatter.pass_number`, and `PassConfig`.
- Wrap the brace-count-aware discovery functions
  `discover_tokens_with_brace_count` and `discover_all_pass_tokens`.
- Preserve full backward compatibility of the shipped Phase C Python surface
  (v1.2.0): every currently-exported name keeps its current behavior and
  single-pass semantics.
- This is the first sprint in the tandem Python-binding sequence
  (see [Phase D README — Python Binding Parity](./README.md#python-binding-parity)).
  It ships immediately after D.1 lands and unblocks nothing else in D.2 — it
  wraps D.1's library surface only.

All work is confined to `bindings/python`. This sprint depends on D.1 having
landed the Rust library surface it wraps. Per repo boundary rules,
`bindings/python` may depend on **`sc-composer` only** — never on
`sc-compose` (the CLI crate) or ATM-specific crates.

## Hard Dependencies

- [Sprint D.1 — Multi-Pass Library Foundation](sprint-d-1-library-foundation.md)
  — `ParsedTemplate.passes`, `Frontmatter::pass_number`, `PassConfig`,
  `discover_tokens_with_brace_count`, `discover_all_pass_tokens`
- [Phase D README](./README.md) — [Python Binding Parity](./README.md#python-binding-parity)
- [docs/architecture.md](../architecture.md) — §3.3 `bindings/python` adapter
  responsibilities and the dependency-direction rules
- [CLAUDE.md](../../CLAUDE.md) — boundary rules 3–5 (Python adapter may depend
  on `sc-composer` only)

## Exact Targets

- `bindings/python/src/types.rs` — `PyPassConfig` wrapper; extend
  `PyParsedTemplate` (add `passes`), `PyFrontmatter` (add `pass_number`)
- `bindings/python/src/functions.rs` — `discover_tokens_with_brace_count`,
  `discover_all_pass_tokens`
- `bindings/python/src/convert.rs` — `extract_variable_names` and
  `extract_metadata_map` helpers if not already present, reused by
  `PyPassConfig::new`
- `bindings/python/src/lib.rs` — no structural change (module registration is
  already delegated to `types::register` / `functions::register`)
- `bindings/python/python/sc_compose/__init__.py` — re-export new names in the
  import block and `__all__`
- `bindings/python/python/sc_compose/_native.pyi` — type stubs for every new
  class, property, and function
- `bindings/python/tests/test_smoke.py` — multi-pass import-surface and
  behavior tests
- `docs/phase-D/sprint-d-1-py-bindings.md` — this document

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `D1` — Multi-pass `ParsedTemplate` and `Frontmatter` surface (wraps D.1)
  - `PyParsedTemplate` gains a `passes -> list[Frontmatter]` getter returning
    every pass in outer-to-inner order.
  - The existing `frontmatter -> Frontmatter | None` getter is **preserved
    unchanged** and delegates to the library's backward-compatible
    `ParsedTemplate::frontmatter()` accessor (returns the first/outermost pass
    for stacked templates, `None` for header-less templates).
  - `PyFrontmatter` gains a `pass_number -> int` getter delegating to
    `Frontmatter::pass_number()`.
  - No change to `body`, `required_variables`, `defaults`, `metadata`,
    `diagnostics` getters.

- `D2` — `PassConfig` exposure (wraps D.1)
  - New `PyPassConfig` wrapper class named `PassConfig` with getters:
    `pass_number -> int`, `required_variables -> list[VariableName]`,
    `defaults -> dict[str, Any]`, `metadata -> dict[str, Any]`.
  - Constructible from Python:
    `PassConfig(pass_number, required_variables=None, defaults=None, metadata=None)`
    so Python callers can assemble per-pass config (mirrors how the existing
    `PyComposePolicy.__new__` builds policy inputs).
  - `PyComposePolicy.passes` (a constructor keyword and getter accepting
    `list[PassConfig]`) is **out of scope for this sprint** — it is only
    meaningful once D.2's multi-pass compose loop exists to consume it. It
    belongs to D.2-py. This sprint only ships the standalone `PassConfig`
    class so it exists for D.2-py to wire in.

- `D3` — Brace-count-aware discovery functions (wraps D.1)
  - `discover_tokens_with_brace_count(text: str, brace_count: int) -> list[VariableName]`
    — delegates to `sc_composer::discover_tokens_with_brace_count`.
  - `discover_all_pass_tokens(parsed: ParsedTemplate) -> dict[int, list[VariableName]]`
    — delegates to `sc_composer::discover_all_pass_tokens`, keyed by pass
    number.
  - Existing `discover_tokens(text) -> list[VariableName]` is unchanged
    (double-brace default, backward compat).

- `D4` — Import surface, stubs, and tests
  - `__init__.py` re-exports (import block + `__all__`): `PassConfig`,
    `discover_tokens_with_brace_count`, `discover_all_pass_tokens`.
  - `_native.pyi` stubs added for every new class/property/function, matching
    the existing stub style.
  - `test_smoke.py`: extend `test_import_surface_exposes_c2_api` (or add a
    parallel `test_import_surface_exposes_d1_py_api`) and add behavior tests
    (see Acceptance Criteria).

## Required Work

- Add `PyPassConfig` to `types.rs` following the `PyComposePolicy` pattern
  (`#[pyclass(name = "PassConfig", skip_from_py_object)]`, `inner: PassConfig`,
  getters, `#[new]` constructor, `__repr__`).
- Extend `PyParsedTemplate` with a `passes` getter mapping
  `self.inner.passes()` → `Vec<PyFrontmatter>`; leave `frontmatter` untouched.
- Extend `PyFrontmatter` with a `pass_number` getter.
- Add `extract_variable_names` and `extract_metadata_map` helpers to
  `convert.rs` if the existing helpers don't already cover these shapes
  (`list[str | VariableName]` → `Vec<VariableName>`,
  `dict[str, Any]` → `BTreeMap<String, MetadataValue>`), reusing
  `extract_var_map` / `py_to_json_value` conventions already in the file.
- Add `discover_tokens_with_brace_count` and `discover_all_pass_tokens` to
  `functions.rs`, mapping errors with the existing `compose_error_to_pyerr` /
  `render_error_to_pyerr` helpers — **no new error types**.
- Register `PyPassConfig` in `types::register` and both new functions in
  `functions::register`.
- Update `__init__.py` and `_native.pyi` in lockstep with the Rust surface.
- Write `test_smoke.py` coverage.
- Run `cargo fmt`, `cargo clippy`, `cargo test` for the crate, then the
  maturin-backed `pytest` smoke suite.

## Explicit Code Samples

### `PyPassConfig` wrapper (types.rs)

```rust
#[pyclass(name = "PassConfig", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyPassConfig {
    pub(crate) inner: PassConfig,
}

#[pymethods]
impl PyPassConfig {
    #[new]
    #[pyo3(signature = (pass_number, required_variables=None, defaults=None, metadata=None))]
    fn new(
        pass_number: u8,
        required_variables: Option<&Bound<'_, PyAny>>,
        defaults: Option<&Bound<'_, PyAny>>,
        metadata: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: PassConfig {
                pass_number,
                required_variables: extract_variable_names(required_variables)?,
                defaults: extract_var_map(defaults)?,
                metadata: extract_metadata_map(metadata)?,
            },
        })
    }

    #[getter]
    fn pass_number(&self) -> u8 {
        self.inner.pass_number
    }

    #[getter]
    fn required_variables(&self) -> Vec<PyVariableName> {
        self.inner
            .required_variables
            .iter()
            .cloned()
            .map(|inner| PyVariableName { inner })
            .collect()
    }
    // defaults / metadata getters mirror PyFrontmatter::defaults / ::metadata
}
```

### `PyParsedTemplate.passes` getter (backward-compatible)

```rust
#[pymethods]
impl PyParsedTemplate {
    // UNCHANGED — preserves Phase C behavior (outermost pass for stacked
    // templates, None for header-less input).
    #[getter]
    fn frontmatter(&self) -> Option<PyFrontmatter> {
        self.inner
            .frontmatter()
            .cloned()
            .map(|inner| PyFrontmatter { inner })
    }

    // NEW — full multi-pass shape, outer-to-inner order.
    #[getter]
    fn passes(&self) -> Vec<PyFrontmatter> {
        self.inner
            .passes()
            .iter()
            .cloned()
            .map(|inner| PyFrontmatter { inner })
            .collect()
    }

    #[getter]
    fn body(&self) -> String {
        self.inner.body().to_owned()
    }
}
```

### `discover_tokens_with_brace_count` / `discover_all_pass_tokens` (functions.rs)

```rust
#[pyfunction]
fn discover_tokens_with_brace_count(text: &str, brace_count: usize) -> Vec<PyVariableName> {
    sc_composer::discover_tokens_with_brace_count(text, brace_count)
        .into_iter()
        .map(|inner| PyVariableName { inner })
        .collect()
}

#[pyfunction]
fn discover_all_pass_tokens(
    parsed: PyRef<'_, PyParsedTemplate>,
) -> BTreeMap<usize, Vec<PyVariableName>> {
    sc_composer::discover_all_pass_tokens(&parsed.inner)
        .into_iter()
        .map(|(pass, tokens)| {
            (
                pass,
                tokens.into_iter().map(|inner| PyVariableName { inner }).collect(),
            )
        })
        .collect()
}
```

### `_native.pyi` stub additions

```python
class PassConfig:
    def __init__(
        self,
        pass_number: int,
        required_variables: list[str | VariableName] | None = None,
        defaults: dict[str, Any] | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> None: ...
    @property
    def pass_number(self) -> int: ...
    @property
    def required_variables(self) -> list[VariableName]: ...
    @property
    def defaults(self) -> dict[str, Any]: ...
    @property
    def metadata(self) -> dict[str, Any]: ...


def discover_tokens_with_brace_count(text: str, brace_count: int) -> list[VariableName]: ...
def discover_all_pass_tokens(parsed: ParsedTemplate) -> dict[int, list[VariableName]]: ...
```

## This Sprint Does Not Close

- Any change to the `sc-composer` library or `sc-compose` CLI behavior — this
  sprint is binding-only and assumes D.1 has landed.
- `render_all` / multi-pass `compose()` Python exposure and
  `PyComposePolicy.passes` — deferred to D.2-py (needs D.2's compose loop).
- `verify` and `template-init` Python exposure — deferred to D.4-py (needs
  D.4's library surface).
- Exposing any D.3 CLI flags (`--all`, `--pass N`, `--variable-delimiters`) to
  Python — only underlying library functions are ever wrapped, and D.3 has
  not landed yet regardless.
- A high-level Pythonic multi-pass convenience API (e.g. a `MultiPassTemplate`
  facade) beyond thin wrappers over the library functions.
- Async / streaming rendering surfaces.
- Wheel-publishing / release-version bump for the multi-pass surface (handled
  by the phase-end promotion of `integrate/phase-d`).

## Acceptance Criteria

- `AC1` for `D1`
  - `parse_template_document("---\npass: 2\n---\n---\n---\nbody").passes` has
    length 2 with `passes[0].pass_number == 2`.
  - `parse_template_document("hello").frontmatter is None` and
    `.passes == []` (backward compat preserved).
  - For a single-header template, `frontmatter` returns the same object shape
    as Phase C.
- `AC2` for `D2`
  - `PassConfig(2, required_variables=["team"]).pass_number == 2` and
    `.required_variables` returns `[VariableName("team")]`.
  - `PassConfig` round-trips its `defaults` and `metadata` dict arguments
    through the corresponding getters.
- `AC3` for `D3`
  - `discover_tokens_with_brace_count("{{{ a }}}", 3)` returns `["a"]`.
  - `discover_tokens_with_brace_count("{{{ outer }}} {{ inner }}", 3)` returns
    `["outer"]` only.
  - `discover_all_pass_tokens(parsed)` returns a `dict[int, list[VariableName]]`
    keyed by pass number.
- `AC4` for `D4`
  - Every new name is importable from `sc_compose` and present in `__all__`.
  - `_native.pyi` type-checks (no `mypy`/`pyright` regressions in the smoke
    suite's configured checker).
- `AC5` backward-compat guard
  - Every Phase C exported name and behavior is unchanged.
  - The full existing `test_smoke.py` suite passes without modification (new
    tests are additive).

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose-py` (crate unit tests, incl. new wrapper tests)
- `maturin develop` + `pytest bindings/python/tests` (wheel smoke suite,
  including the new multi-pass tests)
- `git diff --check`
