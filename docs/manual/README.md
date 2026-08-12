# sc-compose feature manuals

Use these pages when you need the behavior of a `sc-compose` feature in a
script, CI job, or interactive session. They are bundled into the executable,
so `sc-compose help <topic>` works without a checkout, network connection, or
extra documentation files.

Run `sc-compose help` for a human-readable index. Run `sc-compose help --list`
when a script needs the same topic names as one stable, newline-delimited list.

- [Exit codes](exit-codes.md) — the process-status contract for automation.
- [Render](render.md) — resolve, validate, and write a rendered template.
- [Resolve](resolve.md) — locate a concrete profile template and inspect its search trace.
- [Validate](validate.md) — check templates, variables, and stacked passes without rendering.
- [Verify](verify.md) — compare rendered output with a deployed file.
- [Extract](extract.md) — recover values from a known template and rendered output.
- [Template init](template-init.md) — convert a concrete file into a pass-aware template.

Additional feature manuals should add a document here and register it in the
CLI's ordered manual-topic registry so the index and `--list` output stay in
the same deterministic order.
