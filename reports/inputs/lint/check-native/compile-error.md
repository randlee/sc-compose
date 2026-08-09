# title: check.native compile-error fixture
# sets:
#   - lint
#   - check-native
# copy_json:
#   command: check.native
#   fixture: tests/fixtures/sc-lint/check-native/compile-error
#   outcome: failed
#   rule_id: CLI.BACKEND_EXEC_FAILURE
#   raw_artifact: reports/inputs/lint/check-native/compile-error.json

Input fixture: `tests/fixtures/sc-lint/check-native/compile-error`.

The actual `sc-lint --json --root <fixture> check native` invocation preserves
the native `cargo check --workspace` failure as a non-pass command result.
The structured error is `CLI.BACKEND_EXEC_FAILURE` for step `check.native` and
retains the compiler's `MissingType` diagnostic; the raw envelope is retained
as `compile-error.json` and the rendered panel as `compile-error-panel.html`.
