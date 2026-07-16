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
- `.github/workflows/release.yml`
- `release/publish-artifacts.toml`
- `docs/architecture.md`
- `docs/publishing.md`
- `docs/publishing-agent.md`
- `docs/phase-C/README.md`
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
- optionally `pythonize` only if direct serde-to-Python conversion materially
  simplifies wrapper code after implementation review

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
  - Python compatibility floor for the package
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

C1 delivers:

- adapter package scaffold under `bindings/python/`
- workspace membership wired in root `Cargo.toml`
- `pyproject.toml` configured for maturin mixed-project layout
- typed Python package skeleton under `bindings/python/python/sc_compose/`
- first three wrapped functions:
  - `render_template`
  - `render_loaded_template`
  - `compose_file`
- first required wrapper types:
  - `LoadedTemplateRequest`
  - `RenderedArtifact`
  - `ComposePolicy`
  - `ComposeRequest`
  - `ComposeResult`
  - `RuntimeKind`
  - `ProfileKind`
  - `UnknownVariablePolicy`
- one smoke test that imports `sc_compose` and exercises:
  - inline template rendering
  - preloaded template rendering
  - file-mode composition from a temporary repo-like tree
- wheel builds on macOS, Linux, and Windows in CI

C1 explicitly defers:

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
  - `python-version`: one stable floor version for C1 smoke, not a full
    interpreter matrix yet
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

## Release Workflow Additions

Implementation should extend the main release train in
`.github/workflows/release.yml` with Python packaging steps.

Required additions:

- new build job: `build-python-wheels`
  - runs after `gate-and-tag`
  - builds wheels for:
    - `ubuntu-latest`
    - `macos-latest`
    - `windows-latest`
  - uploads wheel artifacts
- new build step for source distribution:
  - either inside `build-python-wheels` or a dedicated `build-python-sdist` job
  - produces one sdist artifact from `bindings/python/`
- new publish job: `publish-pypi`
  - runs after wheel and sdist artifacts exist
  - downloads those artifacts
  - publishes with maturin
- release job update:
  - attach wheels and sdist to the GitHub Release alongside existing archives

Recommended publish command:

```bash
maturin upload --non-interactive dist/*
```

with:

- `MATURIN_PYPI_TOKEN=${{ secrets.PYPI_API_TOKEN }}`
- `MATURIN_NON_INTERACTIVE=1`

This keeps PyPI publication aligned with the existing release workflow pattern,
which already publishes from workflow-managed credentials.

## Required New Secret

Document one new required secret only:

- `PYPI_API_TOKEN`
  - used to populate `MATURIN_PYPI_TOKEN` during `publish-pypi`

No secret values are created or checked into the repo as part of this plan.

## Release Metadata Changes

Implementation should also update:

- `release/publish-artifacts.toml`
  - add a Python artifact section or equivalent source-of-truth entry for:
    - wheel artifacts
    - source distribution
- `docs/publishing.md`
  - add PyPI verification steps
- `docs/publishing-agent.md`
  - add the new required secret and PyPI verification rules

The release docs should treat PyPI as a first-class release channel alongside:

- crates.io
- GitHub Releases
- Homebrew
- `winget`

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

- `bindings/python/` exists with the planned mixed Rust/Python layout
- root `Cargo.toml` includes the new workspace member
- `pyproject.toml` builds through maturin
- `import sc_compose` succeeds from an installed wheel on macOS, Linux, and
  Windows
- `render_template`, `render_loaded_template`, and `compose_file` are callable
  from Python
- typed package markers ship with the wheel
- one CI job builds and smoke-tests wheels on all three operating systems
- release workflow can build wheel artifacts and an sdist without changing the
  existing Rust crate publish order
- no PyO3 dependency or Python packaging logic enters `crates/sc-composer`
- no `sc-compose` CLI types or observability paths leak into the Python public
  API

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
  - expand interpreter matrix
  - evaluate `abi3` or `abi3t` only after the base adapter is stable
