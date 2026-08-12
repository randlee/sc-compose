# sc-compose verify

After deploying a generated file, use `verify` to answer the practical
question “does this file still match its template?” It renders the template
and compares it with the deployed file without modifying either input. A
clean comparison exits successfully; drift gets a distinct status so release
scripts can reject stale output.

## Basic usage

```text
sc-compose verify [OPTIONS] DEPLOYED
```

In file mode, identify the template with `--file TEMPLATE` or `--against
TEMPLATE` (provide only one), then pass the concrete deployed file as the
positional `DEPLOYED` argument:

```shell
sc-compose verify --file prompt.md.j2 deployed/prompt.md
sc-compose verify --against prompt.md.j2 --json deployed/prompt.md
```

Profile mode uses the common `--mode`, `--kind`, `--agent`, `--runtime`, and
`--root` selectors. `--all` verifies stacked passes using pass-scoped
`--pass N`, `--var`, and `--var-file` groups. Repeat `--builtin-var
NAME=VALUE` to override a supported built-in value for deterministic output.
Use `--quiet` to suppress a text diff, or `--json` to return `clean`, paths,
and an optional diff in the diagnostic envelope.

## Exit status and failures

- `0` means the rendered result matches the deployed file.
- `1` means verification drift was found. Text mode prints the differing
  paths and, unless `--quiet` is set, the diff; JSON mode includes `clean:
  false` and the diff payload.
- `ERR_CONFIG_MODE` means a file-mode template was not supplied with `--file`
  or `--against`. Supplying both template options is an
  `ERR_CONFIG_PARSE` usage error.
- Resolution, validation, and rendering failures retain their specific
  `ERR_RESOLVE_*`, `ERR_VAL_*`, or render diagnostic codes.

Treat exit code `1` differently from an invalid invocation (exit code `3`),
so automation can distinguish stale deployment from a broken request.
