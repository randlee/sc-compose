# title: sc-runtime safe synchronization fixture
# sets:
#   - lint
#   - sc-runtime
# copy_json:
#   command: lint.sc-runtime
#   fixture: tests/fixtures/sc-lint/sc-runtime/pass
#   outcome: pass
#   raw_artifact: reports/inputs/lint/sc-runtime/pass.json

Input fixture: `tests/fixtures/sc-lint/sc-runtime/pass`.

The production synchronization helper uses `wait_timeout(...)` and inspects the
returned `WaitTimeoutResult`. The actual `sc-lint --json --root <fixture>
lint sc-runtime` invocation found no runtime findings; the raw envelope is
retained as `pass.json` and the rendered panel is `pass-panel.html`.
