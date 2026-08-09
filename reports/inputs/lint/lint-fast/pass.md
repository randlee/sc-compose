# title: lint.fast pass fixture
# sets:
#   - lint
#   - lint-fast
# copy_json:
#   command: lint.fast
#   fixture: tests/fixtures/sc-lint/lint-fast/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/lint-fast/pass.json

Input fixture: `tests/fixtures/sc-lint/lint-fast/pass`.

The actual `sc-lint --json --root <fixture> lint fast` invocation preserves the
five-step fast profile (`fmt`, `version`, `manifests`, `spell`, and `pytests`)
with xwin excluded. The raw envelope is retained as `pass.json` and the
rendered panel as `pass-panel.html`.
