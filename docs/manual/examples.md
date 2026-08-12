# sc-compose examples

Reach for `examples` when you want to explore a known-good template, learn the
rendering workflow, or produce a bundled example without assembling a pack
yourself. It lists and renders the example packs shipped with sc-compose. A
pack is selected by name; the bundled data directory can be overridden with
`SC_COMPOSE_DATA_DIR` when testing a checkout or a custom installation.

## Usage

List available packs:

```text
sc-compose examples list [--json]
```

Render one pack:

```text
sc-compose examples <NAME> [--var <KEY=VALUE>]... [--var-file <FILE>]...
    [--env-prefix <PREFIX>] [--strict]
    [--unknown-var-mode <error|warn|ignore>]
    [--output <PATH>] [--guidance <TEXT>] [--guidance-file <FILE>]
    [--prompt <TEXT>] [--prompt-file <FILE>] [--json] [--dry-run]
```

`--var` and `--var-file` provide inputs; a var-file must contain a JSON or
YAML object. `--env-prefix` imports matching environment variables, while
`--strict` and `--unknown-var-mode` control undeclared and extra-variable
diagnostics. Rendered content goes to stdout unless `--output` is supplied.
`--guidance` and `--prompt` append blocks to the rendered body, and their
`*-file` forms read those blocks from a file or stdin. `--dry-run` reports the
derived output target without writing it. `--json` selects the structured
output envelope.

## Examples

List packs and render the bundled `hello` example:

```console
$ sc-compose examples list
hello
$ sc-compose examples hello --var name=Casey
Hello Casey!
```

Use a JSON var-file for an example with structured inputs:

```console
$ sc-compose examples pytest-fixture --var-file ./vars.json --output ./fixture.py
```

When running from a checkout, point discovery at the repository data root:

```console
$ SC_COMPOSE_DATA_DIR="$PWD" sc-compose examples list
```

## Common failures

- An unknown pack reports `ERR_CONFIG_PACK_NOT_FOUND` and suggests running
  `sc-compose examples list`.
- A missing or unusable bundled data root reports `ERR_CONFIG_PARSE`; inspect
  or set `SC_COMPOSE_DATA_DIR`.
- Ambiguous or non-renderable example files report
  `ERR_CONFIG_PACK_NOT_RENDERABLE`. A pack must resolve to one usable
  root-level `.j2` example.
- A missing or unreadable var-file reports `ERR_CONFIG_READ`; malformed
  content reports `ERR_CONFIG_PARSE` or a var-file-specific diagnostic such as
  `ERR_CONFIG_VARFILE`. Render and validation failures retain their own
  diagnostic code and status.
