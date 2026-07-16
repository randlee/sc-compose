---
id: C1
title: Maturin Python Bindings Foundation
status: planned
branch: plan/maturin-bindings-implementation-plan
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/maturin-bindings-implementation-plan
---

# Sprint C1 — Maturin Python Bindings Foundation

## Goal

Define the first implementation slice for a real Python package that exposes
`sc-compose` composition APIs as an importable library while preserving the
current Rust architecture boundaries.

This sprint is the implementation plan for the first deliverable in Phase C.
It is intentionally narrower than the full v1 Python API surface. The goal is
to get one reviewable adapter package scaffolded, built in CI, and proven on
all three release platforms before the remaining wrapper surface is added.

## Fixed Product Decisions

These decisions are closed for this sprint and must not be revisited during
implementation:

- the deliverable is a true Python library, not a CLI-wrapper package
- the adapter lives in this repo under `bindings/python/`
- package name is `sc-compose`
- Python import name is `sc_compose`
- v1 scope is composition APIs only
- reporting and observability APIs are out of scope
- versioning is lockstep with the workspace version
- PyPI publication joins the main release train
- `sc-composer` remains a pure Rust library
- PyO3 lives only in the new Python adapter package

## Hard Dependencies

- [docs/phase-C/maturin-bindings-investigation.md](./maturin-bindings-investigation.md)
- [docs/architecture.md](../architecture.md)
- [docs/project-plan.md](../project-plan.md)
- [CLAUDE.md](../../CLAUDE.md)

## Exact Targets

This sprint plans work against these future implementation targets:

- `Cargo.toml`
- `bindings/python/Cargo.toml`
- `bindings/python/pyproject.toml`
- `bindings/python/python/sc_compose/__init__.py`
- `bindings/python/python/sc_compose/py.typed`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/src/lib.rs`
- `bindings/python/tests/test_smoke.py`
- `.github/workflows/ci.yml`
- `docs/architecture.md`
- `docs/phase-C/sprint-C1-maturin-bindings.md`

## Confirmed Scope

Phase C uses a separate Python adapter over `sc-composer`.

The adapter package owns:

- PyO3 wrappers
- Python packaging metadata
- Pythonic request/result shims
- wheel and sdist production
- Python smoke tests

The adapter package does not own:

- CLI dispatch
- report runtime APIs
- observability adapters
- ATM integration
- any semantic reimplementation of rendering or validation

## Deliverables

C1 commits exactly these deliverables:

- `D1`
  - add `bindings/python/` as a new in-repo adapter package with the mixed
    Rust/Python layout defined below
- `D2`
  - add the new workspace member to root `Cargo.toml` without introducing PyO3
    or Python packaging logic into `crates/sc-composer`
- `D3`
  - implement `bindings/python/Cargo.toml`, `bindings/python/pyproject.toml`,
    `bindings/python/src/lib.rs`, and the typed Python package skeleton under
    `bindings/python/python/sc_compose/`
- `D4`
  - wrap exactly three callable functions for the first implementation slice:
    `render_template`, `render_loaded_template`, and `compose_file`
- `D5`
  - add the minimum Python-facing wrapper types and enums required to support
    those three functions
- `D6`
  - add one CI job in `.github/workflows/ci.yml` that builds wheels and runs a
    smoke test on macOS, Linux, and Windows using one pinned Python version
- `D7`
  - add one smoke test module at `bindings/python/tests/test_smoke.py` that
    verifies import plus the three wrapped functions from installed wheels
- `D8`
  - amend `docs/architecture.md` so the repo documents `bindings/python` as a
    third, Python-facing adapter package that depends on `sc-composer` only

Every other Python-binding concern is out of scope for C1 unless it is
explicitly named in this deliverables list.

## Structural Shape

The implementation target layout for this sprint is:

```text
bindings/python/
├── Cargo.toml
├── pyproject.toml
├── python/
│   └── sc_compose/
│       ├── __init__.py
│       ├── _native.pyi
│       └── py.typed
├── src/
│   └── lib.rs
└── tests/
    └── test_smoke.py
```

The Rust workspace remains the semantic source of truth:

- `crates/sc-composer`
  - owns composition behavior
- `crates/sc-compose`
  - owns the CLI
- `bindings/python`
  - owns Python FFI and packaging only

## Rust Crate Shape

`bindings/python/Cargo.toml` should define a new workspace member with:

- package name: `sc-compose-py`
- library name: `sc_compose_native`
- crate type: `["cdylib", "rlib"]`
- edition and version inherited from the workspace
- path dependency on `sc-composer = { path = "../../crates/sc-composer", version = "1.1.0" }`
- `pyo3 = "0.29"` as the Python binding layer
- `serde_json = "1"` for value conversion
- `pythonize` is explicitly out of scope for C1 and must not be introduced in
  this sprint

The sprint should start without `abi3`.

Why:

- the investigation already accepted maturin + PyO3 as the adapter path
- PyO3 documents `abi3` as an optional constrained mode rather than a default
- a first sprint should minimize platform surprises and ship version-specific
  wheels first

The implementation should also avoid the deprecated `extension-module` cargo
feature. Current PyO3 guidance is to let maturin handle extension-module build
configuration and keep the crate testable in a Cargo workspace.

## Python Packaging Shape

`bindings/python/pyproject.toml` should define:

- `[build-system]`
  - `requires = ["maturin>=1.9.4,<2.0"]`
  - `build-backend = "maturin"`
- `[project]`
  - `name = "sc-compose"`
  - `version` sourced from the workspace during release sync
  - `requires-python = ">=3.11"`
  - README, license, repository, and classifiers
- `[tool.maturin]`
  - `python-source = "python"`
  - `module-name = "sc_compose._native"`
  - package/include settings needed for `py.typed` and `_native.pyi`

This split is required because the pip-install name contains a hyphen while the
Python import name must remain `sc_compose`.

The Rust module name exposed from `bindings/python/src/lib.rs` must therefore
be `_native`, not `sc_compose`.

## Python Import Surface

The package import contract for v1 remains:

```python
import sc_compose
```

`python/sc_compose/__init__.py` should re-export the stable public surface from
`._native` and keep the native module private:

```python
from ._native import (
    ComposeRequest,
    ComposeResult,
    ComposePolicy,
    LoadedTemplateRequest,
    RenderedArtifact,
    RenderTemplateError,
    ComposeError,
    ValidationReport,
    ResolveResult,
    FrontmatterInitResult,
    InitResult,
    ProfileKind,
    RuntimeKind,
    UnknownVariablePolicy,
    compose_file,
    frontmatter_init,
    init_workspace,
    render_loaded_template,
    render_template,
    resolve_profile,
    validate_file,
)
```

The `.pyi` file and `py.typed` marker are part of this sprint’s contract so the
first release is typed from day one.

## Full V1 API Boundary

The Phase C v1 target surface is:

- `render_template`
- `render_loaded_template`
- `compose_file`
- `validate_file`
- `resolve_profile`
- `frontmatter_init`
- `init_workspace`

Associated Python-facing types and enums are in scope only insofar as they are
required to support those functions:

- `ComposePolicy`
- `ComposeRequest`
- `ComposeResult`
- `LoadedTemplateRequest`
- `RenderedArtifact`
- `ValidationReport`
- `ResolveResult`
- `FrontmatterInitResult`
- `InitResult`
- `RuntimeKind`
- `ProfileKind`
- `UnknownVariablePolicy`

Phase B reporting types and observability types are explicitly excluded.

## C1 Implementation Slice

C1 does not deliver the entire v1 surface in one pass.

## Deferred To C4

C1 explicitly defers these release-train items to
[Sprint C4 — Python Release Train And Packaging Hardening](./sprint-C4-python-release-train.md):

- all `release.yml` Python release-train work
- PyPI publish credentials and publish automation
- `release/publish-artifacts.toml` amendments
- `docs/publishing.md` and `docs/publishing-agent.md` amendments
- GitHub Release wheel and sdist attachment behavior

C1 separately defers these non-release implementation items:

- `validate_file`
- `resolve_profile`
- `frontmatter_init`
- `init_workspace`
- richer exception hierarchy
- Python-specific convenience helpers beyond the minimum needed to make the
  wrapped APIs usable

## Concrete Wrapper Rules

The wrapper layer should follow these mapping rules:

- plain Python `dict`, `list`, `str`, `int`, `float`, `bool`, and `None` map to
  `serde_json::Value`
- file paths enter Rust as strings or `pathlib.Path`-compatible path strings
- Rust `Result<T, ComposeError>` converts to Python exceptions with:
  - stable message text
  - stable diagnostic code when one exists
- Python enums should round-trip through the same variants already exposed by
  `sc-composer`
- wrappers should prefer explicit builder or constructor methods over exposing
  raw Rust struct fields when invariants need validation

The wrapper should not expose observer hooks, logger sinks, or any `sc-compose`
CLI-only concepts.

## Rust FFI Boundary Sample

The binding crate should follow this boundary pattern:

```rust
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::create_exception;
use sc_composer::{ComposeError, ComposeRequest, ComposeResult};

create_exception!(sc_compose, ScComposeError, PyException);

#[pyclass]
#[derive(Clone)]
pub struct PyComposeResult {
    #[pyo3(get)]
    pub rendered_text: String,
}

#[pyfunction]
fn compose_file(request: PyComposeRequest) -> PyResult<PyComposeResult> {
    let request: ComposeRequest = request.try_into()?;
    let result: ComposeResult = sc_composer::compose(&request).map_err(compose_error_to_pyerr)?;
    Ok(PyComposeResult {
        rendered_text: result.rendered_text,
    })
}

#[pymodule]
#[pyo3(name = "_native")]
fn native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("ScComposeError", py.get_type::<ScComposeError>())?;
    module.add_class::<PyComposeResult>()?;
    module.add_function(wrap_pyfunction!(compose_file, module)?)?;
    Ok(())
}

fn compose_error_to_pyerr(error: ComposeError) -> PyErr {
    let message = error.to_string();
    ScComposeError::new_err(message)
}
```

This sample is normative for C1 in three ways:

- `#[pymodule]` owns the private `_native` module only
- wrapped functions return `PyResult<_>`
- Rust composition errors are converted in one dedicated helper rather than
  ad hoc at each call site

## Concrete File Breakdown

### `Cargo.toml`

- add `bindings/python` to `[workspace].members`
- keep the existing two crate packages intact
- do not add PyO3 to shared workspace dependencies until there is a clear need
  to share that dependency across multiple crates

### `bindings/python/Cargo.toml`

- define the adapter package
- add:
  - `pyo3 = "0.29"`
  - `sc-composer` path dependency
  - `serde_json = "1"`
- set `[lib]`
  - `name = "sc_compose_native"`
  - `crate-type = ["cdylib", "rlib"]`

### `bindings/python/src/lib.rs`

- define `#[pymodule]`
  - `#[pyo3(name = "_native")]`
- define `#[pyfunction]`
  - `render_template`
  - `render_loaded_template`
  - `compose_file`
- define `#[pyclass]` wrappers needed for:
  - `LoadedTemplateRequest`
  - `RenderedArtifact`
  - `ComposePolicy`
  - `ComposeRequest`
  - `ComposeResult`
- define Python-visible enums or string wrappers for:
  - `RuntimeKind`
  - `ProfileKind`
  - `UnknownVariablePolicy`
- define a small exception surface:
  - `ScComposeError`
  - one helper for attaching diagnostic code when available

### `bindings/python/pyproject.toml`

- define the Python package metadata
- point maturin at `python/sc_compose`
- include the typed-package markers
- keep version lockstep documented and wired to release automation

### `bindings/python/python/sc_compose/__init__.py`

- re-export the stable API
- avoid Python-side logic beyond import organization

### `bindings/python/python/sc_compose/_native.pyi`

- declare the first-sprint public signatures
- provide enough type detail for the wrapped request/result classes

### `bindings/python/tests/test_smoke.py`

- import `sc_compose`
- assert the module exposes the expected names
- render one inline template
- render one loaded template with a supporting template
- compose one file-mode request from a temp directory

## CI Additions

Implementation should add one dedicated Python wheel build job to
`.github/workflows/ci.yml`.

Recommended job shape:

- job name: `python-wheels`
- matrix axes:
  - `os`: `ubuntu-latest`, `macos-latest`, `windows-latest`
  - `python-version`: `"3.11"`
- key steps:
  - checkout
  - setup Python
  - install Rust toolchain `1.94.1`
  - install `maturin`
  - run `maturin build --manifest-path bindings/python/Cargo.toml`
  - install the produced wheel
  - run `pytest bindings/python/tests/test_smoke.py`

Why one Python version in C1:

- C1 is proving platform portability and packaging shape first
- multi-interpreter support can expand after the adapter surface exists
- the release train already spans multiple OS targets, which is the larger
  initial risk

## Architecture Amendment Required

`docs/architecture.md` currently documents a two-crate baseline.

Phase C implementation must amend it to describe a third crate/package:

- `bindings/python`
  - Python-facing adapter crate/package
  - depends on `sc-composer` only
  - owns PyO3 and maturin configuration
  - must not depend on `sc-compose`
  - must not depend on `sc-observability`

The amended dependency direction becomes:

- `sc-compose` -> `sc-composer`
- `sc-compose` -> `sc-observability`
- `bindings/python` -> `sc-composer`

Forbidden dependency additions remain:

- `sc-composer` -> `bindings/python`
- `bindings/python` -> `sc-compose`
- `bindings/python` -> ATM-specific crates

## Out Of Scope

C1 must not modify:

- `.github/workflows/release.yml`
- `release/publish-artifacts.toml`
- `docs/publishing.md`
- `docs/publishing-agent.md`
- PyPI credential wiring or secrets handling
- GitHub Release attachment logic for wheels or sdists

Those release-train items are deferred intact to
[Sprint C4 — Python Release Train And Packaging Hardening](./sprint-C4-python-release-train.md); see the canonical
[Deferred To C4](#deferred-to-c4) list above.

## Explicit Non-Goals

This sprint does not include:

- report rendering or report catalog bindings
- CLI command wrappers
- `sc-observability` integration
- observer callback design
- browser-open helpers
- ATM adapters
- PyPI trusted-publishing redesign
- `abi3` wheel optimization
- free-threaded Python support
- a full multi-Python-version compatibility matrix

## Acceptance Criteria

The first implementation sprint closes only when all of the following are true:

- `AC1` for `D1`
  - `bindings/python/` exists with the planned mixed Rust/Python layout
- `AC2` for `D2`
  - root `Cargo.toml` includes the new workspace member
  - no PyO3 dependency or Python packaging logic enters `crates/sc-composer`
- `AC3` for `D3`
  - `bindings/python/Cargo.toml`, `bindings/python/pyproject.toml`, and the
    typed Python package skeleton exist
  - `pyproject.toml` sets `requires-python = ">=3.11"`
- `AC4` for `D4`
  - `render_template`, `render_loaded_template`, and `compose_file` are
    callable from Python
- `AC5` for `D5`
  - the minimum wrapper types and enums required by the three wrapped
    functions are exposed from `sc_compose`
  - no `sc-compose` CLI types or observability paths leak into the Python
    public API
- `AC6` for `D6`
  - one CI job named `python-wheels` builds and smoke-tests wheels on macOS,
    Linux, and Windows using `python-version: "3.11"`
- `AC7` for `D7`
  - `import sc_compose` succeeds from an installed wheel on all three OS jobs
  - the smoke suite exercises inline render, loaded-template render, and
    file-mode composition
  - typed package markers ship with the built wheel
- `AC8` for `D8`
  - `docs/architecture.md` documents `bindings/python` as a third,
    Python-facing adapter package that depends on `sc-composer` only
- `AC9` scope guard
  - C1 makes no changes to `.github/workflows/release.yml`,
    `release/publish-artifacts.toml`, `docs/publishing.md`, or
    `docs/publishing-agent.md`

## Required Validation

When C1 is implemented, the owning agent must run:

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- Python smoke tests from installed wheels on:
  - macOS
  - Linux
  - Windows

## Follow-On Sprints

This sprint leaves these concrete next slices for later Phase C work:

- C2
  - add `validate_file`
  - add `resolve_profile`
  - extend smoke coverage to profile-mode resolution
- C3
  - add `frontmatter_init`
  - add `init_workspace`
  - add richer exception typing and user-facing docs
- C4
  - deliver the release-train work listed in the canonical
    [Deferred To C4](#deferred-to-c4) section
  - expand interpreter matrix
  - evaluate `abi3` or `abi3t` only after the base adapter is stable
