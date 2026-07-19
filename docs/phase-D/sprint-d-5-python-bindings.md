---
id: D.5
title: Python Bindings for Multi-Pass Composition
status: planned
branch: sprint/d-5-python-bindings
target: integrate/phase-d
---

# Sprint D.5 — Python Bindings for Multi-Pass Composition

> **Draft status:** This is a pre-review draft prepared for skillrx (final
> design sign-off authority for this repo). Signatures and wrapper names below
> are concrete proposals, not finalized decisions. Unresolved boundary/shape
> questions are collected in [Open Design Questions](#open-design-questions-for-skillrx-review)
> and must be settled before implementation begins.

## Goal

- Expose the multi-pass stacked-header surface delivered by D.1–D.4 to Python
  via the existing PyO3 `bindings/python` crate, extending the current wrapper
  conventions rather than introducing a new binding pattern.
- Wrap the D.1 library-foundation types (`ParsedTemplate.passes`,
  `Frontmatter.pass_number`, `PassConfig`) and the brace-count-aware discovery
  functions (`discover_tokens_with_brace_count`, `discover_all_pass_tokens`).
- Wrap the D.2 programmatic multi-pass entry point (`render_all`) and surface
  per-pass config on the Python `ComposePolicy`.
- Wrap the D.4 library `verify()` entry point and the multi-pass
  `template-init` conversion — subject to the CLI/library boundary resolution
  in the open questions below.
- Preserve full backward compatibility of the shipped Phase C Python surface
  (v1.2.0): every currently-exported name keeps its current behavior and
  single-pass semantics.

All work is confined to `bindings/python`. This sprint depends on D.1–D.4
having landed the Rust library surface it wraps. Per repo boundary rules,
`bindings/python` may depend on **`sc-composer` only** — never on `sc-compose`
(the CLI crate) or ATM-specific crates. The CLI itself (`--all`, `--pass N`,
`--variable-delimiters` from D.3) is **not** exposed to Python; only the
underlying `sc-composer` library functions are.

## Hard Dependencies

- [Sprint D.1 — Multi-Pass Library Foundation](sprint-d-1-library-foundation.md)
  — `ParsedTemplate.passes`, `Frontmatter::pass_number`, `PassConfig`,
  `discover_tokens_with_brace_count`, `discover_all_pass_tokens`
- [Sprint D.2 — Multi-Pass Composition Pipeline](sprint-d-2-composition-pipeline.md)
  — `render_all`, multi-pass `compose()` auto-detection
- [Sprint D.3 — Multi-Pass CLI Surface](sprint-d-3-cli-surface.md) — `lib.rs`
  re-exports of the new types (GAP-11); CLI flags themselves are out of scope
- [Sprint D.4 — template-init + verify](sprint-d-4-template-init-verify.md) —
  `sc_composer::verify`, `VerifyResult`, `template-init` conversion
- [Phase D README](./README.md)
- [docs/architecture.md](../architecture.md) — §3.3 `bindings/python` adapter
  responsibilities and the dependency-direction rules
- [CLAUDE.md](../../CLAUDE.md) — boundary rules 3–5 (Python adapter may depend
  on `sc-composer` only)

## Exact Targets

- `bindings/python/src/types.rs` — `PyPassConfig` wrapper; extend
  `PyParsedTemplate` (add `passes`), `PyFrontmatter` (add `pass_number`),
  `PyComposePolicy` (add `passes`); add `PyVerifyResult`
- `bindings/python/src/functions.rs` — `discover_tokens_with_brace_count`,
  `discover_all_pass_tokens`, `render_all`, `verify` (and `template_init` if
  the library-hosted core lands per the open question)
- `bindings/python/src/convert.rs` — `extract_pass_contexts` helper for
  `list[tuple[int, dict]]` → `Vec<(u8, HashMap<VariableName, InputValue>)>`
- `bindings/python/src/lib.rs` — no structural change (module registration is
  already delegated to `types::register` / `functions::register`)
- `bindings/python/python/sc_compose/__init__.py` — re-export new names in the
  import block and `__all__`
- `bindings/python/python/sc_compose/_native.pyi` — type stubs for every new
  class, property, and function
- `bindings/python/tests/test_smoke.py` — multi-pass import-surface and
  behavior tests
- `docs/phase-D/sprint-d-5-python-bindings.md` — this document

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
    so Python callers can assemble `ComposePolicy.passes` (mirrors how the
    existing `PyComposePolicy.__new__` builds policy inputs).
  - `PyComposePolicy` gains a `passes` constructor keyword and getter
    (`passes -> list[PassConfig]`) so multi-pass config can be supplied to
    `compose()`/`validate()` without a CLI. (See open question Q4 on whether
    per-pass config belongs on the policy or is inferred from the template.)

- `D3` — Brace-count-aware discovery functions (wraps D.1)
  - `discover_tokens_with_brace_count(text: str, brace_count: int) -> list[VariableName]`
    — delegates to `sc_composer::discover_tokens_with_brace_count`.
  - `discover_all_pass_tokens(parsed: ParsedTemplate) -> dict[int, list[VariableName]]`
    — delegates to `sc_composer::discover_all_pass_tokens`, keyed by pass
    number.
  - Existing `discover_tokens(text) -> list[VariableName]` is unchanged
    (double-brace default, backward compat).

- `D4` — Programmatic multi-pass rendering (wraps D.2)
  - `render_all(parsed: ParsedTemplate, contexts: list[tuple[int, dict[str, Any]]]) -> str`
    — delegates to `sc_composer::render_all`, preserving its context-count and
    pass-number validation (surfaced as `ScValidationError` / `ScComposeError`).
  - Multi-pass `compose()` requires **no new Python function**: the existing
    `compose(request)` already auto-detects stacked headers via D.2. This
    sprint only ensures per-pass config is reachable (D2 above) and adds a
    smoke test proving a stacked-header template composes end-to-end through
    the existing `compose()` binding.
  - `Renderer.with_delimiters(open, close)` is **already exposed** (Phase C,
    classmethod); no work needed — noted here so it is not re-implemented.

- `D5` — `verify` exposure (wraps D.4)
  - New `PyVerifyResult` wrapper class named `VerifyResult` with getters:
    `clean -> bool`, `diff -> str | None`, `exit_code -> int`.
  - `verify(template_path, deployed_path, contexts, overrides=None) -> VerifyResult`
    — delegates to `sc_composer::verify`. `template_path`/`deployed_path` accept
    `str | PathLike[str]` via the existing `coerce_path_like` helper;
    `contexts` is `list[tuple[int, dict[str, Any]]]`; `overrides` is
    `dict[str, str] | None` for builtin-variable overrides (`RENDER_DATE`,
    `RENDER_TIMESTAMP`).

- `D6` — `template-init` exposure (wraps D.4) — **conditional on Q1**
  - If the multi-pass template-init core is hosted in `sc-composer` (see Q1),
    expose `template_init(path, passes, force=False, dry_run=False) -> FrontmatterInitResult`
    where `passes` is `list[tuple[int, dict[str, str]]]` (pass number → var
    name → concrete value). Reuses the existing `PyFrontmatterInitResult`
    wrapper.
  - The existing single-pass `frontmatter_init(path, force, dry_run)` binding
    is preserved unchanged.
  - If Q1 resolves to "template-init stays CLI-only," this deliverable is
    dropped from D.5 and recorded explicitly under
    [This Sprint Does Not Close](#this-sprint-does-not-close); it is **not**
    silently deferred.

- `D7` — Import surface, stubs, and tests
  - `__init__.py` re-exports (import block + `__all__`): `PassConfig`,
    `VerifyResult`, `discover_tokens_with_brace_count`,
    `discover_all_pass_tokens`, `render_all`, `verify`, and `template_init`
    (if D6 lands).
  - `_native.pyi` stubs added for every new class/property/function, matching
    the existing stub style.
  - `test_smoke.py`: extend `test_import_surface_exposes_c2_api` (or add a
    parallel `test_import_surface_exposes_d5_api`) and add behavior tests
    (see Acceptance Criteria).

## Required Work

- Add `PyPassConfig` to `types.rs` following the `PyComposePolicy` pattern
  (`#[pyclass(name = "PassConfig", skip_from_py_object)]`, `inner: PassConfig`,
  getters, `#[new]` constructor, `__repr__`).
- Extend `PyParsedTemplate` with a `passes` getter mapping
  `self.inner.passes()` → `Vec<PyFrontmatter>`; leave `frontmatter` untouched.
  (Assumes D.1 exposes a public `ParsedTemplate::passes() -> &[Frontmatter]`
  accessor, since the field itself is private per D.1 AC5. If D.1 landed only
  the private field plus `frontmatter()`, a one-line public accessor must be
  added to `sc-composer` — flagged as a prerequisite, not new library design.)
- Extend `PyFrontmatter` with a `pass_number` getter.
- Extend `PyComposePolicy::new` and add a `passes` getter (see Q4).
- Add `PyVerifyResult` to `types.rs`.
- Add `extract_pass_contexts` to `convert.rs` converting a Python
  `list[tuple[int, dict]]` into `Vec<(u8, HashMap<VariableName, InputValue>)>`,
  reusing `extract_var_map` / `py_to_json_value` for the inner dicts and
  raising `ScValidationError` on malformed pass numbers or values.
- Add `discover_tokens_with_brace_count`, `discover_all_pass_tokens`,
  `render_all`, and `verify` to `functions.rs`, mapping errors with the
  existing `compose_error_to_pyerr` / `render_error_to_pyerr` helpers — **no
  new error types**; the existing `ScComposeError` hierarchy covers every
  failure mode (`verify` I/O errors map through `ComposeError::Config` /
  `ScConfigError`).
- Register every new class in `types::register` and every new function in
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

### `render_all` and `verify` functions (functions.rs)

```rust
#[pyfunction]
fn render_all(
    parsed: PyRef<'_, PyParsedTemplate>,
    contexts: &Bound<'_, PyAny>,
) -> PyResult<String> {
    let contexts = extract_pass_contexts(contexts)?;
    sc_composer::render_all(&parsed.inner, &contexts).map_err(compose_error_to_pyerr)
}

#[pyfunction]
#[pyo3(signature = (template_path, deployed_path, contexts, overrides=None))]
fn verify(
    template_path: &Bound<'_, PyAny>,
    deployed_path: &Bound<'_, PyAny>,
    contexts: &Bound<'_, PyAny>,
    overrides: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyVerifyResult> {
    let template_path = coerce_path_like(template_path)?;
    let deployed_path = coerce_path_like(deployed_path)?;
    let contexts = extract_pass_contexts(contexts)?;
    let overrides = extract_string_map_opt(overrides)?;
    sc_composer::verify(template_path, deployed_path, &contexts, overrides)
        .map(|inner| PyVerifyResult { inner })
        .map_err(compose_error_to_pyerr)
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


class VerifyResult:
    @property
    def clean(self) -> bool: ...
    @property
    def diff(self) -> str | None: ...
    @property
    def exit_code(self) -> int: ...


def discover_tokens_with_brace_count(text: str, brace_count: int) -> list[VariableName]: ...
def discover_all_pass_tokens(parsed: ParsedTemplate) -> dict[int, list[VariableName]]: ...
def render_all(
    parsed: ParsedTemplate,
    contexts: list[tuple[int, dict[str, Any]]],
) -> str: ...
def verify(
    template_path: str | PathLike[str],
    deployed_path: str | PathLike[str],
    contexts: list[tuple[int, dict[str, Any]]],
    overrides: dict[str, str] | None = None,
) -> VerifyResult: ...
```

## Open Design Questions (for skillrx review)

These are unresolved and block finalization. Recommendations are the drafter's
proposal, not decisions.

- **Q1 — template-init CLI/library boundary.** D.4's samples place
  `template_init` in the **CLI crate** (`sc-compose/src/commands/template_init.rs`),
  but `bindings/python` may depend on `sc-composer` **only**. The existing
  single-pass `frontmatter_init` is bindable precisely because it lives in the
  library (`sc_composer::frontmatter_init`). To expose multi-pass template-init
  to Python, its conversion core (longest-match-first replacement + stacked
  header generation) must be hosted in `sc-composer`, with the CLI command as a
  thin wrapper. **Recommendation:** amend D.4 (or add a small library-hosting
  task to D.5) so `sc_composer::template_init` exists; otherwise drop D6 and
  record template-init as explicitly CLI-only. This is the single most
  important item to settle.

- **Q2 — class hierarchy vs single flexible wrapper for the multi-pass shape.**
  Should stacked-header templates get a distinct Python class hierarchy (e.g.
  `PassHeader`, `MultiPassTemplate`) or extend the existing single wrappers?
  **Recommendation:** extend `ParsedTemplate`/`Frontmatter` in place (add
  `passes` / `pass_number`), mirroring the library's own backward-compatible
  accessor strategy from D.1's AC5. No new class hierarchy — a single wrapper
  carries both shapes. Confirm this is acceptable vs. a cleaner but
  breaking-adjacent split.

- **Q3 — `contexts` representation.** `render_all`/`verify` take ordered
  per-pass contexts. **Recommendation:** `list[tuple[int, dict]]` (preserves
  outer-to-inner order and matches the Rust `&[(u8, HashMap<...>)]` slice)
  rather than `dict[int, dict]` (loses guaranteed ordering pre-3.7 semantics
  and hides duplicate-pass errors). Confirm the tuple-list shape.

- **Q4 — per-pass config on `ComposePolicy`.** Multi-pass `compose()`
  auto-detects stacked headers (D.2), but `ComposePolicy.passes:
  Vec<PassConfig>` carries caller-supplied per-pass config. Should the Python
  `ComposePolicy` constructor gain a `passes` keyword, or is per-pass config
  considered CLI-only (built from `--pass N` args) and therefore out of the
  Python surface? **Recommendation:** add `passes` to `PyComposePolicy` for
  parity, but this is worth an explicit ruling since it widens the policy
  constructor.

- **Q5 — `verify` builtin-override semantics in Python.** D.4 scopes builtin
  overrides as non-persistent. The Python `overrides` dict is per-call only;
  confirm no expectation of a Python-side persistent override store.

## This Sprint Does Not Close

- Any change to the `sc-composer` library or `sc-compose` CLI behavior — D.5 is
  binding-only and assumes D.1–D.4 have landed.
- Exposing the D.3 CLI flags (`--all`, `--pass N`, `--variable-delimiters`) to
  Python — only the underlying library functions are wrapped.
- `template-init` Python exposure **if Q1 resolves to CLI-only** (recorded here
  explicitly rather than silently deferred).
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
  - `ComposePolicy(passes=[PassConfig(1)]).passes` round-trips (if Q4 → yes).
- `AC3` for `D3`
  - `discover_tokens_with_brace_count("{{{ a }}}", 3)` returns `["a"]`.
  - `discover_tokens_with_brace_count("{{{ outer }}} {{ inner }}", 3)` returns
    `["outer"]` only.
  - `discover_all_pass_tokens(parsed)` returns a `dict[int, list[VariableName]]`
    keyed by pass number.
- `AC4` for `D4`
  - `render_all(parsed, [(2, {"team": "wyvern"}), (1, {"task": "test"})])`
    returns the fully-resolved 2-pass output.
  - `render_all` with a context count ≠ pass count raises `ScValidationError`
    (or `ScComposeError`).
  - A stacked-header template composes end-to-end through the existing
    `compose(request)` binding (smoke test).
- `AC5` for `D5`
  - `verify(template, deployed, contexts)` returns `VerifyResult(clean=True,
    diff=None, exit_code=0)` when rendered output matches the deployed file.
  - Drift returns `clean=False`, `diff is not None`, `exit_code == 1`.
  - `verify(..., overrides={"RENDER_DATE": "2026-01-01"})` produces
    deterministic output.
  - A missing template/deployed file raises the mapped `ScConfigError` /
    `ScComposeError` with a stable message.
- `AC6` for `D6` (only if Q1 → library-hosted)
  - `template_init(path, [(2, {"team": "wyvern"}), (1, {"task": "test"})])`
    returns a `FrontmatterInitResult` describing the 2-pass conversion.
  - `dry_run=True` reports `would_change` without writing.
  - Single-pass output omits `pass: 1` (matches D.4 normalization).
- `AC7` for `D7`
  - Every new name is importable from `sc_compose` and present in `__all__`.
  - `_native.pyi` type-checks (no `mypy`/`pyright` regressions in the smoke
    suite's configured checker).
- `AC8` backward-compat guard
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
