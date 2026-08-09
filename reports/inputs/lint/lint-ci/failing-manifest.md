# title: lint.ci manifest failure fixture
# sets:
#   - lint
#   - lint-ci
# copy_json:
#   command: lint.ci
#   fixture: tests/fixtures/sc-lint/lint-ci/failing-manifest
#   outcome: failed
#   rule_id: CLI.BACKEND_EXEC_FAILURE
#   raw_artifact: reports/inputs/lint/lint-ci/failing-manifest.json

Input fixture: `tests/fixtures/sc-lint/lint-ci/failing-manifest`.

The actual `sc-lint --json --root <fixture> lint ci` invocation preserves the
manifest-policy failure as a non-pass `CLI.BACKEND_EXEC_FAILURE` at step
`manifests`; the structured output identifies the missing
`[package].homepage.workspace = true` field. The raw envelope is retained as
`failing-manifest.json` and the rendered panel as `failing-manifest-panel.html`.
