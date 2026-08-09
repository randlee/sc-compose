# title: check xwin — unavailable capability
# sets:
#   - publish
#   - lint
# command: check.xwin
# status: capability_error
# fixture: tests/fixtures/sc-lint/check-xwin/capability-negative
# raw_json: reports/inputs/lint/check-xwin/capability-negative.json

The capability-negative fixture runs the same allowlisted target without
`cargo xwin`. sc-lint emits `CLI.CAPABILITY_ERROR` with the required tool and
target details. This panel documents the explicit non-pass result; it must not
be converted into a successful report with warning text.
