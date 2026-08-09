# title: clippy.native pass fixture
# sets:
#   - lint
#   - clippy-native
# copy_json:
#   command: clippy.native
#   fixture: tests/fixtures/sc-lint/clippy-native/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/clippy-native/pass.json

Input fixture: `tests/fixtures/sc-lint/clippy-native/pass`.

The actual `sc-lint --json --root <fixture> clippy native` invocation ran the
native `cargo clippy --workspace --all-targets -- -D warnings` workflow. The
structured envelope preserves the `clippy.native` command and step identity
with `data.status = "pass"`; the raw envelope is retained as `pass.json` and
the rendered panel as `pass-panel.html`.
