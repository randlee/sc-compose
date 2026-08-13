# sc-compose validate

Before writing generated output, use `validate` to find missing inputs or
template mistakes early. It resolves and checks frontmatter, required inputs,
referenced variables, and template structure without producing rendered
output. Plain validation is explicitly `static_only`: it does not render and
does not prove that a future context-specific output will parse. A successful
text invocation prints `valid (static_only)`; JSON mode preserves the state
and diagnostics in the standard envelope.

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
- `--check-render` renders in memory using the supplied context, checks the
  exact output before returning, and never writes or emits the rendered body;
- `--json-escape-mode auto|legacy` selects JSON interpolation mode. When the
  flag is absent, root frontmatter selects the mode and otherwise validation
  uses `auto`;
- `--json` emits `{ "valid": ... }` and the diagnostics array in the JSON
  envelope.

Examples:

```shell
sc-compose validate --file prompt.md.j2 --root . --var name=Ada
sc-compose validate --file prompt.md.j2 --strict --lint --json
sc-compose validate --check-render --file config.json.j2 --var environment=prod --json
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

For JSON templates, validation emits `WARN_JSON_LEGACY_ESCAPE_MODE` once when
legacy mode is selected or a manually quoted placeholder is detected. The
warning includes the migration guidance and points to
`docs/migration/json-escape-mode.md`. `validate --lint` includes this same
contract warning in addition to lint findings; it does not use a separate JSON
renderer or mode resolver.

With `--check-render`, inspect the structured state. `render_checked` means the
exact supplied context rendered and passed the format gate; `context_required`,
`contract_invalid`, and `render_invalid` are non-emitting states. In JSON mode,
the result is in `payload` and has no rendered body or output-file field.
