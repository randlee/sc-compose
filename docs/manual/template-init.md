# sc-compose template-init

`template-init` converts a concrete file into a pass-aware sc-compose
template. It discovers the values you identify, adds frontmatter, and writes
the template back to the same file. Use `--dry-run` to inspect the proposed
template without writing it.

## Basic usage

```text
sc-compose template-init FILE [OPTIONS]
```

At least one pass group is required. Start each group with `--pass N`, then
provide one or more `--var KEY=VALUE` replacements and optional `--var-file
PATH` inputs. Pass number `0` is normalized to the default pass. `--force`
allows rewriting a file that already has frontmatter. `--json` emits the
changed path, variable list, and change status in the standard envelope.

Examples:

```shell
sc-compose template-init config.json \
  --pass 1 --var service=api --var region=us-west
sc-compose template-init prompt.md --pass 1 --var name=Ada --dry-run
sc-compose template-init config.json --pass 1 --var service=api \
  --json --force
```

JSON templates keep replacements as JSON values so a subsequent render can
round-trip strings and structured values without adding a second layer of
quoting. The command is intentionally in-place; copy the concrete source if
you need to preserve it.

## Common failures

- `ERR_CONFIG_PARSE` means a pass group is missing, a `--var` value is not
  `KEY=VALUE`, a variable name is invalid, or the file cannot be converted.
- Path canonicalization and read failures use `ERR_CONFIG_PARSE` with the
  affected path; inspect that path before retrying.
- `ERR_CONFIG_READONLY` means the file already contains frontmatter or cannot
  be rewritten; use `--force` only when replacing existing metadata is
  intended.
- Template parser and validation errors retain their specific `ERR_VAL_*` or
  render diagnostic code.

For automation, combine `--json` with `--dry-run` to preview the exact file
impact before allowing an in-place write.
