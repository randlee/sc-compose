# sc-compose templates

`templates` manages user-owned template packs. Packs live below the user
template directory, which can be overridden with `SC_COMPOSE_TEMPLATE_DIR`.
Each renderable pack is a directory containing exactly one root-level `*.j2`
file. An optional `template.json` may provide `description`, `version`, and
validated `input_defaults` values.

## Usage

List packs:

```text
sc-compose templates list [--json]
```

Import a file or directory as a pack:

```text
sc-compose templates add <SRC> [NAME] [--json]
```

Render a pack:

```text
sc-compose templates <NAME> [--var <KEY=VALUE>]... [--var-file <FILE>]...
    [--env-prefix <PREFIX>] [--strict]
    [--unknown-var-mode <error|warn|ignore>]
    [--output <PATH>] [--guidance <TEXT>] [--guidance-file <FILE>]
    [--prompt <TEXT>] [--prompt-file <FILE>] [--json] [--dry-run]
```

The render options have the same meaning as they do for `examples`. `add`
derives a pack name from the source when `NAME` is omitted, creates the user
template root and its README when necessary, and copies the source into the
new pack. The `template.json` `input_defaults` map supplies optional render
inputs that callers can override.

## Examples

Import and render a single-file pack in an explicit user template directory:

```console
$ printf 'Hello {{ name }}!\n' > greeting.md.j2
$ export SC_COMPOSE_TEMPLATE_DIR="$PWD/.sc-compose-templates"
$ sc-compose templates add ./greeting.md.j2 greeting
$ sc-compose templates list
greeting
$ sc-compose templates greeting --var name=Casey
Hello Casey!
```

A directory pack can include a root-level template and `template.json`:

```json
{
  "description": "A greeting pack",
  "version": "1.0.0",
  "input_defaults": { "name": "world" }
}
```

## Common failures

- An unknown pack reports `ERR_CONFIG_PACK_NOT_FOUND` and suggests
  `sc-compose templates list`.
- An existing destination reports `ERR_CONFIG_TEMPLATE_EXISTS`; delete it or
  choose a different pack name.
- A pack with no root-level `*.j2` file, or with more than one, reports
  `ERR_CONFIG_PACK_NOT_RENDERABLE`. Nested templates do not count as the
  pack's render entrypoint.
- Invalid or unreadable `template.json`, invalid variable names, and malformed
  pack configuration report `ERR_CONFIG_PARSE`.
- A missing var-file reports `ERR_CONFIG_READ`; malformed or non-object
  variable data reports `ERR_CONFIG_PARSE` or `ERR_CONFIG_VARFILE`.

Set `SC_COMPOSE_TEMPLATE_DIR` consistently for `add`, `list`, and rendering;
otherwise each invocation may use a different platform user-data location.
