# sc-compose exit codes

The `sc-compose` process status is stable for shell scripts and other
automation. A successful command returns `0`; failures use the categories
below.

| Code | Meaning |
|---:|---|
| `0` | Success. The requested operation completed. |
| `1` | Verification drift. The deployed file differs from the rendered template. |
| `2` | Validation or render failure. The input was understood, but it could not be validated or rendered. |
| `3` | Usage or configuration failure. The command line or configuration was invalid. |

Display requests such as `sc-compose --help` and `sc-compose --version` return
`0`. Scripts should treat any non-zero status as a failed operation and use
the command's diagnostic code or stderr message for details.
