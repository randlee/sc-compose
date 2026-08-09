# title: clippy xwin failing analysis fixture
# sets:
#   - lint
#   - clippy-xwin
# copy_json:
#   command: clippy.xwin
#   fixture: tests/fixtures/sc-lint/clippy-xwin/failing-analysis
#   outcome: failed
#   error_code: CLI.BACKEND_EXEC_FAILURE
#   raw_artifact: reports/inputs/lint/clippy-xwin/failing-analysis.json

Input fixture: `tests/fixtures/sc-lint/clippy-xwin/failing-analysis`.

The fixture intentionally triggers `unused-mut` under `-D warnings`. The
expected result is a structured `CLI.BACKEND_EXEC_FAILURE` from the
`clippy.xwin` step, not a pass with warning text. The raw envelope is retained
as `failing-analysis.json`; the rendered panel is
`failing-analysis-panel.html`.
