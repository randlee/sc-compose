---
id: C.1
title: Maturin Python Bindings Foundation
status: planned
branch: sprint/c-1-maturin-bindings
worktree: ../sc-compose-worktrees/sprint/c-1-maturin-bindings
---

# Sprint C.1 — Maturin Python Bindings Foundation

## Goal

Define the first implementation slice for a real Python package that exposes
`sc-compose` composition APIs as an importable library while preserving the
current Rust architecture boundaries.

This sprint is the implementation plan for the first deliverable in Phase C.
It is scoped to adapter/crate scaffolding, CI wheel packaging, and one proven
end-to-end callable — it does not attempt the full v1 Python API surface. The
goal is to get one reviewable adapter package scaffolded, built in CI, and
proven on all three release platforms with a single working callable before
the remaining wrapper surface is added in
[Sprint C.2 — Python API Surface](./sprint-c-2-python-api-surface.md).

## Fixed Product Decisions

These decisions are closed for this sprint and must not be revisited during
implementation:

- the deliverable is a true Python library, not a CLI-wrapper package
- the adapter lives in this repo under `bindings/python/`
- package name is `sc-compose`
- Python import name is `sc_compose`
- the import name intentionally diverges from the investigation memo's
  illustrative `sc_composer` sample so the Python module matches the pip
  package identity while avoiding collision with the Rust crate name
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
- `docs/project-plan.md`
- `CLAUDE.md`
- `scripts/release_artifacts.py`
- `docs/phase-C/sprint-c-1-maturin-bindings.md`

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

C.1 commits exactly these deliverables:

- `D1`
  - add `bindings/python/` as a new in-repo adapter package with the mixed
    Rust/Python layout defined below
- `D2`
  - add the new workspace member to root `Cargo.toml` without introducing PyO3
    or Python packaging logic into `crates/sc-composer`
- `D3`
  - implement `bindings/python/Cargo.toml`, `bindings/python/pyproject.toml`,
    `bindings/python/src/lib.rs`, and the typed Python package skeleton under
    `bindings/python/python/sc_compose/`, including the version-sync wiring
    through `scripts/release_artifacts.py sync-python-version` and
    `verify-python-version`
- `D4`
  - wrap exactly one proven end-to-end callable, `compose_file`, plus the
    minimal request/result/exception types required to call it:
    `ComposeRequest`, `ComposeResult`, `ComposePolicy`, `ComposeMode`, and the
    `ScComposeError` exception
- `D5`
  - add one CI job in `.github/workflows/ci.yml` that builds wheels and runs a
    smoke test on macOS, Linux, and Windows using one pinned Python version
- `D6`
  - add one smoke test module at `bindings/python/tests/test_smoke.py` that
    verifies import plus `compose_file` from installed wheels
- `D7`
  - amend `docs/architecture.md` so the repo documents `bindings/python` as a
    third, Python-facing adapter package that depends on `sc-composer` only
  - update `CLAUDE.md` and `docs/project-plan.md` so the repo-wide boundary
    rules name `bindings/python` and its allowed and forbidden dependency edges

Every other Python-binding concern is out of scope for C.1 unless it is
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
- `pythonize` is explicitly out of scope for C.1 and must not be introduced in
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
  - `version` is rewritten by
    `python3 scripts/release_artifacts.py sync-python-version --workspace-toml Cargo.toml --pyproject bindings/python/pyproject.toml`
    immediately before `maturin build` or `maturin sdist`
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

Release validation must also run:

```bash
python3 scripts/release_artifacts.py verify-python-version \
  --workspace-toml Cargo.toml \
  --pyproject bindings/python/pyproject.toml \
  --version <X.Y.Z>
```

The Python release path is not considered closed unless both the sync step and
the verify step are present in the future release workflow.

## Python Import Surface

The package import contract for C.1 is intentionally minimal:

```python
import sc_compose
```

`python/sc_compose/__init__.py` should re-export the C.1 surface from
`._native` and keep the native module private:

```python
from ._native import (
    ComposeRequest,
    ComposeResult,
    ComposePolicy,
    ComposeMode,
    ScComposeError,
    compose_file,
)
```

The `.pyi` file and `py.typed` marker are part of this sprint's contract so the
first release is typed from day one.

The remaining v1 callables, wrapper types, the `Renderer` class, the full
exception hierarchy, and token discovery are deferred to
[Sprint C.2 — Python API Surface](./sprint-c-2-python-api-surface.md); see
[Deferred To C.2](#deferred-to-c2) below.

## Concrete Wrapper Rules

The wrapper layer should follow these mapping rules:

- plain Python `dict`, `list`, `str`, `int`, `float`, `bool`, and `None` map to
  `serde_json::Value`
- file paths enter Rust as strings or `pathlib.Path`-compatible path strings
- Rust `Result<T, ComposeError>` converts to Python exceptions with:
  - stable message text
  - stable diagnostic code when one exists
- wrappers should prefer explicit builder or constructor methods over exposing
  raw Rust struct fields when invariants need validation

The wrapper should not expose observer hooks, logger sinks, reporting runtime
APIs, or any `sc-compose` CLI-only concepts in C.1.

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

This sample is normative for C.1 in three ways:

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
  - `compose_file`
- define `#[pyclass]` wrappers needed for:
  - `ComposePolicy`
  - `ComposeRequest`
  - `ComposeMode`
  - `ComposeResult`
- define a small exception surface:
  - `ScComposeError`

### `bindings/python/pyproject.toml`

- define the Python package metadata
- point maturin at `python/sc_compose`
- include the typed-package markers
- keep version lockstep documented and wired to release automation through
  `scripts/release_artifacts.py sync-python-version` before wheel and sdist
  builds plus `verify-python-version` as the release gate

### `bindings/python/python/sc_compose/__init__.py`

- re-export the stable C.1 API
- avoid Python-side logic beyond import organization

### `bindings/python/python/sc_compose/_native.pyi`

- declare the C.1 public signatures
- provide enough type detail for `compose_file` and its request/result types

### `bindings/python/tests/test_smoke.py`

- import `sc_compose`
- assert the module exposes the expected names
- compose one file-mode request from a temp directory using `compose_file`
- assert `ScComposeError` is raised for one invalid-input case

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

Why one Python version in C.1:

- C.1 is proving platform portability and packaging shape first
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
- `bindings/python` -> `sc-observability`
- `bindings/python` -> ATM-specific crates

## Out Of Scope

C.1 must not modify:

- `.github/workflows/release.yml`
- `release/publish-artifacts.toml`
- `docs/publishing.md`
- `docs/publishing-agent.md`
- PyPI credential wiring or secrets handling
- GitHub Release attachment logic for wheels or sdists

Those release-train items are deferred intact to
[Sprint C.3 — Python Release Train And Packaging Hardening](./sprint-c-3-python-release-train.md).

## Deferred To C.2

C.1 explicitly defers the remaining v1 API surface to
[Sprint C.2 — Python API Surface](./sprint-c-2-python-api-surface.md):

- the remaining non-reporting callable surface: `compose`, `validate`,
  `resolve_template_path`, `resolve_profile`, `render_template`,
  `render_loaded_template`, `parse_template_document`, `expand_includes`,
  `validate_file`, `frontmatter_init`, `init_workspace`,
  `validate_input_value`, `input_value_from_yaml`, `to_forward_slash`, and
  `BUILTIN_VARIABLE_NAMES`
- the full wrapper type, enum, and constant surface required to support that
  callable set
- the `Renderer::with_delimiters` public seam and the `Renderer` Python class
- the full exception hierarchy (`ScRenderError`, `ScValidationError`,
  `ScResolveError`, `ScIncludeError`, `ScConfigError`) beyond `ScComposeError`
- token discovery (`discover_tokens` visibility change and its Python wrapper)

## Deferred To C.3

C.1 defers these release-train items to
[Sprint C.3 — Python Release Train And Packaging Hardening](./sprint-c-3-python-release-train.md):

- all `release.yml` Python release-train work
- PyPI publish credentials and publish automation
- `release/publish-artifacts.toml` amendments
- `docs/publishing.md` and `docs/publishing-agent.md` amendments
- GitHub Release wheel and sdist attachment behavior

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
- any callable, type, or class beyond `compose_file` and its minimal support
  types (see [Deferred To C.2](#deferred-to-c2))

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
  - `compose_file` is callable from Python and returns a typed
    `ComposeResult`
  - invalid input raises `ScComposeError` with a stable message
  - no `sc-compose` CLI types or observability paths leak into the Python
    public API
- `AC5` for `D5`
  - one CI job named `python-wheels` builds and smoke-tests wheels on macOS,
    Linux, and Windows using `python-version: "3.11"`
- `AC6` for `D6`
  - `import sc_compose` succeeds from an installed wheel on all three OS jobs
  - the smoke suite exercises `compose_file` and the `ScComposeError` path
  - typed package markers ship with the built wheel
- `AC7` for `D7`
  - `docs/architecture.md` documents `bindings/python` as a third,
    Python-facing adapter package that depends on `sc-composer` only
  - `CLAUDE.md` and `docs/project-plan.md` name `bindings/python` in the same
    boundary rule set and forbid `bindings/python` dependency edges back into
    `sc-compose`, `sc-observability`, or ATM-specific crates
- `AC8` scope guard
  - C.1 makes no changes to `.github/workflows/release.yml`,
    `release/publish-artifacts.toml`, `docs/publishing.md`, or
    `docs/publishing-agent.md`
  - C.1 introduces no callable, class, or exception beyond `compose_file` and
    its minimal support types

## Required Validation

When C.1 is implemented, the owning agent must run:

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `TMP_PYPROJECT="$(python3 - <<'PY'
from pathlib import Path
import tempfile
print(Path(tempfile.mkdtemp(prefix='sc-compose-')) / 'pyproject.toml')
PY
)"`
- `cp bindings/python/pyproject.toml "$TMP_PYPROJECT"`
- `python3 scripts/release_artifacts.py sync-python-version --workspace-toml Cargo.toml --pyproject "$TMP_PYPROJECT"`
- `python3 scripts/release_artifacts.py verify-python-version --workspace-toml Cargo.toml --pyproject "$TMP_PYPROJECT" --version "$(python3 - <<'PY'
import tomllib
from pathlib import Path
print(tomllib.loads(Path('Cargo.toml').read_text(encoding='utf-8'))['workspace']['package']['version'])
PY
)"`
- Python smoke tests from installed wheels on:
  - macOS
  - Linux
  - Windows

## Follow-On Sprints

This sprint leaves these concrete next slices for later Phase C work:

- observer callback slice (not yet drafted as a sprint doc)
  - add observer-owned Python callbacks and event wrappers
  - add `compose_with_observer`, `validate_with_observer`, and
    `resolve_profile_with_observer`
  - add `DiagnosticEnvelope<T>` or another explicit Python JSON-envelope seam
- docs/examples slice (not yet drafted as a sprint doc)
  - extend user-facing docs and examples for the broader API surface
  - evaluate whether recovery-hint and observer event types merit direct
    Python modeling

Sprint C.2 (Python API surface, delivering the canonical
[Deferred To C.2](#deferred-to-c2) section above) and Sprint C.3
(release-train work, delivering the canonical
[Deferred To C.3](#deferred-to-c3) section above) are both drafted as real
sprint docs and therefore take the next open numbers per the Sprint Numbering
rule in `sprint-planning-guidelines.md`. The two placeholder slices above
remain undrafted and keep their prose-only follow-on status with no sprint
number until they are actually written up.
