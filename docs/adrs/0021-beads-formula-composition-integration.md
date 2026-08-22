# ADR-0021: Beads Formula Composition Host-Neutral Integration

## Status

Proposed. This ADR must be accepted and its boundary amendments approved before
Phase R implementation source is authored.

## Context

Beads formulas may be authored as `.formula.toml` or `.formula.json` files.
The current Beads parser feeds both formats into one Go `Formula` model;
`.formula.toml` is preferred and JSON is the legacy fallback. Its Pydantic
models are for the separate MCP issue-tracker integration, and `bd schema`
describes issue/dependency JSON output rather than formula input. They are not
formula contracts this repository can reuse.

Formulas need ordinary template composition for repeated static data, while
`bd` remains authoritative for formula parsing, variable semantics, state, and
creation of persistent beads. Reimplementing Beads schema or state rules in
`sc-composer` would duplicate an upstream contract and make divergence likely.

There are three intended callers: `sc-compose bead ...`, Python extensions,
and a future upstream `bd compose ...` command. They need the same
machine-readable request and receipt rather than independently shaped
integrations.

## Decision

### A separate, host-neutral crate owns the integration

Add `crates/sc-composer-beads`. It depends on `sc-composer` for rendering and
uses a direct subprocess invocation of the configured `bd` executable. It
does not link to Beads, parse Beads formula syntax, read or write Beads
databases itself, or depend on the CLI or any foreign-language adapter.

```text
sc-compose bead CLI ─┐
Python adapter ──────┼──> sc-composer-beads ──> sc-composer
future bd compose ───┘             │
                                  └──> bd executable (authoritative)
```

`sc-compose` is a thin CLI adapter over this crate. A separate Maturin/PyO3
adapter is a thin Python adapter over the same crate. A future Beads-side
command can execute the installed CLI with a JSON request and consume the same
receipt; it does not require a Rust reverse dependency.

### Stable request and receipt protocol

The public library, CLI, and Python adapter use the versioned
`sc-compose/beads/v1` contract:

```rust
pub enum BeadOperation { Render, Validate, PreviewPour, Pour }

pub struct BeadComposeRequest {
    pub schema: String, // exactly "sc-compose/beads/v1"
    pub operation: BeadOperation,
    pub working_directory: PathBuf,
    pub template: PathBuf,
    pub rendered_formula: PathBuf,
    pub compose_variables: serde_json::Map<String, serde_json::Value>,
    pub formula_name: Option<String>,
    pub bead_variables: std::collections::BTreeMap<String, String>,
    pub bd_executable: Option<PathBuf>,
    pub pour_authorization: Option<PourAuthorization>,
}

pub struct BeadComposeReceipt {
    pub schema: String, // exactly "sc-compose/beads/v1"
    pub operation: BeadOperation,
    pub rendered_formula: PathBuf,
    pub stages: Vec<BeadStageReceipt>,
    pub outcome: BeadOutcome,
}

pub fn execute_bead_request(
    request: &BeadComposeRequest,
) -> Result<BeadComposeReceipt, BeadComposeError>;
```

All paths resolve relative to `working_directory`; receipts contain normalized
absolute paths. The template path is confined to that directory. `Render` and
`Validate` write only an explicit output within it; `PreviewPour` and `Pour`
are the deliberate exception because their output must be in the active Beads
registry resolved by `bd where`. `compose_variables` remain structured JSON so
authors can use normal Jinja lists and objects. `bead_variables` remain ordered
scalar `key=value` pairs and are supplied only to `bd --var`; this preserves
Beads' own runtime-variable contract.

The only schema interpretation is the request/receipt protocol. `bd` stdout
and stderr are recorded as bounded diagnostic evidence, not reparsed into an
independent Beads schema.

### Rendering and Beads operations

Beads integration fixes sc-compose expression delimiters to triple braces:
`{{{ compose_value }}}`. This preserves Beads runtime placeholders such as
`{{ bead_value }}` in the rendered formula. Normal Jinja control blocks,
including `{% for %}`, remain available for static expansion. There is no new
generic `--mode formula`, list-variable type, or Beads feature inside
`sc-composer`.

Operations are ordered and fail closed:

| Operation | Required work | State effect |
| --- | --- | --- |
| `Render` | Render to the requested formula path. | Writes only that requested output. |
| `Validate` | Render, then invoke `bd cook <file> --dry-run --json` with ordered `--var` inputs. | No Beads database write. |
| `PreviewPour` | Complete `Validate`, resolve the active registry with `bd where --json`, then invoke `bd mol pour <formula-name> --dry-run --json`. | No Beads database write. |
| `Pour` | Complete `Validate`, resolve the active registry with `bd where --json`, then invoke `bd mol pour <formula-name> --json`. | Creates persistent Beads state only with explicit authorization. |

`PreviewPour` and `Pour` require a non-empty `formula_name`. The crate obtains
the authoritative active Beads directory with `bd where --json` and requires
the rendered path to equal `<active-beads-dir>/formulas/<formula-name>.formula.toml`
or `.formula.json`. It rejects a second extension for the same formula name in
that active registry: Beads prefers TOML, so accepting both would make the
requested file ambiguous. The crate does not silently copy a rendered formula
into that directory. This makes the file-writing boundary explicit, avoids
worktree/redirect search-path shadowing, and matches Beads' own resolution.

`Pour` requires the exact typed authorization value
`PourAuthorization::CreatePersistentBeads`; neither CLI nor Python defaults to
it. The runner uses `std::process::Command` arguments, never a shell string or
`sh -c`.

### Version and boundary policy

Phase R verifies the contract against a pinned Beads `v1.2.2` release binary
for Linux, macOS, and Windows, including its published checksum. The local
developer binary is not the CI source of truth. A later Beads upgrade requires
the same real integration tests before the pin changes.

Before source is authored, update `docs/architecture.md`, `CLAUDE.md`, and
the sc-lint boundary inventory so that:

- `sc-composer-beads` may depend only on `sc-composer`, serde/error support,
  and Rust standard-library process/filesystem APIs;
- `sc-compose` may additionally depend on `sc-composer-beads`;
- `bindings/sc-composer-beads-python` may depend only on
  `sc-composer-beads` plus approved PyO3/maturin/serde dependencies; and
- no domain crate depends on a CLI, Python adapter, Beads source package, ATM,
  or a foreign-language adapter.

## Consequences

- Beads remains the validator and state owner, so formula behavior tracks the
  real installed `bd` rather than a partial copied implementation.
- Static Jinja expansion and Beads runtime substitution coexist without brace
  collisions.
- Python and a future `bd compose` consumer share a stable protocol and exact
  receipts, reducing adapter drift.
- `Pour` stays deliberately opt-in and auditable.

## Rejected alternatives

### Add `--mode formula` or Beads-specific variable types to `sc-composer`

Rejected. Existing structured JSON inputs and Jinja loops already compose
static lists. A generic renderer must not acquire Beads parsing, schema, or
state policy.

### Reimplement `bd validate` / pour semantics in Rust

Rejected. The Beads executable, not this repository, defines valid formulas,
runtime substitution, database state, and pour behavior.

### Shell out from Python or have each adapter implement its own process flow

Rejected. It would duplicate authorization, argv construction, failure
classification, and receipts. The Rust crate owns that one integration seam.

### Add `bd compose` in this repository now

Rejected. Phase R provides a host-neutral protocol specifically so Beads can
adopt it later without introducing a reverse dependency or taking ownership of
the Beads CLI.

## References

- [Beads formula parser](https://github.com/gastownhall/beads/blob/main/internal/formula/parser.go)
- [Beads shared `Formula` model](https://github.com/gastownhall/beads/blob/main/internal/formula/types.go)
- [Beads `cook` command](https://github.com/gastownhall/beads/blob/main/cmd/bd/cook.go)
- [Beads `mol pour` command](https://github.com/gastownhall/beads/blob/main/cmd/bd/pour.go)
- [Issue #551](https://github.com/randlee/sc-compose/issues/551)
