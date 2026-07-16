---
id: C.2
title: Python API Surface
status: planned
branch: sprint/c-2-python-api-surface
worktree: ../sc-compose-worktrees/sprint/c-2-python-api-surface
---

# Sprint C.2 — Python API Surface

## Goal

Complete the full v1 non-reporting Python API surface on top of the adapter
scaffold and single proven callable delivered by
[Sprint C.1 — Maturin Python Bindings Foundation](./sprint-c-1-maturin-bindings.md).

This sprint resolves the `Renderer` customization seam as a single committed
Rust signature, wraps every remaining v1 callable and its supporting types,
and wraps the full public error surface as Python exceptions with a stable
`.message`/`.code` contract. It is an implementation sprint: it lands real,
production-ready Rust and Python code, not a design-only artifact.

## Hard Dependencies

- [docs/phase-C/sprint-c-1-maturin-bindings.md](./sprint-c-1-maturin-bindings.md)
  — the adapter package, CI wheel job, and `compose_file`/`ScComposeError`
  baseline this sprint builds on
- [docs/phase-C/maturin-bindings-investigation.md](./maturin-bindings-investigation.md)
- [docs/architecture.md](../architecture.md)
- [docs/project-plan.md](../project-plan.md)
- [CLAUDE.md](../../CLAUDE.md)

## Exact Targets

- `crates/sc-composer/src/renderer.rs`
- `crates/sc-composer/src/validation.rs`
- `bindings/python/src/lib.rs`
- `bindings/python/python/sc_compose/__init__.py`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`
- `docs/architecture.md`
- `Cargo.toml`
- `docs/phase-C/sprint-c-2-python-api-surface.md`

## Renderer Seam Decision

C.1 left the `Renderer` customization seam unresolved. This sprint commits to
exactly one signature and removes the other option that was previously
presented as interchangeable.

`Renderer::with_options()` (`crates/sc-composer/src/renderer.rs`) takes
`impl FnOnce(&mut minijinja::Environment<'static>)`. `minijinja::Environment`
is a third-party type. Making `with_options` public would leak `minijinja`
into `sc-composer`'s public API, and a closure over a third-party engine type
cannot cross a PyO3/Python boundary at all. `minijinja::Environment` must
never appear in `sc-composer`'s public API.

The only committed seam is:

```rust
impl Renderer {
    /// Create a renderer with non-default block/variable delimiters.
    ///
    /// # Panics
    ///
    /// Panics if `open` or `close` are not valid delimiter tokens accepted
    /// by the underlying template engine. This narrow internal seam is
    /// infallible-by-signature and does not surface a `RenderError`; callers
    /// must pass validated delimiter tokens.
    #[must_use]
    pub fn with_delimiters(open: &str, close: &str) -> Self {
        Self::with_options(|env| {
            env.set_syntax(
                minijinja::syntax::SyntaxConfig::builder()
                    .variable_delimiters(open, close)
                    .build()
                    .expect("valid delimiter configuration"),
            );
        })
    }
}
```

`with_options` itself stays `pub(crate)`. `with_delimiters` is the only public
customization entry point. Unlike the rest of this sprint's public
constructors, `with_delimiters` does not raise `ScValidationError` or
`ScConfigError` on malformed delimiter tokens — invalid tokens currently
panic. This is an accepted narrow exception to the
"constructors validate and raise ... rather than accepting malformed values
silently" contract stated in
[Wrapper Class Code Samples](#wrapper-class-code-samples), scoped to this one
internal engine-configuration seam.

## Deliverables

C.2 commits exactly these deliverables:

- `D1`
  - wrap the remaining full non-reporting callable surface from Python:
    `compose`, `validate`, `resolve_template_path`, `resolve_profile`,
    `render_template`, `render_loaded_template`, `parse_template_document`,
    `expand_includes`, `frontmatter_init`, `init_workspace`,
    `validate_input_value`, `input_value_from_yaml`, `to_forward_slash`, and
    `BUILTIN_VARIABLE_NAMES`
- `D2`
  - add the Python-facing wrapper types, enums, and constants required to
    support the D1 surface: `LoadedTemplateRequest`, `NamedTemplateAsset`,
    `RenderedArtifact`, `ExpandedTemplate`, `ParsedTemplate`, `Frontmatter`,
    `FrontmatterInitResult`, `InitResult`, `ValidationReport`, `Diagnostic`,
    `VariableName`, `ProfileName`, `ConfiningRoot`, `ResolverPolicy`,
    `ResolveResult`, `RuntimeKind`, `ProfileKind`, `UnknownVariablePolicy`,
    `VariableSource`, `DiagnosticSeverity`, `DiagnosticCode`
- `D3`
  - rely on the already-enabled `features = ["custom_syntax"]` setting on the
    workspace-level `minijinja` dependency declaration in the root
    `Cargo.toml` — the workspace wires third-party crate features directly at
    `[workspace.dependencies]` rather than through per-crate `[features]`
    tables, and `minijinja::syntax::SyntaxConfig` is gated behind that
    minijinja `custom_syntax` feature
  - add `Renderer::with_delimiters(open: &str, close: &str) -> Self` to
    `sc-composer`'s public API per [Renderer Seam Decision](#renderer-seam-decision)
    and wrap the reusable `Renderer` class for Python, including constructor,
    render helpers, and delimiter customization
- `D4`
  - wrap the full public error surface as Python exceptions —
    `ScComposeError` (already present from C.1), `ScRenderError`,
    `ScValidationError`, `ScResolveError`, `ScIncludeError`, `ScConfigError`
    — each with stable `.message` and `.code` access where a diagnostic code
    exists
- `D5`
  - make token discovery callable from Python by changing `discover_tokens`
    from `pub(crate)` to public in `crates/sc-composer/src/validation.rs` and
    exposing a Python wrapper for variable discovery workflows
- `D6`
  - update `docs/architecture.md` so the normative architecture baseline does
    not drift from the newly shipped `sc-composer` public surface: add
    `Renderer::with_delimiters(open: &str, close: &str) -> Self` to the
    Public API Shape section (§8) and its `Renderer` entry in the API
    Ownership Matrix (§8.1), and document `discover_tokens` as a public
    `validation` module function in the Module Architecture section (§4)

Every other Python-binding concern is out of scope for C.2 unless it is
explicitly named in this deliverables list. This list is the single
authoritative source for C.2 scope; the appendices below
([Non-Normative Appendix](#non-normative-appendix)) restate detail for
convenience but do not add scope beyond D1–D5.

## Wrapper Class Code Samples

### `Renderer`

```python
class Renderer:
    def __init__(self) -> None: ...
    @classmethod
    def with_delimiters(cls, open: str, close: str) -> "Renderer": ...
    def render(self, template: str, context: dict) -> str: ...
    def render_named(self, name: str, template: str, context: dict) -> str: ...
```

### `Frontmatter` / `ParsedTemplate`

```python
class Frontmatter:
    variables: dict
    profile: "ProfileName | None"

class ParsedTemplate:
    frontmatter: Frontmatter
    body: str
```

### `ConfiningRoot` / `VariableName` / `ProfileName`

```python
class ConfiningRoot:
    def __init__(self, root: str) -> None: ...
    def confine(self, candidate: str) -> str: ...

class VariableName:
    def __init__(self, value: str) -> None: ...
    def __str__(self) -> str: ...

class ProfileName:
    def __init__(self, value: str) -> None: ...
    def __str__(self) -> str: ...
```

All four constructors validate their input and raise `ScValidationError` or
`ScConfigError` (per the exception hierarchy below) rather than accepting
malformed values silently.

### `ResolverPolicy`

[docs/architecture.md §20](../architecture.md#20-extensibility) marks
`ResolverPolicy` as an intentionally open, extensible Rust type on the
Rust-native surface (`custom resolver policies` is a named extension point).
That open-extensibility promise does **not** extend across the Python
boundary in v1. `ResolverPolicy` is opaque and read-only from Python for
C.2: it is returned embedded in `ComposePolicy` (via `compose`/`validate`
request round-trips) but Python code cannot construct a custom
`ResolverPolicy` or subclass/extend its resolution behavior.

```python
class ResolverPolicy:
    """Opaque, read-only in v1. No public constructor and no field access
    beyond `__repr__`. Python callers select policy behavior only through
    the enums and options already exposed on `ComposePolicy`."""
```

Python-side construction of custom resolver policies (mirroring the Rust
extensibility point) is explicitly deferred to a later sprint and is not a
C.2 deliverable.

### Remaining D2 Wrapper Types (Signature Table)

The following D2 types are read-only data/result pyclasses (no custom
methods beyond field access and `__repr__`), mirrored field-for-field from
their Rust definitions. `X | None` denotes an optional field.

| Python type | Field | Python type |
| --- | --- | --- |
| `LoadedTemplateRequest` | `template_name` | `str` |
| | `template_text` | `str` |
| | `context` | `dict` |
| `NamedTemplateAsset` | `template_name` | `str` |
| | `template_text` | `str` |
| `RenderedArtifact` | `rendered` | `str` |
| | `template_name` | `str` |
| `ExpandedTemplate` | `text` | `str` |
| | `resolved_files` | `list[str]` |
| | `frontmatters` | `list[tuple[str, Frontmatter \| None]]` |
| | `include_chains` | `dict[str, list[str]]` |
| `FrontmatterInitResult` | `target_path` | `str` |
| | `frontmatter_text` | `str` |
| | `discovered_variables` | `list[VariableName]` |
| | `changed` | `bool` |
| | `would_change` | `bool` |
| `InitResult` | `prompts_dir` | `str` |
| | `gitignore_updated` | `bool` |
| | `scanned_templates` | `list[str]` |
| | `recommendations` | `list[Diagnostic]` |
| | `validation_passed` | `bool` |
| `ValidationReport` | `ok` | `bool` |
| | `warnings` | `list[Diagnostic]` |
| | `errors` | `list[Diagnostic]` |
| | `resolve_result` | `ResolveResult` |
| `Diagnostic` | `severity` | `DiagnosticSeverity` |
| | `code` | `DiagnosticCode` |
| | `message` | `str` |
| | `path` | `str \| None` |
| | `line` | `int \| None` |
| | `column` | `int \| None` |
| | `include_chain` | `list[str]` |
| `ResolveResult` | `resolved_path` | `str` |
| | `attempted_paths` | `list[str]` |
| | `ambiguity_candidates` | `list[str]` |

Plain enums (member names are `UPPER_SNAKE_CASE` in Python, values are the
lowercase/snake_case Rust `serde` wire form):

| Python type | Members |
| --- | --- |
| `RuntimeKind` | `CLAUDE`, `CODEX`, `GEMINI`, `OPENCODE` |
| `ProfileKind` | `AGENT`, `COMMAND`, `SKILL` |
| `UnknownVariablePolicy` | `ERROR`, `WARN`, `IGNORE` (default `IGNORE`) |
| `VariableSource` | `EXPLICIT_INPUT`, `ENVIRONMENT`, `BUILTIN`, `TEMPLATE_INPUT_DEFAULT`, `FRONTMATTER_DEFAULT`, `INCLUDED_DEFAULT` |
| `DiagnosticSeverity` | `ERROR`, `WARNING`, `INFO` |

These types carry no separate acceptance criterion beyond `AC2` ("every D2
type, enum, and constant is exposed from `sc_compose` and used by at least
one D1 callable's signature"), which already covers them.

## Exception Hierarchy Contract

Every exception in the public surface exposes:

```python
class ScComposeError(Exception):
    message: str
    code: str | None

class ScRenderError(ScComposeError): ...
class ScValidationError(ScComposeError): ...
class ScResolveError(ScComposeError): ...
class ScIncludeError(ScComposeError): ...
class ScConfigError(ScComposeError): ...
```

- `.message` is always a stable, non-empty string.
- `.code` is `None` when the underlying Rust error has no diagnostic code,
  and a stable string identifier otherwise.
- each subclass maps to exactly one Rust error family; no Rust error variant
  maps to more than one Python exception class.

### `DiagnosticCode` / Exception `.code` Relationship

`DiagnosticCode` (D2) and each exception's `.code` (D4) refer to the same
underlying value space: the stable code registry in
[docs/architecture.md §18.1](../architecture.md#181-failure-mode-matrix)
(`ERR_RESOLVE_NOT_FOUND`, `ERR_VAL_MISSING_REQUIRED`, `ERR_CONFIG_PARSE`,
etc.). `DiagnosticCode` is exposed as a Python `StrEnum` (or plain `str`
alias, implementer's choice, but not a distinct opaque wrapper class) whose
member values are exactly equal to the `.code` strings raised by the
corresponding exception. No conversion step exists or is needed between a
`Diagnostic.code` value and an `ScComposeError` subclass's `.code` value for
the same underlying failure — they are the same string. `AC2` and `AC4`
jointly assert this: `AC2` requires `DiagnosticCode` values to round-trip
through `Diagnostic.code`, and `AC4` requires exception `.code` values to be
drawn from the same `DiagnosticCode` value space.

## Python Import Surface

`python/sc_compose/__init__.py` extends the C.1 surface to the full v1
contract:

```python
from ._native import (
    ComposeRequest,
    ComposeResult,
    ComposePolicy,
    ComposeMode,
    LoadedTemplateRequest,
    NamedTemplateAsset,
    RenderedArtifact,
    ExpandedTemplate,
    ParsedTemplate,
    Frontmatter,
    FrontmatterInitResult,
    InitResult,
    ValidationReport,
    Diagnostic,
    VariableName,
    ProfileName,
    ConfiningRoot,
    ResolverPolicy,
    ResolveResult,
    Renderer,
    RuntimeKind,
    ProfileKind,
    UnknownVariablePolicy,
    VariableSource,
    DiagnosticSeverity,
    DiagnosticCode,
    BUILTIN_VARIABLE_NAMES,
    ScComposeError,
    ScRenderError,
    ScValidationError,
    ScResolveError,
    ScIncludeError,
    ScConfigError,
    compose,
    compose_file,
    validate,
    resolve_template_path,
    resolve_profile,
    render_template,
    render_loaded_template,
    parse_template_document,
    expand_includes,
    frontmatter_init,
    init_workspace,
    validate_input_value,
    input_value_from_yaml,
    to_forward_slash,
    discover_tokens,
)
```

## Non-Normative Appendix

The following sections restate the D1–D5 surface by file for implementer
convenience. They cross-reference deliverable IDs and are not an independent
source of scope — if anything below appears to add a callable, type, or
exception not named in [Deliverables](#deliverables), the Deliverables list
wins.

### `bindings/python/src/lib.rs` (D1, D2, D3, D4, D5)

- `#[pyfunction]` additions for the full D1 callable list
- `#[pyclass]` additions for the full D2 type list plus `Renderer` (D3)
- exception additions for the full D4 list
- a public `discover_tokens` wrapper (D5)

### `bindings/python/python/sc_compose/__init__.py` (D1–D5)

- re-export the full v1 surface per
  [Python Import Surface](#python-import-surface)

### `bindings/python/python/sc_compose/_native.pyi` (D1–D5)

- declare typed signatures for every name in
  [Python Import Surface](#python-import-surface)

### `bindings/python/tests/test_smoke.py` (D1–D5)

- exercise `compose`, `validate`, `resolve_template_path`, and
  `resolve_profile`
- render one inline template and one loaded template with a supporting
  template
- parse one template document and inspect frontmatter/body
- expand one include graph
- run `validate_input_value`, `input_value_from_yaml`, and `to_forward_slash`
- exercise `Renderer.with_delimiters` to prove the customization seam is
  actually usable from Python
- exercise `discover_tokens`
- assert each of `ScRenderError`, `ScValidationError`, `ScResolveError`,
  `ScIncludeError`, and `ScConfigError` is raised by at least one call path,
  and that `.message`/`.code` are accessible

## Out Of Scope

C.2 must not modify:

- `.github/workflows/release.yml`
- `release/publish-artifacts.toml`
- `docs/publishing.md`
- `docs/publishing-agent.md`
- PyPI credential wiring or secrets handling
- GitHub Release attachment logic for wheels or sdists
- `bindings/python/Cargo.toml`, `bindings/python/pyproject.toml`, or the CI
  `python-wheels` job shape beyond what is needed to keep the existing C.1 job
  green against the expanded surface

Release-train items are deferred intact to
[Sprint C.3 — Python Release Train And Packaging Hardening](./sprint-c-3-python-release-train.md).

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

- `AC1` for `D1`
  - every D1 callable is callable from Python and returns the typed result
    from [Python Import Surface](#python-import-surface)
- `AC2` for `D2`
  - every D2 type, enum, and constant is exposed from `sc_compose` and used
    by at least one D1 callable's signature
  - `FrontmatterInitResult` and `InitResult` are pyclasses returned by
    `frontmatter_init` and `init_workspace` respectively
  - `DiagnosticCode` member values are exactly the stable code strings from
    [docs/architecture.md §18.1](../architecture.md#181-failure-mode-matrix)
    and round-trip through `Diagnostic.code` per
    [DiagnosticCode / Exception `.code` Relationship](#diagnosticcode--exception-code-relationship)
  - `ResolverPolicy` is opaque and read-only from Python per
    [`ResolverPolicy`](#resolverpolicy): no public constructor, no
    Python-side extension of resolution behavior
- `AC3` for `D3`
  - `sc-composer` exposes `Renderer::with_delimiters(open: &str, close: &str) -> Self`
    as its only public renderer-customization seam
  - `minijinja::Environment` does not appear in any public signature in
    `crates/sc-composer`
  - Python can construct `Renderer.with_delimiters(...)` and render with
    non-default delimiters
- `AC4` for `D4`
  - `ScRenderError`, `ScValidationError`, `ScResolveError`, `ScIncludeError`,
    and `ScConfigError` are all raised by at least one Python call path
  - every exception exposes `.message` (always a non-empty string) and
    `.code` (`None` or a stable string) per
    [Exception Hierarchy Contract](#exception-hierarchy-contract)
  - every non-`None` exception `.code` value is drawn from the same
    `DiagnosticCode` value space per
    [DiagnosticCode / Exception `.code` Relationship](#diagnosticcode--exception-code-relationship)
- `AC5` for `D5`
  - Python can call token discovery without invoking the full validation
    pipeline
- `AC6` for `D6`
  - `docs/architecture.md` §8 (Public API Shape) lists
    `Renderer::with_delimiters(open: &str, close: &str) -> Self`
  - `docs/architecture.md` §8.1 (API Ownership Matrix) reflects the
    `Renderer` row's expanded seam
  - `docs/architecture.md` §4 (Module Architecture) documents
    `discover_tokens` as a public `validation` module function
- `AC7` scope guard
  - C.2 makes no changes to `.github/workflows/release.yml`,
    `release/publish-artifacts.toml`, `docs/publishing.md`, or
    `docs/publishing-agent.md`
  - no `sc-compose` CLI types or observability paths leak into the Python
    public API

## Required Validation

When C.2 is implemented, the owning agent must run:

- `cargo build -p sc-composer` to prove the workspace-enabled
  `minijinja` `custom_syntax` feature compiles and
  `Renderer::with_delimiters` builds against
  `minijinja::syntax::SyntaxConfig`
- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- Python smoke tests from installed wheels on:
  - macOS
  - Linux
  - Windows
