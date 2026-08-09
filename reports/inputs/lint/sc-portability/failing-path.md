# title: sc-portability hardcoded Unix path fixture
# sets:
#   - lint
#   - sc-portability
# copy_json:
#   command: lint.sc-portability
#   fixture: tests/fixtures/sc-lint/sc-portability/failing-path
#   outcome: findings
#   rule_id: PORT-001
#   raw_artifact: reports/inputs/lint/sc-portability/failing-path.json

Input fixture: `tests/fixtures/sc-lint/sc-portability/failing-path`.

The test helper deliberately constructs `/tmp/sc-compose-portability`, a
Unix-only path. The expected structured result is `PORT-001` with
`data.status = "fail"`; the raw envelope is retained as `failing-path.json`
and the rendered panel is `failing-path-panel.html`.
