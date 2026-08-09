# title: check.native pass fixture
# sets:
#   - lint
#   - check-native
# copy_json:
#   command: check.native
#   fixture: tests/fixtures/sc-lint/check-native/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/check-native/pass.json

Input fixture: `tests/fixtures/sc-lint/check-native/pass`.

The actual `sc-lint --json --root <fixture> check native` invocation ran the
native `cargo check --workspace` workflow. The structured envelope preserves
the `check.native` command and step identity with `data.status = "pass"`;
the raw envelope is retained as `pass.json` and the rendered panel as
`pass-panel.html`.
