# sc-compose resolve

When you need to know which file a named profile will use, reach for
`resolve` before rendering. It finds the concrete template path without
changing or rendering it, making profile search behavior easy to inspect and
safe to use from a script that needs the path for a later operation.

## Basic usage

```text
sc-compose resolve [OPTIONS]
```

Resolve is profile-only. Select profile mode explicitly and provide the
profile identity, for example:

```shell
sc-compose resolve --mode profile --kind agent --agent planner --root .
```

The common profile selectors are `--kind`, `--agent` (also available as
`--agent-type`), and `--runtime` (also available as `--ai`). `--root` confines
the search to a workspace. `--json` emits a diagnostic envelope whose payload
contains `resolved_path`, `search_trace`, and `found`; without it, the command
prints the resolved path and each searched candidate.

## Common failures

- `ERR_CONFIG_MODE` is returned when resolve is requested in file mode; use
  `--mode profile`.
- `ERR_RESOLVE_NOT_FOUND` means no profile candidate matched the requested
  identity. Check `--root`, `--kind`, `--agent`, and `--runtime`.
- `ERR_RESOLVE_AMBIGUOUS` means more than one candidate matched and the
  profile identity needs to be made more specific.
- Malformed configuration or invalid CLI combinations use
  `ERR_CONFIG_PARSE`.

The search trace in JSON mode is useful for diagnosing repository-layout
problems while keeping output safe for scripts.
