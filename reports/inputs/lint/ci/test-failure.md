# title: top-level ci test failure fixture
# sets:
#   - lint
#   - ci
# copy_json:
#   command: ci
#   fixture: tests/fixtures/sc-lint/ci/test-failure
#   outcome: failed
#   raw_artifact: reports/inputs/lint/ci/test-failure.json

Input fixture: `tests/fixtures/sc-lint/ci/test-failure`.

The actual top-level CI workflow reached its final `cargo test --workspace`
step, preserved the structured `CI-TEST-FINDING-001` diagnostic, and remained a
non-pass result. The raw envelope is retained as `test-failure.json`.
