# title: sc-boundary dependency-policy violation fixture
# sets:
#   - lint
#   - sc-boundary
# copy_json:
#   command: lint.sc-boundary
#   fixture: tests/fixtures/sc-lint/sc-boundary/dependency-violation
#   outcome: findings
#   rule_id: SCB-DEPENDENCY-001
#   raw_artifact: reports/inputs/lint/sc-boundary/dependency-violation.json

Input fixture: `tests/fixtures/sc-lint/sc-boundary/dependency-violation`.

The fixture deliberately makes `boundary-app` depend directly on
`boundary-api` without listing that package in `BOUNDARY-BoundaryApp`'s
`allowed_dependencies`. The expected structured result is
`SCB-DEPENDENCY-001` with a non-pass `data.status`; the raw envelope is
retained as `dependency-violation.json` and the rendered panel is
`dependency-violation-panel.html`.
