# sc-compose exit codes

Every `sc-compose` command ends by giving the shell a small status number.
That number lets a script, CI job, or calling application decide what happened
without parsing human-readable output: `0` is clean success, while a non-zero
status means the result needs attention. Use the status to choose the next
automation step, then use the command's diagnostic code or message for detail.

There is one intentional distinction: `verify` can complete its comparison
successfully and still return `1` when the deployed file differs from the
rendered template. That is a valid drift result, not a parser or rendering
failure. All other commands use only `0`, `2`, and `3`.

| Code | Meaning |
|---:|---|
| `0` | Success. The requested operation completed. |
| `1` | `verify` only: a successful comparison found drift between the deployed file and the rendered template. |
| `2` | Validation or render failure. The input was understood, but it could not be validated or rendered. |
| `3` | Usage or configuration failure. The command line or configuration was invalid. |

Display requests such as `sc-compose --help` and `sc-compose --version` return
`0`. For non-display commands, scripts should treat any non-zero status as a
result that is not clean and use the command's diagnostic code or stderr
message for details.
