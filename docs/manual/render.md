# sc-compose render

`render` resolves a template, validates its inputs, and writes the rendered
result. With no `--output`, the result is written to standard output. The same
manual is available in the installed binary with `sc-compose help render`.

## Basic usage

```text
sc-compose render [OPTIONS]
```

In file mode (the default), provide `--file TEMPLATE` and, when needed,
`--root ROOT`. Profile mode selects a named profile instead:
`--mode profile --kind KIND --agent NAME`. Common input options include:

- `--var KEY=VALUE` (repeatable) and `--var-file PATH` for input values;
- `--env-prefix PREFIX` to import matching environment variables;
- `--strict` and `--unknown-var-mode MODE` for variable-policy checks;
- `--runtime RUNTIME` and `--root ROOT` for profile selection and path
  confinement.

Rendering options are `--output PATH`, `--guidance TEXT`,
`--guidance-file PATH` (or `-` for standard input), `--prompt TEXT`, and
`--prompt-file PATH`. Use `--json` for the versioned JSON envelope and
`--dry-run` to report the derived output target without writing files.

## Multiple passes and delimiters

`--all` renders all stacked template passes. Supply pass-scoped values with
`--pass N`, followed by that pass's `--var` and `--var-file` arguments. For a
single custom-delimiter pass, use either `--brace-count N` (where `N` is at
least 2) or `--variable-delimiters OPEN CLOSE`; these two options cannot be
combined with `--all` or with each other.

For example:

```shell
sc-compose render --file prompt.md.j2 --root . --var name=Ada
sc-compose render --file config.json.j2 --json --var environment=prod
sc-compose render --all --file staged.md.j2 \
  --pass 1 --var name=first --pass 2 --var name=second
```

## Common failures

- `ERR_CONFIG_MODE` means the selected file/profile mode does not match the
  supplied arguments.
- `ERR_RESOLVE_NOT_FOUND` or `ERR_RESOLVE_AMBIGUOUS` means the template or
  profile could not be resolved uniquely.
- `ERR_VAL_MISSING_REQUIRED`, `ERR_VAL_UNDECLARED_TOKEN`, and related
  `ERR_VAL_*` diagnostics identify input or strict-validation failures.
- `ERR_RENDER_WRITE` identifies an output-file or standard-output write
  failure. Invalid option combinations and malformed pass groups use
  `ERR_CONFIG_PARSE`.

Use `--json` when a caller needs diagnostics and recovery hints in a stable
machine-readable envelope; text mode prints the human-readable diagnostics.
