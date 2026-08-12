# sc-compose feature manuals

These manuals are bundled into the `sc-compose` executable and can be read
from the command line with `sc-compose help <topic>`. Run `sc-compose help`
for the index, or `sc-compose help --list` for a stable, scriptable topic list.

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
CLI's ordered manual-topic registry.
