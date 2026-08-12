# sc-compose feature manuals

Use these pages when you need the behavior of a `sc-compose` feature in a
script, CI job, or interactive session. They are bundled into the executable,
so `sc-compose help <topic>` works without a checkout, network connection, or
extra documentation files.

Run `sc-compose help` for a human-readable index. Run `sc-compose help --list`
when a script needs the same topic names as one stable, newline-delimited list.

- [Exit codes](exit-codes.md) — the process-status contract for automation.
- [Frontmatter initialization](frontmatter-init.md) — add required-variable
  frontmatter to a concrete template file.
- [Workspace initialization](init.md) — bootstrap `.prompts/` and validate
  workspace templates.
- [Examples](examples.md) — list and render bundled example packs.
- [Templates](templates.md) — import, list, and render user template packs.
- [Reports](reports.md) — create, materialize, verify, and publish report
  artifacts.

Additional feature manuals should add a document here and register it in the
CLI's ordered manual-topic registry so the index and `--list` output stay in
the same deterministic order.
