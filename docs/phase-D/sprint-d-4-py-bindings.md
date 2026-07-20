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

- Expose D.4's **library-owned** drift-verification surface to Python through
  the existing PyO3 `bindings/python` crate.
- Wrap `sc_composer::verify(request, deployed_path)` and the structured
  `VerifyResult` returned by the Rust library.
- Preserve D.4's actual ownership boundary: `verify` is library-owned per
  [ADR-0007](../adrs/0007-verify-library-cli-boundary.md), while multi-pass
  `template-init` is CLI-owned in `sc-compose` and therefore outside the
  scope of a `bindings/python` library adapter.
- Reuse existing Python request construction and builtin override behavior
  instead of inventing a second verification-specific input model.

All work is confined to `bindings/python`. This sprint depends on D.4's Rust
surface having passed QA and merged to `integrate/phase-d`. Per repo boundary
rules, `bindings/python` may depend on **`sc-composer` only** — never on
`sc-compose`, its CLI command modules, or ATM-specific crates.

## Hard Dependencies

- [Sprint D.4 — template-init + verify](sprint-d-4-template-init-verify.md)
  — actual Rust library and CLI ownership split
- [ADR-0007: Verify Library-and-CLI Boundary](../adrs/0007-verify-library-cli-boundary.md)
  — `verify` belongs in `sc-composer`; CLI UX remains in `sc-compose`
- [ADR-0009: Phase D Python-Binding Parity Sequencing](../adrs/0009-phase-d-python-binding-parity-sequencing.md)
  — tandem dispatch rules and canonical `contexts` contract
- [Sprint D.2-py — Python Bindings — Multi-Pass Composition Pipeline](sprint-d-2-py-bindings.md)
  — existing Python request / policy / per-pass context conventions reused by
  `verify`; also the source of the `types/` module split reused here
- [Phase D README](./README.md) — [Python Binding Parity](./README.md#python-binding-parity)
- [docs/architecture.md](../architecture.md) — adapter responsibilities and
  crate boundaries
- [CLAUDE.md](../../CLAUDE.md) — boundary rules 3–5 (`bindings/python` may
  depend on `sc-composer` only)

## Scope

This sprint adds only the library-owned verification surface. It deliberately
does not wrap multi-pass `template-init`, because that logic lives in
`crates/sc-compose/src/commands/template_init.rs` rather than `sc-composer`.

## Exact Targets

- `bindings/python/src/types/mod.rs` — register `PyVerifyResult`
- `bindings/python/src/types/results.rs` — add `PyVerifyResult` wrapper
- `bindings/python/src/functions.rs` — add `verify(request, deployed_path)`
  wrapper and registration
- `bindings/python/python/sc_compose/__init__.py` — re-export `verify` and
  `VerifyResult`
- `bindings/python/python/sc_compose/_native.pyi` — type stubs for
  `VerifyResult` and `verify`
- `bindings/python/tests/test_smoke.py` — verification smoke tests for clean,
  drift, and builtin-override cases
- `docs/project-plan.md`
- `docs/phase-D/sprint-d-4-py-bindings.md` — this document

## Deliverables

- `D1` — `verify()` Python wrapper (wraps D.4)
  - `verify(request: ComposeRequest, deployed_path: str | PathLike[str]) -> VerifyResult`
    wrapper in `bindings/python/src/functions.rs`
  - Delegates directly to `sc_composer::verify`
  - Reuses the existing Python `ComposeRequest` shape, including any per-pass
    `ComposePolicy.passes` supplied by D.2-py
  - Uses the existing `ScComposeError` / `ScConfigError` hierarchy for failure
    mapping; no verification-specific Python exception class is introduced

- `D2` — `VerifyResult` Python wrapper (wraps D.4)
  - `VerifyResult` wrapper class exposing `clean`, `resolved_template_path`,
    `deployed_path`, `rendered_text`, `deployed_text`, `diff`, `warnings`
  - `__repr__` makes drift state and paths easy to inspect in REPL/debug use,
    following the general Phase C/D wrapper style

- `D3` — Builtin override parity through existing request semantics
  - Python callers override `RENDER_DATE` / `RENDER_TIMESTAMP` the same way
    they already override other inputs: by populating `ComposeRequest.vars_input`
    with builtin keys before calling `verify()`
  - No separate `overrides=` parameter is introduced
  - Smoke coverage proves deterministic verification via builtin override input

- `D4` — Import surface, stubs, and tests
  - `__init__.py` re-exports `verify` and `VerifyResult`
  - `_native.pyi` defines both final signatures
  - `test_smoke.py` covers clean verification, drift detection with unified
    diff, builtin override determinism, and missing deployed file error mapping

## Required Work

- Add `PyVerifyResult` wrapper to `types/results.rs`, mirroring the style of
  `PyComposeResult` and `PyValidationReport`
- Add getters for every `VerifyResult` field and a concise `__repr__`
- Register `PyVerifyResult` in `types::register` (`types/mod.rs`)
- Add `verify()` wrapper to `functions.rs`
- Register the new function in `functions::register`
- Update `__init__.py` and `_native.pyi` in lockstep
- Add smoke coverage for clean/drift/error paths and builtin override reuse
- Keep the sprint explicitly docs-consistent with the D.4 Rust ownership
  outcome: no Python wrapper around CLI-owned `template-init`

## Explicit Code Samples

### `verify()` wrapper (functions.rs)

```rust
#[pyfunction]
#[allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 extracted arguments use owned PyRef values."
)]
fn verify(
    request: PyRef<'_, PyComposeRequest>,
    deployed_path: &Bound<'_, PyAny>,
) -> PyResult<PyVerifyResult> {
    let deployed_path = coerce_path_like(deployed_path)?;
    sc_composer::verify(&request.inner, deployed_path)
        .map(|inner| PyVerifyResult { inner })
        .map_err(compose_error_to_pyerr)
}
```

### `PyVerifyResult` wrapper (types/results.rs)

```rust
#[pyclass(name = "VerifyResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyVerifyResult {
    pub(crate) inner: VerifyResult,
}

#[pymethods]
impl PyVerifyResult {
    #[getter]
    fn clean(&self) -> bool {
        self.inner.clean
    }

    #[getter]
    fn diff(&self) -> Option<String> {
        self.inner.diff.clone()
    }

    #[getter]
    fn warnings(&self) -> Vec<PyDiagnostic> {
        self.inner
            .warnings
            .iter()
            .cloned()
            .map(|inner| PyDiagnostic { inner })
            .collect()
    }
}
```

### Builtin override reuse through `ComposeRequest`

```python
from sc_compose import ComposeMode, ComposePolicy, ComposeRequest, verify

request = ComposeRequest(
    root=repo_root,
    mode=ComposeMode.file("template.md.j2"),
    vars_input={"RENDER_DATE": "2026-01-01"},
    policy=ComposePolicy(),
)

result = verify(request, deployed_path)
assert result.clean is True
```

### `_native.pyi` additions

```python
class VerifyResult:
    @property
    def clean(self) -> bool: ...
    @property
    def resolved_template_path(self) -> str: ...
    @property
    def deployed_path(self) -> str: ...
    @property
    def rendered_text(self) -> str: ...
    @property
    def deployed_text(self) -> str: ...
    @property
    def diff(self) -> str | None: ...
    @property
    def warnings(self) -> list[Diagnostic]: ...


def verify(
    request: ComposeRequest,
    deployed_path: str | PathLike[str],
) -> VerifyResult: ...
```

## This Sprint Does Not Close

- Multi-pass `template-init` Python exposure
  - D.4's actual multi-pass `template-init` implementation is CLI-owned in
    `sc-compose`, not library-owned in `sc-composer`
  - Per repo boundary rules, `bindings/python` may not depend on `sc-compose`
  - If Python-side template-init access is ever desired, it would require a
    separate CLI façade or a future library extraction, neither of which is in
    scope for this sprint
- Any Python wrapper over `sc-compose verify` CLI flags or exit-code mapping;
  this sprint wraps the library `verify()` API only
- Any persistent builtin override configuration surface; overrides remain
  per-request through `ComposeRequest.vars_input`
- Any non-library adapter over `template-init` shelling out to the CLI
- Any change to the D.4 Rust implementation itself

## Backward Compatibility

- Verification reuses the existing `ComposeRequest`/`ComposePolicy` shape,
  including any `ComposePolicy.passes` from D.2-py.
- No verify-specific override model is introduced.
- No Python wrapper around template-init is added or implied.

## Acceptance Criteria

- `AC1` for `D1`
  - `verify(request, deployed_path)` returns a `VerifyResult` wrapper whose
    `clean` field is `True` when the rendered template matches the deployed file
  - The wrapper maps underlying Rust failures through the existing Python error
    hierarchy without introducing new exception types

- `AC2` for `D2`
  - Drifted deployed output yields `VerifyResult.clean is False`
  - `VerifyResult.diff` contains the unified diff text
  - `VerifyResult.resolved_template_path` and `deployed_path` expose the actual
    paths used by the Rust library result

- `AC3` for `D3`
  - A Python caller can make verification deterministic by overriding
    `RENDER_DATE` or `RENDER_TIMESTAMP` through `ComposeRequest.vars_input`
  - No verify-specific `overrides=` adapter parameter is required or added

- `AC4` for `D4`
  - `verify` and `VerifyResult` are importable from `sc_compose` and present in
    `__all__`
  - `_native.pyi` includes the final `verify` / `VerifyResult` signatures
  - Smoke tests cover clean, drift, builtin-override, and missing-file paths

- `AC5` boundary guard
  - The sprint doc explicitly records that multi-pass `template-init` remains
    out of scope because it is CLI-owned
  - No Python wrapper around `template-init` is added or implied by this plan

## Validation

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo test -p sc-compose-py`
- `maturin develop`
- `pytest bindings/python/tests`
- `git diff --check`
