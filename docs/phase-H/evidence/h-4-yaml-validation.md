# H.4 YAML extraction validation record

- Validation date: 2026-08-03
- Baseline: `origin/integrate/phase-h` at `c68daf2`
- Worktree: `sprint/h-4-yaml-extraction`
- Validation commit: `e9c55c6`
- Scope: YAML extraction through the shared `ExtractFormat` dispatch, Python
  binding, text CLI, and JSON CLI.

## Focused implementation checks

The following focused checks passed before the implementation push:

- `cargo test -p sc-composer --test extract_integration yaml_` — 2 passed.
- `cargo test -p sc-compose --test cli extract_text_supports_yaml` — 1 passed.
- `cargo test -p sc-compose --test json_cli extract_json_yaml` — 1 passed.
- Rebuilt the native Python wheel from this worktree and ran
  `python3 -m pytest bindings/python/tests/test_smoke.py` against it — 32
  passed.

The shared YAML corpus includes a realistic ATM-style configuration with
template frontmatter plus adversarial malformed, duplicate-key, multi-document,
alias, and typed-scalar boundaries. The report uses `YamlPathSegment` mapping
keys and sequence indexes and `YamlExtractionSource::StringScalar`; matching is
delegated to `extract/raw_text.rs`.

## Required validation

All required validation passed on `e9c55c6`:

- `cargo fmt --all --check` — passed.
- `cargo test --workspace` — passed (including 68 CLI unit tests, 130 CLI
  integration tests, 59 JSON-CLI tests, 1 boundary test, 17 Python-binding
  Rust tests, 140 composer unit tests, 26 extraction integration tests, and
  14 composer integration tests).
- `cargo clippy --all-targets --all-features -- -D warnings` — passed.
- `cargo test -p sc-compose --test repo_boundaries` — 1 passed.
- `cargo test -p sc-compose-py` — 17 passed.
- Rebuilt the native wheel from this worktree with `maturin build`, then
  `python3 -m pytest bindings/python/tests/test_smoke.py` against that wheel —
  32 passed.
- `git diff --check` — passed.
