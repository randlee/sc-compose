# sc-compose extract

When you inherit a rendered document but need the values that produced it,
use `extract` with the known template as a guide. It compares the two files
and recovers values conservatively, supporting structured XML, JSON, YAML, and
TOML output plus `raw` mode for known-template text.

## Basic usage

```text
sc-compose extract TEMPLATE RENDERED [OPTIONS]
```

`TEMPLATE` is the source template and `RENDERED` is the concrete output.
Choose `--format xml|json|yaml|toml|raw`; XML is the default. Repeat
`--include NAME` to recover only selected variables or `--exclude NAME` to
omit selected variables. `--json` emits values, occurrence provenance,
confidence, and warnings in the standard JSON envelope; text mode prints the
recovered report.

Examples:

```shell
sc-compose extract templates/config.json.j2 build/config.json \
  --format json --json
sc-compose extract prompt.md.j2 rendered.md --format raw \
  --include name --include project
```

Extraction is deliberately conservative. Values are reported only when the
template/output structure supplies enough evidence; repeated or conflicting
occurrences may be reported as ambiguous instead of silently choosing one.

## Common failures

- `ERR_CONFIG_READ` means either input file could not be read; check both
  paths and their permissions.
- Invalid include/exclude variable names use
  `ERR_EXTRACT_INVALID_REQUEST`.
- Malformed, unsupported, or ambiguous inputs use
  `ERR_EXTRACT_MALFORMED`, `ERR_EXTRACT_UNSUPPORTED`, or
  `ERR_EXTRACT_AMBIGUOUS`.
- Format-specific diagnostics include `ERR_EXTRACT_JSON_*`,
  `ERR_EXTRACT_YAML_*`, `ERR_EXTRACT_TOML_*`, and `ERR_EXTRACT_XML_*`.

Use JSON mode when downstream tooling needs occurrence paths, source spans,
or confidence without scraping human-readable output.
