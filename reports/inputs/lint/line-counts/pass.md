# title: line-counts below-limit fixture
# sets:
#   - lint
#   - line-counts
# copy_json:
#   command: lint.line-counts
#   fixture: tests/fixtures/sc-lint/line-counts/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/line-counts/pass.json

Input fixture: `tests/fixtures/sc-lint/line-counts/pass`.

The pinned sc-lint Python adapter found that the source file remains below the
configured production limit. The raw envelope is retained next to this source
entry as `pass.json`; the rendered panel is `pass-panel.html`.
