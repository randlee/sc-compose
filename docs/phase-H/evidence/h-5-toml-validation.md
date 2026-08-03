# H.5 TOML extraction validation record

- Validation date: 2026-08-03
- H.4 merge baseline: `cc12676`
- Implementation commits: `042281e`, `cd2684d`
- Worktree: `sprint/h-5-toml-extraction`
- Scope: TOML extraction through the shared `ExtractFormat` dispatch, Python
  binding, text CLI, and JSON CLI.

## Focused implementation checks

- `cargo test -p sc-composer --test extract_integration toml_` — 2 passed.
- `cargo test -p sc-compose --test cli extract_text_supports_toml` — 1
  passed.
- `cargo test -p sc-compose --test json_cli extract_json_toml` — 1 passed.
- Rebuilt the native Python wheel from this worktree and ran the smoke suite —
  33 passed.

The TOML corpus covers a realistic Cargo-style configuration with frontmatter,
nested tables, inline tables, arrays-of-tables, and stable table-key and array
paths. Adversarial coverage includes malformed input, duplicate keys, typed
placeholder values, dynamic keys, missing paths, and array shape mismatch.
Placeholder matching delegates to `extract/raw_text.rs`.

## Required validation

All required validation passed on the implementation tip:

- `cargo fmt --all --check` — passed.
- `cargo test --workspace` — passed (including 68 CLI unit tests, 131 CLI
  integration tests, 60 JSON-CLI tests, 1 boundary test, 17 Python-binding
  Rust tests, 140 composer unit tests, 28 extraction integration tests, and
  14 composer integration tests).
- `cargo clippy --all-targets --all-features -- -D warnings` — passed.
- `cargo test -p sc-compose --test repo_boundaries` — 1 passed.
- `cargo test -p sc-compose-py` — 17 passed.
- Rebuilt the native wheel with `maturin build`, then ran
  `python3 -m pytest bindings/python/tests/test_smoke.py` against that wheel —
  33 passed.
- `git diff --check` — passed.
