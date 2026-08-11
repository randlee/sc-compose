# title: lint.full pass fixture
# sets:
#   - lint
#   - lint-full
# copy_json:
#   command: lint.full
#   fixture: tests/fixtures/sc-lint/lint-full/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/lint-full/pass.json

Input fixture: `tests/fixtures/sc-lint/lint-full/pass`.

The real `sc-lint --json --root <fixture> lint full` workflow completed all
profile steps, including the conditional xwin checks in this deterministic
host-side run. The raw envelope is retained as `pass.json`.
