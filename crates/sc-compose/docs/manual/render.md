# sc-compose render

When you need to turn a template into a concrete document, use `render`. It
prepares the template's inputs, checks them, and writes the finished result so
you can use it in a file, a pipeline, or another command. With no `--output`,
the result goes to standard output. The same manual is available in the
installed binary with `sc-compose help render`.

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

For JSON templates, `--json-escape-mode auto|legacy` selects the interpolation
contract. `auto` is the default and expects bare placeholders such as
`{{ value }}` in a JSON value position; the renderer owns JSON quoting and
preserves the value's type. `legacy` safely supports existing manually quoted
string placeholders such as `"{{ value }}"` without adding a second set of
quotes. A root frontmatter `json_escape_mode` is used when the flag is absent.

Rendering options are `--output PATH`, `--guidance TEXT`,
`--guidance-file PATH` (or `-` for standard input), `--prompt TEXT`, and
`--prompt-file PATH`. Use `--json` for the versioned JSON envelope,
`--dry-run` to report the derived output target without writing files, and
`--check-render` to request an explicit checked-render report before emission.

JSON templates are always checked before ordinary `render` emits stdout or
creates an output file. A malformed result fails closed with
`ERR_RENDER_JSON_MALFORMED`; the rendered body is never emitted. JSON supports
at most 127 nested objects or arrays, counting the root container as level 1.
Exceeding this bound during parsing fails with
`ERR_RENDER_JSON_DEPTH_LIMIT` (exit `2`) naming the limit, rather than claiming
the body is malformed. This limit also applies to `--append`; a depth failure
leaves its destination unchanged. The
`--check-render` flag also applies this gate to non-JSON text and includes a
`render_check` object in JSON output. A successful check reports
`state: "render_checked"`; callers must not treat a static validation result
as proof that a context-specific render is safe.

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

## Appending JSON records

`--append PATH` adds the rendered result to a JSON Lines file as one record. It
is meant for logs and ledgers that several processes write at once, such as
one record per test run or review.

```shell
sc-compose render --file run-record.json.j2 --var-file run.json --strict \
  --append logs/runs.jsonl
```

In order, `render --append`:

1. renders and validates exactly as an ordinary render does, including
   `--strict` and the JSON template checks; nothing is opened yet;
2. requires the result to be exactly one JSON object
   (`ERR_RENDER_JSON_MALFORMED` if it is not JSON,
   `ERR_RENDER_APPEND_NOT_OBJECT` if it is an array, string or number);
   because the object is parsed and re-serialized, its key order may differ
   from the rendered source order;
3. writes it as one compact UTF-8 line (embedded newlines stay escaped)
   followed by `\n`;
4. opens the file for append (creating it if needed) and holds an exclusive
   cross-process lock across the check and the write, so concurrent appends
   never interleave and none is lost;
5. refuses, unchanged, a non-empty file whose last byte is not `\n`
   (`ERR_RENDER_APPEND_NO_FINAL_NEWLINE`): repair the file's last line first.

Guarantees: any failure before the write leaves existing content
byte-for-byte unchanged (a file that did not exist may be left empty after an
open or lock failure). A lock or write failure is `ERR_RENDER_WRITE`; if a
write fails part-way, the file is truncated back to its previous length, or
the diagnostic says a partial last line may remain if that also fails. All of
these exit `2`. Nothing stronger is claimed for arbitrary disk failures.

`--append` cannot be combined with `--output` or `--dry-run`, and is not
accepted by `examples` or `templates` (usage errors, exit `3`). With `--json`,
the payload reports `output_path` (the file), `bytes_written` (including the
final newline) and `appended: true`, and carries no rendered body. Timestamps
in records come from your inputs; sc-compose adds none.

## Common failures

- `ERR_CONFIG_MODE` means the selected file/profile mode does not match the
  supplied arguments.
- `ERR_RESOLVE_NOT_FOUND` or `ERR_RESOLVE_AMBIGUOUS` means the template or
  profile could not be resolved uniquely.
- `ERR_VAL_MISSING_REQUIRED`, `ERR_VAL_UNDECLARED_TOKEN`, and related
  `ERR_VAL_*` diagnostics identify input or strict-validation failures.
- `ERR_RENDER_WRITE` identifies an output-file, standard-output, append-lock
  or append-write failure. Invalid option combinations and malformed pass groups use
  `ERR_CONFIG_PARSE`.
- `WARN_JSON_LEGACY_ESCAPE_MODE` identifies a quoted-placeholder shape or
  explicit legacy mode that should be migrated to bare placeholders and
  `auto`. `ERR_JSON_ESCAPE_MODE_NON_JSON` means a JSON mode was selected for a
  non-JSON template.
- `ERR_RENDER_JSON_DEPTH_LIMIT` identifies output exceeding 127 nested JSON
  objects or arrays; its diagnostic gives the limit and source location.
- `ERR_RENDER_JSON_MALFORMED` identifies a complete rendered JSON body that
  failed parsing. Its diagnostic includes the template, line, column, and byte
  offset but does not echo rendered values.

Use `--json` when a caller needs diagnostics and recovery hints in a stable
machine-readable envelope; text mode prints the human-readable diagnostics.
