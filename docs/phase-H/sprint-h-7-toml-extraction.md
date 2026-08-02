---
id: H.7
title: TOML Extraction
status: planned
branch: sprint/h-7-toml-extraction
worktree: ../sc-compose-worktrees/sprint/h-7-toml-extraction
target: develop
---

# Sprint H.7 — TOML Extraction

## Goal

- Implement the accepted known-template TOML extraction contract across Rust,
  Python, and CLI surfaces.
- Keep TOML parser semantics, paths, and rendered-string behavior explicit and
  independent from YAML and JSON assumptions.

## Hard Dependencies

- H.1 accepts TOML format and parser semantics.
- H.2/H.3 establish the format-selection and report-extension patterns.

## Exact Targets

- `crates/sc-composer/src/extract/mod.rs`
- `crates/sc-composer/src/extract/toml.rs`
- `crates/sc-composer/src/extract/tests.rs`
- `crates/sc-composer/tests/extract_integration.rs`
- `crates/sc-compose/src/commands/extract.rs`
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `bindings/python/src/functions.rs`
- `bindings/python/src/types/results.rs`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H7-D1 — Add the approved TOML adapter and format selection without changing
  XML, JSON, or YAML behavior.
- H7-D2 — Implement H.1-defined table, array-of-table, key, duplicate-key,
  scalar, null-equivalent, and malformed-input semantics.
- H7-D3 — Expose identical TOML reports, diagnostics, and filtering through
  Rust, Python, text CLI, and JSON CLI surfaces.
- H7-D4 — Add realistic Cargo/config TOML fixtures and adversarial cases for
  every intentional boundary.

## Required Work

- Define stable paths for nested tables and array-of-table occurrences.
- Keep TOML rendered values string-based and do not infer source types.
- Ensure parser errors and duplicate keys remain distinct from unsupported
  template syntax and ambiguity.

## Explicit Code Samples

```text
sc-compose extract TEMPLATE RENDERED --format toml
extract_variables(template, rendered, *, format="toml", include=None, exclude=None)
```

Both adapters must delegate to the shared `ExtractFormat::Toml` library path.

## This Sprint Does Not Close

- YAML extraction; that is H.6.
- XML mixed-content or dirty-prefix behavior.
- TOML schema inference, typed-value recovery, or unknown-template discovery.

## Acceptance Criteria

- TOML success and failure behavior is identical across Rust, Python, and CLI.
- Table and array-of-table paths are stable and ambiguity-safe.
- Every H.1 TOML policy has implementation and regression coverage.
- Existing XML/JSON/YAML extraction and TOML composition/config behavior remain
  unchanged.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
