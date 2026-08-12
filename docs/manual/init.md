# sc-compose init

`init` bootstraps a workspace for composed outputs. It creates a `.prompts/`
directory when needed, ensures `.gitignore` contains `.prompts/`, and scans
the workspace's `.j2` files for validation recommendations. The scan skips
`.git` and `target` directories. Existing template files are not rewritten.

## Usage

```text
sc-compose init [--root <ROOT>] [--dry-run] [--json]
```

`--root` selects the workspace and defaults to `.`. `--dry-run` reports the
planned filesystem changes and still validates the discovered templates
without creating `.prompts/` or editing `.gitignore`. `--json` emits a
machine-readable envelope: a dry run includes `action`, `would_affect`,
`would_change`, and `skipped`; a write includes `workspace_root` and
`created_files`.

## Examples

Preview a new workspace before changing it:

```console
$ sc-compose init --root ./project --dry-run
would_affect: ./project/.prompts
would_affect: ./project/.gitignore
```

Initialize it after reviewing the plan:

```console
$ sc-compose init --root ./project
workspace_root: /path/to/project
```

The command's recommendations also identify validation issues and missing
frontmatter in scanned templates. The initialization itself is safe to run
with `--dry-run` first when a repository contains many templates.

## Common failures

- A root that cannot be canonicalized, or a directory that cannot be scanned,
  reports `ERR_CONFIG_PARSE`. Check that `--root` names an accessible
  directory.
- An unreadable existing `.gitignore` reports `ERR_CONFIG_READ`.
- If `.prompts/` and the `.gitignore` entry already exist, a write-mode rerun
  reports `ERR_CONFIG_READONLY`; use `--dry-run` to inspect the no-op state.
- A scanned template can make validation fail with its own diagnostic code.
  In that case the command returns the validation/render failure status (`2`)
  and leaves the workspace changes in place; fix the reported template and
  rerun validation.

`init` does not install example or user template packs. Those are managed by
the `examples` and `templates` commands.
