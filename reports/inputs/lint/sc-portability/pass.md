# title: sc-portability pass fixture
# sets:
#   - lint
#   - sc-portability
# copy_json:
#   command: lint.sc-portability
#   fixture: tests/fixtures/sc-lint/sc-portability/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/sc-portability/pass.json

Input fixture: `tests/fixtures/sc-lint/sc-portability/pass`.

The actual `sc-lint --json --root <fixture> lint sc-portability` invocation
found no portability findings. The raw envelope is retained next to this
source entry as `pass.json`; the rendered panel is `pass-panel.html`.
