# sc-compose feature manuals

These manuals are bundled into the `sc-compose` executable and can be read
from the command line with `sc-compose help <topic>`. Run `sc-compose help`
for the index, or `sc-compose help --list` for a stable, scriptable topic list.

- [Exit codes](exit-codes.md) — the process-status contract for automation.
- [Render](render.md) — resolve, validate, and write a rendered template.
- [Resolve](resolve.md) — locate a concrete profile template and inspect its search trace.
- [Validate](validate.md) — check templates, variables, and stacked passes without rendering.
- [Verify](verify.md) — compare rendered output with a deployed file.
- [Extract](extract.md) — recover values from a known template and rendered output.
- [Template init](template-init.md) — convert a concrete file into a pass-aware template.

Additional feature manuals should add a document here and register it in the
CLI's ordered manual-topic registry.
