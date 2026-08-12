# title: line-counts over-limit fixture
# sets:
#   - lint
#   - line-counts
# copy_json:
#   command: lint.line-counts
#   fixture: tests/fixtures/sc-lint/line-counts/over-limit
#   outcome: findings
#   rule_id: line-counts
#   raw_artifact: reports/inputs/lint/line-counts/over-limit.json

Input fixture: `tests/fixtures/sc-lint/line-counts/over-limit`.

The fixture configures a five-line production limit while its source contains
18 production lines. The adapter preserves the structured finding and
`data.status = "fail"`; the raw envelope is retained as `over-limit.json` and
the rendered panel is `over-limit-panel.html`.
