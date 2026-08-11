# title: Identity literals — approved canonical value
# sets:
#   - publish
#   - lint
# command: lint.identity-literals
# status: pass
# fixture: tests/fixtures/sc-lint/identity-literals/pass
# raw_json: reports/inputs/lint/identity-literals/pass.json

The pass fixture declares the fictional `team-lead@example.invalid` value as
the canonical identity and uses it only from
`crates/demo/src/constants.rs`. The test exercises the normal CI-materialized
Python adapter through the sc-compose target registry and records a zero-
finding pass.
