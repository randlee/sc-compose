---
id: G.3
title: Python Extraction Bindings
status: complete
branch: sprint/g-3-python-extraction-bindings
worktree: ../sc-compose-worktrees/sprint/g-3-python-extraction-bindings
target: develop
---

# Sprint G.3 — Python Extraction Bindings

## Goal

Expose the G.2 known-template XML extraction API to the first customer through
the existing `bindings/python` PyO3 adapter. Python receives the same report,
occurrence provenance, diagnostics, and fail-closed boundaries as Rust; it
does not reimplement template analysis or extraction.

## Hard dependencies

- G.1's extraction contract and G.2's stable `sc-composer` API.
- Existing Phase C/D Python adapter conventions, exception hierarchy, stubs,
  wheel packaging, and Python-only dependency direction.
- `bindings/python` may depend on `sc-composer` only; it must not depend on
  `sc-compose`, CLI modules, ATM, or a research harness.

## Exact targets

- `bindings/python/src/functions.rs`
- `bindings/python/src/types/mod.rs`
- `bindings/python/src/types/results.rs`
- `bindings/python/src/errors.rs` only for shared error mapping
- `bindings/python/python/sc_compose/__init__.py`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`
- `bindings/python/pyproject.toml` only if the public package metadata needs
  an extraction capability note
- `docs/architecture.md`

## Deliverables

- `G3-D1` — Add a Python `extract_variables` callable that accepts template
  and rendered text plus include/exclude selections and delegates directly to
  `sc_composer::extract`.
- `G3-D2` — Add Python wrapper types for the extraction report, occurrences,
  sources, warnings, and diagnostics with stable properties and useful
  representations for the first customer.
- `G3-D3` — Reuse the existing exception hierarchy for malformed XML,
  unsupported syntax, ambiguity, and configuration failures; missing
  occurrences surface as a non-fatal WARN_EXTRACT_NOT_OBSERVED diagnostic
  within a successful ExtractionReport, not as an exception. Do not create a
  Python-only semantic error model.
- `G3-D4` — Update the package import surface and `_native.pyi` stubs in lockstep
  with the Rust adapter registration.
- `G3-D5` — Add Python smoke coverage proving value/provenance parity with the
  Rust contract, repeated-sibling correctness, include/exclude behavior,
  string-value semantics, and fail-closed unsupported cases.

## Python contract

```python
def extract_variables(
    template: str,
    rendered: str,
    *,
    include: list[str] | None = None,
    exclude: list[str] | None = None,
) -> ExtractionReport: ...

class ExtractionReport:
    values: dict[str, str]
    occurrences: list[ExtractionOccurrence]
    confidence: float
    diagnostics: list[Diagnostic]
```

The Python API is in-memory like the Rust API. A caller that needs files reads
them at the Python orchestration boundary; this keeps path policy and semantic
extraction ownership in one place.

## This sprint does not close

- CLI arguments, JSON envelopes, or process exit codes;
- a second extraction implementation in Python;
- Python support for JSON/Markdown output, loop reconstruction, unknown
  template identification, or typed-value inference;
- changes to the extraction algorithm or the Minijinja renderer.

## Acceptance criteria

- Python callers can invoke the extraction function from the documented
  package import and receive the complete report contract.
- Rust and Python report values, occurrence paths, diagnostics, and boundary
  outcomes agree for the shared fixtures.
- `_native.pyi`, package exports, Rust registration, and smoke tests are
  consistent; wheel packaging remains valid.
- The adapter introduces no dependency on `sc-compose`, ATM, or a research
  harness,
  and no existing Python API changes behavior.

## Required validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
