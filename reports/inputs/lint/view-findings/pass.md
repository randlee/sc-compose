# title: view findings stored-artifact collation
# sets:
#   - lint
#   - view-findings
# copy_json:
#   command: view.findings
#   fixture: tests/fixtures/sc-lint/view-findings/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/view-findings/pass.json

Input fixture: `tests/fixtures/sc-lint/view-findings/pass`.

The stored findings payload contains one failing runtime finding and one
passing portability artifact. The actual `sc-lint --json --root <fixture>
view findings` invocation collated both artifact sets while preserving their
tool identities, statuses, summaries, and finding count; the raw envelope is
retained as `pass.json` and the rendered panel is `pass-panel.html`.
