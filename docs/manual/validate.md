# sc-compose validate

`validate` resolves and checks a template without producing rendered output.
It verifies frontmatter, required inputs, referenced variables, and template
structure. A successful text invocation prints `valid`; JSON mode preserves
the result and diagnostics in the standard envelope.

## Basic usage

```text
sc-compose validate [OPTIONS]
```

File mode is the default. Supply `--file TEMPLATE` and optionally `--root
ROOT`, input values, `--strict`, `--unknown-var-mode`, and `--env-prefix`.
Profile mode uses the same selectors as `render` and `resolve`:
`--mode profile --kind KIND --agent NAME --runtime RUNTIME`.

Useful validation switches are:

- `--all` validates all stacked template passes; pass-scoped `--pass N`,
  `--var`, and `--var-file` values can be supplied for each pass;
- `--lint` reports redundant filter chains and other lint findings with source
  locations;
- `--json` emits `{ "valid": ... }` and the diagnostics array in the JSON
  envelope.

Examples:

```shell
sc-compose validate --file prompt.md.j2 --root . --var name=Ada
sc-compose validate --file prompt.md.j2 --strict --lint --json
sc-compose validate --all --file staged.md.j2 \
  --pass 1 --var name=first --pass 2 --var name=second
```

## Common failures

Validation failures normally carry `ERR_VAL_MISSING_REQUIRED`,
`ERR_VAL_MISSING_NESTED_FIELD`, `ERR_VAL_SHAPE_MISMATCH`,
`ERR_VAL_UNDECLARED_TOKEN`, or another `ERR_VAL_*` diagnostic. Include and
profile resolution problems can produce `ERR_RESOLVE_NOT_FOUND` or
`ERR_RESOLVE_AMBIGUOUS`; invalid modes and malformed pass groups use
`ERR_CONFIG_MODE` or `ERR_CONFIG_PARSE`.

In text mode, diagnostics are printed one per line. In JSON mode, inspect the
diagnostic code and source location rather than parsing display text.
