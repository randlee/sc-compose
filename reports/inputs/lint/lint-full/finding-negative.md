# title: lint.full finding fixture
# sets:
#   - lint
#   - lint-full
# copy_json:
#   command: lint.full
#   fixture: tests/fixtures/sc-lint/lint-full/finding-negative
#   outcome: failed
#   raw_artifact: reports/inputs/lint/lint-full/finding-negative.json

Input fixture: `tests/fixtures/sc-lint/lint-full/finding-negative`.

The real full-profile workflow preserved an intentional structured utility
finding as `CLI.BACKEND_EXEC_FAILURE` at the deny step. The result remains a
non-pass outcome and is retained as `finding-negative.json`.
