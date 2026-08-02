---
id: H.6
title: YAML Extraction
status: planned
branch: sprint/h-6-yaml-extraction
worktree: ../sc-compose-worktrees/sprint/h-6-yaml-extraction
target: develop
---

# Sprint H.6 — YAML Extraction

## Goal

- Implement the accepted known-template YAML extraction contract across Rust,
  Python, and CLI surfaces.
- Keep rendered values string-based and make YAML-specific ambiguity and parser
  behavior explicit.

## Hard Dependencies

- H.1 accepts YAML format and parser semantics.
- H.2/H.3 establish the format-selection and report-extension patterns.

## Exact Targets

- `crates/sc-composer/src/extract/mod.rs`
- `crates/sc-composer/src/extract/error.rs`
- `crates/sc-composer/src/extract/yaml.rs`
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

- H6-D1 — Add the approved YAML adapter and format selection without changing
  XML, JSON, or existing YAML var-file behavior.
- H6-D2 — Implement the H.1-defined mapping/path, duplicate-key, alias,
  document-stream, scalar, null, and malformed-input semantics.
- H6-D3 — Expose identical YAML reports, diagnostics, and filtering through
  Rust, Python, text CLI, and JSON CLI surfaces.
- H6-D4 — Add realistic YAML frontmatter/config fixtures and adversarial
  cases for every intentional boundary.

## Required Work

- Keep YAML rendered-output parsing distinct from YAML template frontmatter and
  var-file decoding semantics.
- Do not infer source types from YAML scalar spelling unless H.1 explicitly
  changes the string-only report contract.
- Ensure aliases, anchors, duplicate keys, and multi-document inputs cannot
  silently change occurrence identity.

## Explicit Code Samples

```text
sc-compose extract TEMPLATE RENDERED --format yaml
extract_variables(template, rendered, *, format="yaml", include=None, exclude=None)
```

Both adapters must delegate to the shared `ExtractFormat::Yaml` library path.

## This Sprint Does Not Close

- TOML extraction; that is H.7.
- XML mixed-content or dirty-prefix behavior.
- YAML schema inference, typed-value recovery, or unknown-template discovery.

## Acceptance Criteria

- YAML success and failure behavior is identical across Rust, Python, and CLI.
- Every H.1 YAML policy has implementation and regression coverage.
- Existing XML/JSON extraction and YAML rendering/var-file behavior remain
  unchanged.
- Boundary tests prove the adapter does not import the prototype harness or
  introduce ATM runtime dependencies.

## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test repo_boundaries`
- `cargo test -p sc-compose-py`
- `python3 -m pytest bindings/python/tests/test_smoke.py`
- `git diff --check`
