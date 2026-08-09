# title: sc-runtime unsafe wait-pattern fixture
# sets:
#   - lint
#   - sc-runtime
# copy_json:
#   command: lint.sc-runtime
#   fixture: tests/fixtures/sc-lint/sc-runtime/unsafe-wait
#   outcome: findings
#   rule_id: SCB-RUNTIME-001
#   raw_artifact: reports/inputs/lint/sc-runtime/unsafe-wait.json

Input fixture: `tests/fixtures/sc-lint/sc-runtime/unsafe-wait`.

The production helper uses a bare `Condvar::wait(...)` without timeout
inspection. The expected structured result is `SCB-RUNTIME-001` with
`data.status = "fail"` and a non-pass consumer outcome; the raw envelope is
retained as `unsafe-wait.json` and the rendered panel is
`unsafe-wait-panel.html`.
