# sc-compose reports

Reach for the `reports` command family when a build or review pipeline needs
its evidence collected in a predictable layout and checked before handoff.
The family creates and checks a shared report workspace using a catalog at
`reports/catalog/reports.toml`, latest outputs below `reports/latest/`, and
optional timestamped copies below `reports/archive/`. Each catalog entry
identifies an id, kind, producer, required flag, entrypoint, and metadata path.

## Commands

Initialize the scaffold:

```text
sc-compose reports init [--root <ROOT>] [--json]
```

This creates the catalog, latest/archive/template directories, and the smoke
fixture files. Run the smoke fixture:

```text
sc-compose reports smoke --fixture <FIXTURE> --vars <VARS>
    [--root <ROOT>] [--archive] [--json]
```

`--fixture` is a template path and `--vars` is a JSON or YAML object path,
both resolved under `--root`. The command writes the smoke entrypoint and
metadata to the latest report area; `--archive` also copies the artifact set
to a timestamped archive.

Finalize outputs from another producer:

```text
sc-compose reports finalize --report-id <ID> --kind <KIND>
    --entrypoint <PATH> [--artifact <PATH>]... [--status <STATUS>]
    [--root <ROOT>] [--archive] [--json]
```

`--status` defaults to `pass`. Artifacts must stay under the report's latest
output directory. Render a semantic diagram specification with
`reports render-spec`:

```text
sc-compose reports render-spec --spec <SPEC_PATH>
    [--root <ROOT>] [--archive] [--json]
```

Use `reports index` to summarize catalog entries and their latest sidecars,
`reports verify` to require every catalog entry marked `required = true` to
have its evidence, and `reports publish-manifest` to create the consolidated
machine-readable handoff:

```text
sc-compose reports index --root .
sc-compose reports verify --root .
sc-compose reports publish-manifest --root . --json
```

All seven subcommands accept `--root` (default `.`) and, where shown,
`--json`. Text output reports paths and statuses; JSON output is suitable for
automation.

## Catalog and layout

`reports init` creates a starter catalog resembling:

```toml
[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
```

Catalog kinds include `lint`, `test`, `smoke`, `diagram`, `state_machine`,
`sql_query`, and `custom`. Paths must be normalized and relative to the
workspace root. `reports index` reads each entrypoint and metadata sidecar,
then checks the artifact paths recorded in that sidecar. The publish manifest
is written at `reports/latest/publish-manifest.json` and maps report artifacts
to their publish destinations.

## Common failures

- A missing, malformed, duplicate, or invalid `reports.toml` entry reports
  `ERR_CONFIG_PARSE`. Required fields are `id`, `kind`, `producer`,
  `required`, `entrypoint`, and `metadata`.
- `reports verify` reports `ERR_CONFIG_PARSE` when required entrypoints,
  metadata, or recorded artifacts are missing. Run `reports index` to see the
  missing paths.
- Smoke fixtures and var-files must be readable and remain under `--root`.
  Read failures use `ERR_CONFIG_READ`; malformed or non-object variable data
  uses `ERR_CONFIG_PARSE` or `ERR_CONFIG_VARFILE`.
- `reports finalize` rejects artifacts outside the report's latest directory,
  and publish-manifest generation rejects unsafe artifact paths. These
  configuration failures use `ERR_CONFIG_PARSE`.
- A missing or invalid semantic spec for `render-spec` is also reported as a
  configuration parse failure; inspect the spec path and its required fields.
