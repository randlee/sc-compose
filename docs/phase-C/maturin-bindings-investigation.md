# Maturin Bindings Investigation

## Status

Planning and research spike only. This document recommends whether
`sc-compose` should grow a Python distribution surface and, if so, what shape
that surface should take.

No production code, Cargo dependencies, or release wiring are changed by this
document.

## Executive Summary

Creating a `pip` install path for `sc-compose` is feasible, but there are two
different products hiding behind that request:

1. a Python-installable CLI package
2. real Python bindings over the composition engine

Those are not the same project.

If the goal is only "`pip install` and then run `sc-compose`", maturin can
package a Rust binary and the work is relatively small.

If the goal is "import this from Python and call render/validate/resolve APIs",
the right target is not the `sc-compose` CLI crate. The right target is a new
Python adapter over `sc-composer`, with `sc-composer` remaining the semantic
engine and the adapter owning all PyO3-specific code.

Recommended direction:

- keep `sc-composer` as the pure Rust engine
- do not expose the `sc-compose` CLI crate as the primary Python API
- add a separate Python-facing adapter package in a later phase
- use maturin for build, wheel, and publish orchestration

## What Maturin Would Do Here

`maturin` is the Rust/Python packaging tool that would build wheels, create
source distributions, and publish a Python package to PyPI. It works well for:

- PyO3 extension modules
- mixed Rust/Python package layouts
- platform wheel builds on macOS, Linux, and Windows
- local editable development with `maturin develop`

For this repo, maturin would not replace Cargo. Cargo would still build the
Rust crates. Maturin would sit above that build and package the Python-facing
artifact.

## Repo Constraints That Matter

Current architecture and boundary rules materially constrain the design:

- `sc-composer` must remain a pure library
- `sc-compose` is a thin CLI over `sc-composer`
- ATM-specific integration stays out of this repo
- the normative docs currently describe a two-crate architecture

That means a Python effort should preserve these facts:

- composition semantics stay in `sc-composer`
- Python packaging should not pull CLI-only concerns into the library
- any Python bridge should be additive, not a rewrite of the existing crates

## Packaging Options

### Option A: PyPI package for the CLI binary only

Shape:

- package `sc-compose` as a Python-installable command
- no `import sc_compose` API promise
- maturin packages the Rust binary as a Python package artifact

Pros:

- smallest implementation
- keeps the existing CLI contract intact
- useful for Python-heavy environments that want one installer path

Cons:

- this is not Python bindings
- users still automate through subprocess calls
- no direct access to `sc-composer` request/result types

Assessment:

- feasible
- low risk
- only worth doing if the real need is installer convenience rather than a
  Python library

### Option B: Add PyO3 directly to `sc-composer`

Shape:

- add PyO3 macros and Python module exports directly inside `sc-composer`
- build one crate as both Rust library and Python extension module

Pros:

- one crate owns both semantics and binding surface
- minimal extra crate count

Cons:

- weak fit with the "pure library" rule
- pollutes the core crate with Python packaging concerns
- increases test/build complexity inside the semantic engine
- makes the public Rust API and the Python ABI evolve together

Assessment:

- technically feasible
- architecturally poor fit
- not recommended

### Option C: Add a separate Python adapter over `sc-composer`

Shape:

- keep `sc-composer` unchanged as the Rust engine
- add a new Python-facing adapter package that depends on `sc-composer`
- adapter owns PyO3 module definitions, Python type conversion, and wheel
  packaging
- package via maturin using a mixed Rust/Python project layout

Pros:

- preserves the current engine boundary
- keeps Python-specific concerns out of the core library
- allows a Pythonic API instead of a CLI-shaped API
- keeps the CLI and Python package releasable on different cadences if needed

Cons:

- introduces a third deliverable even if it lives in the same repo
- requires explicit doc updates because the current architecture documents a
  two-crate baseline
- adds CI, release, and compatibility surface area

Assessment:

- best fit for true bindings
- recommended option

## Recommended Structural Shape

If Phase C is approved, the cleanest design is:

- `crates/sc-composer`
  - remains the Rust semantic engine
- `crates/sc-compose`
  - remains the CLI
- `bindings/python/` or similar
  - contains `pyproject.toml`
  - contains the Python package directory
  - contains a small Rust crate exposing PyO3 functions/classes
  - depends on `sc-composer`

That adapter package should not depend on `sc-compose` except possibly for a
separate CLI-installer experiment. The CLI crate brings in command-line UX,
exit-code behavior, logger wiring, and process assumptions that are not good
primary Python API material.

## Recommended Python API Shape

The Python surface should model the library, not shell commands.

Suggested top-level import:

```python
import sc_composer
```

Suggested public API:

```python
from sc_composer import (
    ComposePolicy,
    ComposeRequest,
    ComposeResult,
    Diagnostic,
    FrontmatterInitResult,
    ProfileKind,
    ResolveResult,
    RuntimeKind,
    UnknownVariablePolicy,
    ValidationReport,
    compose_file,
    parse_template_document,
    render_template,
    render_loaded_template,
    resolve_profile,
    validate,
    frontmatter_init,
    init_workspace,
)
```

Suggested behavior:

- `RuntimeKind`
  - mirrors core runtime layouts, including Hermes, without binding the adapter
    to any runtime implementation
- `render_template(template: str, context: dict) -> str`
  - thin wrapper over the existing one-shot renderer
- `render_loaded_template(template_name: str, template_text: str, context: dict, supporting_templates: list[dict] | None = None) -> RenderedArtifact`
  - maps cleanly to the current preloaded render path
- `compose_file(...) -> ComposeResult`
  - file-mode composition over the existing `ComposeRequest`
- `validate(...) -> ValidationReport`
  - validation without render
- `resolve_profile(...) -> ResolveResult`
  - profile lookup without full composition
- `frontmatter_init(path: str, force: bool = False, dry_run: bool = False) -> FrontmatterInitResult`
- `init_workspace(root: str, dry_run: bool = False) -> InitResult`

This investigation section is illustrative background, not the authoritative
Phase C sprint scope. The finalized sprint docs use the real public validation
entry points `validate` and `validate_with_observer` rather than the earlier
illustrative `validate_file` wrapper name used in this memo.

Recommended Python ergonomics:

- accept plain Python `dict` / `list` / scalar inputs and map them into the
  existing `serde_json::Value`-based `InputValue` model
- map Rust errors into Python exceptions with the stable diagnostic code and
  message attached
- expose enums as Python enums or string-valued constants
- keep observer/logging hooks out of the first Python binding milestone unless
  there is a concrete consumer for them

## Why The API Should Not Mirror The CLI Exactly

The CLI has user-interface responsibilities that are not library semantics:

- argument parsing
- stdout/stderr formatting
- exit codes
- file-writing defaults
- logger construction and shutdown

Binding that surface directly would produce a Python API that feels like a
subprocess wrapper in disguise. The better Python API is the request/result
model already present in `sc-composer`.

## Boundary Rule Analysis

The key question from the task is whether a PyO3 surface can be added without
violating the `sc-composer` pure-library rule.

Answer:

- yes, if the PyO3 surface lives in a separate adapter package that depends on
  `sc-composer`
- no, not cleanly, if the PyO3 surface is embedded directly into
  `sc-composer`

Why:

- `sc-composer` today is a runtime-agnostic Rust library
- PyO3 and maturin are language-bridge and packaging concerns, not template
  semantics
- keeping the bridge outside the engine preserves the current architecture and
  keeps future non-Python embeddings possible

This does require one explicit documentation decision if implemented later:
the current "two crates" architecture would need to be amended to describe the
new adapter deliverable.

## Build, CI, and Publish Implications

A real Python binding line would add a second release channel beyond Homebrew,
winget, GitHub Releases, and `cargo install`.

Expected additions:

- `pyproject.toml` for the Python package
- maturin configuration
- a wheel build matrix in CI
- PyPI credentials and publish workflow
- Python smoke/integration tests against built wheels

Likely CI shape:

- keep existing Rust CI unchanged for the current crates
- add Python packaging CI on:
  - macOS
  - Linux
  - Windows
- test at least one import-and-render smoke path per platform
- run Python-side tests against the built wheel, not only against source

Versioning choices that need an explicit decision:

1. lock the Python package version to the workspace version
2. let the Python package evolve on its own version line

Recommendation:

- start with lockstep versioning
- only split version lines if the Python adapter starts shipping on a meaningfully
  different cadence

## ABI and Wheel Strategy

There are two broad choices:

1. build per-Python-version wheels
2. opt into PyO3 `abi3` or `abi3t` support to reduce wheel count

Recommendation:

- do not commit to `abi3` in the planning phase
- validate first whether the desired binding surface stays within the stable ABI
- treat `abi3` as an optimization, not as a starting constraint

Even with `abi3`, platform-specific wheels are still required.

## Testing Implications

A real Phase C implementation should add Python-specific coverage for:

- basic import success
- inline template rendering
- file-mode composition under confinement rules
- validation diagnostics
- profile resolution
- frontmatter-init and init-workspace helpers
- error mapping from Rust diagnostics to Python exceptions

The Rust test suite would remain the source of truth for composition semantics.
Python tests should prove the bridge is faithful, not duplicate every Rust
behavioral test.

## Open Questions

These questions should be answered before scoping implementation:

1. Is the actual goal a Python library, a `pip` install path for the CLI, or
   both?
2. Should the Python package live in this repo or in a separate adapter repo?
3. What package/import names do we want?
   - package name likely allows hyphens
   - import name should be underscore-based
4. Do we want to expose only stable core composition APIs in v1, or also the
   reporting APIs added during Phase B?
5. Do we want PyPI publication as part of the main release train, or as a
   separate publish job?
6. Is there a real consumer who needs observer/logging callbacks from Python?

## Non-Goals For A First Python Binding Sprint

- no ATM-specific behavior
- no wrapper around every CLI command
- no browser-open or publish-network features
- no Python reimplementation of the composition engine
- no promise that every internal reporting helper becomes part of the Python
  public API

## Recommendation

Proceed only if the team wants true Python library consumption or a concrete
PyPI distribution channel.

If the need is installer convenience only, scope a small CLI-packaging sprint
and stop there.

If the need is Python bindings, scope a dedicated follow-on phase with:

- a separate Python adapter package over `sc-composer`
- maturin-based wheel builds
- a small, library-shaped Python API
- explicit doc updates to record the new deliverable and release channel

That path is feasible and technically straightforward, but it is not zero-cost:
the main work is not the binding code itself. The main work is owning another
supported distribution surface, another CI matrix, and another compatibility
contract.

## Primary References

- Maturin user guide: <https://www.maturin.rs/>
- Maturin project layout guide: <https://www.maturin.rs/project_layout.html>
- PyO3 getting started: <https://pyo3.rs/main/getting-started>
- PyO3 build and distribution guide: <https://pyo3.rs/v0.29.0/building-and-distribution>
- PyO3 type stub generation: <https://pyo3.rs/main/type-stub>
- PyO3 FAQ on workspace/testing and `cdylib`/`rlib`: <https://pyo3.rs/v0.29.0/faq>
