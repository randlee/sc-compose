# title: check xwin — capability available
# sets:
#   - publish
#   - lint
# command: check.xwin
# status: pass
# fixture: tests/fixtures/sc-lint/check-xwin/pass
# raw_json: reports/inputs/lint/check-xwin/pass.json

The pass fixture exercises `sc-lint --json --root . check xwin` through the
sc-compose target registry with the xwin capability available. The workflow
reports the canonical `x86_64-pc-windows-msvc` target and retains the raw
success envelope.
