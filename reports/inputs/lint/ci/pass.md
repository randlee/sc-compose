# title: top-level ci pass fixture
# sets:
#   - lint
#   - ci
# copy_json:
#   command: ci
#   fixture: tests/fixtures/sc-lint/ci/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/ci/pass.json

Input fixture: `tests/fixtures/sc-lint/ci/pass`.

The actual `sc-lint --json --root <fixture> ci` workflow completed the CI lint
profile and the final workspace-test step. The run had xwin available, but the
top-level CI profile correctly excluded xwin-only steps. The raw envelope is
retained as `pass.json`.
