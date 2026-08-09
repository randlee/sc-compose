# title: clippy.native warning fixture
# sets:
#   - lint
#   - clippy-native
# copy_json:
#   command: clippy.native
#   fixture: tests/fixtures/sc-lint/clippy-native/warning
#   outcome: failed
#   rule_id: CLI.BACKEND_EXEC_FAILURE
#   raw_artifact: reports/inputs/lint/clippy-native/warning.json

Input fixture: `tests/fixtures/sc-lint/clippy-native/warning`.

The actual `sc-lint --json --root <fixture> clippy native` invocation preserves
the native `cargo clippy --workspace --all-targets -- -D warnings` failure as a
non-pass command result. The structured error is
`CLI.BACKEND_EXEC_FAILURE` for step `clippy.native` and retains the
`clippy::len-zero` diagnostic; the raw envelope is retained as `warning.json`
and the rendered panel as `warning-panel.html`.
