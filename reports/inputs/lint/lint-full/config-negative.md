# title: lint.full missing-utility configuration fixture
# sets:
#   - lint
#   - lint-full
# copy_json:
#   command: lint.full
#   fixture: tests/fixtures/sc-lint/lint-full/config-negative
#   outcome: config_error
#   raw_artifact: reports/inputs/lint/lint-full/config-negative.json

Input fixture: `tests/fixtures/sc-lint/lint-full/config-negative`.

Without the CI-materialized `.just` utilities, the real profile stops at the
missing `lint_cargo_deny.py` utility. sc-compose preserves the upstream
`CLI.BACKEND_EXEC_FAILURE` payload while classifying the result as
`config_error`; this is the documented L.1 packaging characterization.
