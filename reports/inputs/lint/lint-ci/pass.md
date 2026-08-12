# title: lint.ci known sc-lint boundary packaging defect
# sets:
#   - lint
#   - lint-ci
# copy_json:
#   command: lint.ci
#   fixture: tests/fixtures/sc-lint/lint-ci/pass
#   outcome: failed
#   rule_id: CLI.BACKEND_EXEC_FAILURE
#   raw_artifact: reports/inputs/lint/lint-ci/pass.json

Input fixture: `tests/fixtures/sc-lint/lint-ci/pass`.

The actual `sc-lint --json --root <fixture> lint ci` invocation reaches the
CI profile's `sc-boundary` utility and preserves sc-lint#84's known packaging
defect: the utility asks Cargo for `sc-lint-boundary`, which is not a member of
the consumer workspace. This is retained as a non-pass backend result rather
than converted to a successful lint report; `lint ci` remains lint-only and
does not include top-level CI tests. The raw envelope is retained as `pass.json`
and the rendered panel as `pass-panel.html`.
