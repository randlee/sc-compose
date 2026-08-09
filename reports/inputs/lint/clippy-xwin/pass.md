# title: clippy xwin pass fixture
# sets:
#   - lint
#   - clippy-xwin
# copy_json:
#   command: clippy.xwin
#   fixture: tests/fixtures/sc-lint/clippy-xwin/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/clippy-xwin/pass.json

Input fixture: `tests/fixtures/sc-lint/clippy-xwin/pass`.

The actual `sc-lint --json --root <fixture> clippy xwin` invocation completed
the cargo xwin clippy step for `x86_64-pc-windows-msvc` with no diagnostics.
The raw envelope is retained next to this source entry as `pass.json`; the
rendered panel is `pass-panel.html`.
