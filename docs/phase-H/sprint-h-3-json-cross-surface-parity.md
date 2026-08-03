---
id: H.3
title: JSON Cross-Surface Parity
status: complete
branch: sprint/h-3-json-cross-surface-parity
worktree: ../sc-compose-worktrees/sprint/h-3-json-cross-surface-parity
target: develop
---

# Sprint H.3 — JSON Cross-Surface Parity

## Goal

- Expose H.2 JSON extraction through the first-customer Python binding and the
  `sc-compose extract` CLI without duplicating extraction semantics.
- Preserve XML as the default and keep machine-readable output stable.

## Hard Dependencies

- H.1 is complete and its accepted contract is available.
- H.2 JSON core is merged and its Rust tests pass.

## Exact Targets

- `bindings/python/src/functions.rs`
- `bindings/python/src/types/results.rs`
- `bindings/python/src/types/mod.rs`
- `bindings/python/src/errors.rs`
- `bindings/python/python/sc_compose/_native.pyi`
- `bindings/python/tests/test_smoke.py`
- `crates/sc-compose/src/cli.rs`
- `crates/sc-compose/src/commands/extract.rs`
- `crates/sc-compose/tests/cli/extract.rs`
- `crates/sc-compose/tests/json_cli/extract.rs`
- `docs/requirements.md` and `docs/architecture.md` only for H.1-approved
  surface wording

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- H3-D1 — Add the approved format selector to Python and CLI surfaces while
  retaining XML-default compatibility.
- H3-D2 — Serialize JSON paths, sources, warnings, diagnostics, and report
  confidence through the existing binding and CLI envelopes.
- H3-D3 — Add parity tests proving Rust, Python, human CLI, and JSON CLI expose
  identical values and boundary outcomes for the shared JSON fixtures.
- H3-D4 — Keep stdout clean, exit codes stable, and unsupported format or
  filter requests actionable.

## Required Work

- Python must call the shared `sc_composer::extract` entry point and must not
  implement a Python JSON matcher.
- CLI format parsing must be owned by the CLI and passed into the library as a
  typed request.
- Keep the existing XML command invocation valid without a new required flag.
- Add type stubs and public API documentation for the selected format shape.
- Add at least one realistic JSON ATM payload and one adversarial boundary
  case to the shared corpus.

## Explicit Code Samples

```text
extract_variables(template, rendered, *, format="xml", include=None, exclude=None)
sc-compose extract TEMPLATE RENDERED [--format xml|json]
```

The exact spelling must match the H.1-approved API, and XML remains the
backward-compatible default.

## This Sprint Does Not Close

- YAML or TOML extraction; those are H.4 and H.5.
- XML mixed-content extraction, XML dirty-prefix tolerance, or a
  customer-facing raw-text/best-effort mode; those are future-phase scope.
- A second extraction algorithm in either adapter.
- Unknown-template identification or typed JSON recovery.

## Acceptance Criteria

- The same JSON request produces equivalent Rust, Python, text-CLI, and
  JSON-CLI report semantics.
- Existing XML users and tests remain unchanged.
- Every new public option has help text, type-stub/API coverage, error mapping,
  and stdout/exit-code tests.
- Repository boundary checks prove no prototype or ATM runtime dependency was
  introduced.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
