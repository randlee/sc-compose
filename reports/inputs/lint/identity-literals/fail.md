# title: Identity literals — duplicated and test-scope values
# sets:
#   - publish
#   - lint
# command: lint.identity-literals
# status: findings
# fixture: tests/fixtures/sc-lint/identity-literals/fail
# raw_json: reports/inputs/lint/identity-literals/fail.json

The fail fixture declares the fictional `team-lead@example.invalid` value as
canonical only in `crates/demo/src/constants.rs`, then repeats it in
`crates/demo/src/lib.rs` and in a test. sc-lint reports one production-scope
canonical-literal finding and two test-scope forbidden-literal findings. The
sc-compose report must retain that structured non-pass result rather than
turning it into a successful report with warning text.
