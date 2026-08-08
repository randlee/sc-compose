# sc-lint Reporting Contract

L.2 owns the narrow integration boundary between `sc-compose` and the released
`sc-lint 0.4.0` CLI. The analyzer remains the source of truth for rules and
findings; sc-compose only invokes an allowlisted command, captures its machine
envelope, and materializes a reviewable report.

## Invocation

The runner always invokes the installed tool as:

```text
sc-lint --json --root <repo-root> <allowlisted command>
```

The allowlist is the closed `ScLintCommand` enum in
`crates/sc-compose/src/commands/sc_lint.rs`. `reports/inputs/lint/targets.toml`
is the declarative inventory for target sprints and review tooling. A target
descriptor never becomes an arbitrary subprocess argument.

## Result shape

`sc-lint`'s complete JSON envelope is preserved under `raw_payload`. The
sc-compose result adds:

| Field | Meaning |
| --- | --- |
| `command_id` | Stable dotted ID, such as `lint.sc-boundary` |
| `outcome` | Closed enum: `pass`, `findings`, `config_error`, `capability_error`, or `failed` |
| `exit_status` | The subprocess status, unchanged |
| `stdout` / `stderr` | Captured separately; neither is scraped for control flow |
| `diagnostics` | The upstream envelope diagnostics |
| `findings` / `findings_count` | Generic `data.findings` projection |
| `raw_artifact` | Relative link to the exact stdout payload |
| `report` | Relative link to the HTML summary |

`CLI.CONFIG_ERROR` means repository/tool configuration is invalid. A
`CLI.CAPABILITY_ERROR` means configuration is valid but the host lacks a
required capability. Unknown non-zero results remain `failed`; they are never
silently converted to a pass.

## Artifacts and Just contract

Every invocation writes:

```text
reports/latest/sc-lint/index.html
reports/latest/sc-lint/raw/<command-id>.json
```

The canonical recipes are intentionally thin and identical across consumers:

```text
just lint                 # lint.full
just lint fast|full|ci
just lint sc-boundary|sc-portability|sc-runtime|line-counts|identity-literals
just view findings
just check native|xwin
just clippy native|xwin
just ci                   # lint.ci
```

No recipe invokes a target-specific Python converter. Python-backed utility
ownership stays in sc-lint until the packaging/maturin work tracked by sc-lint
#83 is complete.
