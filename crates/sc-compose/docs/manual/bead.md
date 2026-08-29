# Beads formula composition

`sc-compose bead` is a thin adapter for one complete `sc-compose/beads/v1`
JSON request. Choose exactly one operation and provide exactly one request file:

```text
sc-compose bead render --request request.json [--json]
sc-compose bead validate --request request.json [--json]
sc-compose bead preview-pour --request request.json [--json]
sc-compose bead pour --request request.json [--json]
```

The selected subcommand determines `operation`; all other request fields come
from the file. There are no partial request flags.

## Variables

`compose_variables` are structured JSON values expanded by sc-compose with
triple braces, for example `{{{ project.name }}}`. `bead_variables` are scalar
runtime values passed unchanged to Beads, so formulas retain ordinary Beads
placeholders such as `{{ release_name }}`.

## Validation and pour

`validate` renders then runs `bd cook --dry-run`. `preview-pour` additionally
uses `bd where --json` to resolve the active Beads registry. Its rendered file
must be that registry's `formulas/<name>.formula.toml` or `.formula.json`; a
same-name TOML and JSON pair is rejected as ambiguous before pour.

`pour` is irreversible: it creates persistent Beads state only when the
request contains the exact `CreatePersistentBeads` authorization value.

Use `--json` for the standard sc-compose diagnostic envelope. On success its
payload is the versioned `sc-compose/beads/v1` receipt. On pre-execution
failure its payload preserves the stable Beads error code and message.

Human output is deliberately receipt-derived and does not include a Beads
executable version in protocol v1. Adding that value requires a future R.1
receipt-contract amendment; this adapter does not run an extra `bd` command to
derive it.
