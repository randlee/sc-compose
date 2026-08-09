# title: view findings malformed stored payload
# sets:
#   - lint
#   - view-findings
# copy_json:
#   command: view.findings
#   fixture: tests/fixtures/sc-lint/view-findings/malformed-summary
#   outcome: failed
#   raw_artifact: reports/inputs/lint/view-findings/malformed-summary.json

Input fixture: `tests/fixtures/sc-lint/view-findings/malformed-summary`.

The stored findings summary is malformed JSON. The actual
`sc-lint --json --root <fixture> view findings` invocation returns
`CLI.BACKEND_PROTOCOL_ERROR`, exit status `6`, and a non-pass report; the raw
error envelope is retained as `malformed-summary.json` and the rendered panel
is `malformed-summary-panel.html`.
