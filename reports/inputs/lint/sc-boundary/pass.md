# title: sc-boundary pass fixture
# sets:
#   - lint
#   - sc-boundary
# copy_json:
#   command: lint.sc-boundary
#   fixture: tests/fixtures/sc-lint/sc-boundary/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/sc-boundary/pass.json

Input fixture: `tests/fixtures/sc-lint/sc-boundary/pass`.

The actual `sc-lint --json --root <fixture> lint sc-boundary` invocation found
no boundary or manifest-policy findings. The raw envelope is retained next to
this source entry as `pass.json`; the rendered panel is `pass-panel.html`.
