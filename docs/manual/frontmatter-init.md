# sc-compose frontmatter-init

Reach for `frontmatter-init` when you already have a concrete file with Jinja
placeholders and want to make it a usable sc-compose template quickly. It
discovers the variables in the body and adds the smallest useful declaration:
normalized YAML frontmatter with `required_variables`, an empty `defaults`
map, and an empty `metadata` map. It does not replace literal values; use
`template-init` when the file also needs pass-scoped substitutions.

## Usage

```text
sc-compose frontmatter-init --file <FILE> [--force] [--dry-run] [--json]
```

`--file` is required. `--force` allows an existing frontmatter block to be
rewritten from the variables currently found in the body. Without `--force`,
an existing block is left untouched and the command fails. `--dry-run`
prints the generated frontmatter without writing the file. `--json` emits the
standard machine-readable diagnostics envelope; in dry-run mode its payload
also reports `would_affect`, `changed`, `would_change`, and `vars`.

## Examples

Preview the header for a concrete file:

```console
$ printf 'Hello {{ name }}!\n' > greeting.md.j2
$ sc-compose frontmatter-init --file greeting.md.j2 --dry-run
---
required_variables:
  - name
defaults: {}
metadata: {}
---
```

Write the header, or deliberately regenerate an existing one:

```console
$ sc-compose frontmatter-init --file greeting.md.j2
$ sc-compose frontmatter-init --file greeting.md.j2 --force
```

The discovered names include variables used through loop expressions and
dotted references, so a body such as `{{ report.title }}` records
`report.title` in `required_variables`.

## Common failures

- A missing file, an escaping symlink, or another unresolved target reports
  `ERR_RESOLVE_NOT_FOUND`. Inspect the path and pass the intended concrete
  file.
- If the file already has frontmatter and `--force` is absent, the command
  reports `ERR_CONFIG_READONLY` and explains how to rerun it.
- Malformed existing frontmatter or a read failure reports
  `ERR_CONFIG_PARSE`; a write failure reports `ERR_CONFIG_READONLY`.

Use `--json` when a script needs the discovered variable list or a stable
diagnostic code instead of parsing human-readable output.
