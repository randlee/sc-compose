# H.4 YAML extraction validation record

- Validation date: 2026-08-03
- Baseline: `origin/integrate/phase-h` at `c68daf2`
- Worktree: `sprint/h-4-yaml-extraction`
- Scope: YAML extraction through the shared `ExtractFormat` dispatch, Python
  binding, text CLI, and JSON CLI.

## Focused implementation checks

The following checks passed before the implementation push:

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

Full sprint validation is run after the implementation push and will be
recorded in the completion update.
