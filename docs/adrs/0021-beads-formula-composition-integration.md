# ADR-0021: Beads Formula Composition Host-Neutral Integration

## Status

Accepted (2026-08-24, by Rand Lee). Phase R implementation source may not be
authored until the remaining pre-source gate items (CLAUDE.md/architecture.md
boundary amendment, sc-lint negative boundary fixture) are separately
satisfied per sprint R.1's pre-source gate.

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

pub fn parse_request(input: &str) -> Result<BeadComposeRequest, BeadComposeError>;
```

The receipt and error surfaces are fixed for `sc-compose/beads/v1`; adapters
must not invent narrower or differently named variants. The normative Rust
shapes are:

```rust
pub enum BeadStage {
    Render,
    Validate,
    ResolveActiveRegistry,
    PreviewPour,
    Pour,
}

pub enum BeadStageOutcome {
    Succeeded,
    Skipped,
    Failed { code: String },
}

pub struct BeadStageReceipt {
    pub stage: BeadStage,
    pub argv: Vec<String>,
    pub exit_status: Option<i32>,
    pub elapsed_ms: u64,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
    pub outcome: BeadStageOutcome,
}

pub enum BeadOutcome {
    Succeeded,
    Refused { code: String },
    Failed { code: String },
}

pub enum BeadComposeError {
    RequestDeserializationFailed { message: String },
    UnknownSchema { actual: String },
    FormulaPathNotFile { path: PathBuf },
    FormulaExtensionUnsupported { path: PathBuf },
    TemplatePathInvalid { path: PathBuf },
    TemplateOutsideWorkingDirectory { path: PathBuf },
    OutputOutsideWorkingDirectory { path: PathBuf },
    BeadVariableKeyInvalid { key: String },
    BeadVariableKeyDuplicate { key: String },
    FormulaNameRequired,
    PourAuthorizationRequired,
    PourAuthorizationInvalid,
    BdUnavailable { executable: PathBuf },
    RenderFailed { message: String },
    CookFailed { exit_status: Option<i32> },
    ActiveRegistryResolutionFailed { exit_status: Option<i32> },
    FormulaOutsideActiveRegistry { path: PathBuf },
    FormulaRegistryAmbiguous { formula_name: String },
    PreviewPourFailed { exit_status: Option<i32> },
    PourFailed { exit_status: Option<i32> },
}
```

`BeadStageReceipt` records every attempted process stage and bounded output
evidence. Validation failures that occur before spawning `bd` return
`Err(BeadComposeError)` without a process-stage receipt; adapters expose the
corresponding refused error code in their diagnostic envelope.
`BeadComposeError::code()` returns exactly one of these stable machine-readable
codes:

| Error variant | Stable code | Rejection or failure condition |
| --- | --- | --- |
| `RequestDeserializationFailed` | `BEADS_REQUEST_DESERIALIZATION_FAILED` | Request JSON cannot deserialize into the v1 contract. |
| `UnknownSchema` | `BEADS_UNKNOWN_SCHEMA` | Request schema is not `sc-compose/beads/v1`. |
| `FormulaPathNotFile` | `BEADS_FORMULA_NOT_FILE` | Template or rendered formula path is not a regular file. |
| `FormulaExtensionUnsupported` | `BEADS_FORMULA_EXTENSION_UNSUPPORTED` | Formula is not `.formula.toml` or `.formula.json`. |
| `TemplatePathInvalid` | `BEADS_TEMPLATE_PATH_INVALID` | Template path is missing, malformed, or cannot be resolved. |
| `TemplateOutsideWorkingDirectory` | `BEADS_TEMPLATE_OUTSIDE_WORKING_DIR` | Template escapes `working_directory`. |
| `OutputOutsideWorkingDirectory` | `BEADS_OUTPUT_OUTSIDE_WORKING_DIR` | Rendered output escapes the permitted working directory. |
| `BeadVariableKeyInvalid` | `BEADS_VARIABLE_KEY_INVALID` | A Beads runtime-variable key is empty or malformed. |
| `BeadVariableKeyDuplicate` | `BEADS_VARIABLE_KEY_DUPLICATE` | `parse_request` detects that a Beads runtime-variable key is supplied more than once. |
| `FormulaNameRequired` | `BEADS_FORMULA_NAME_REQUIRED` | Preview or pour has no formula name. |
| `PourAuthorizationRequired` | `BEADS_POUR_AUTH_REQUIRED` | Persistent pour lacks the explicit authorization sentinel. |
| `PourAuthorizationInvalid` | `BEADS_POUR_AUTH_INVALID` | Authorization is present but is not `CreatePersistentBeads`. |
| `BdUnavailable` | `BEADS_BD_UNAVAILABLE` | The configured `bd` executable cannot be started. |
| `RenderFailed` | `BEADS_RENDER_FAILED` | Formula rendering failed before `bd` validation. |
| `CookFailed` | `BEADS_COOK_FAILED` | `bd cook --dry-run` failed. |
| `ActiveRegistryResolutionFailed` | `BEADS_WHERE_FAILED` | `bd where --json` failed or returned unusable registry data. |
| `FormulaOutsideActiveRegistry` | `BEADS_FORMULA_OUTSIDE_ACTIVE_REGISTRY` | Formula path is not the active registry path for its name and extension. |
| `FormulaRegistryAmbiguous` | `BEADS_FORMULA_REGISTRY_AMBIGUOUS` | Same-name TOML and JSON formulas coexist in the active registry. |
| `PreviewPourFailed` | `BEADS_PREVIEW_POUR_FAILED` | `bd mol pour --dry-run` failed. |
| `PourFailed` | `BEADS_POUR_FAILED` | Authorized persistent `bd mol pour` failed. |

The `BeadComposeError` variants, their stable codes, `BeadStageReceipt`, and
`BeadOutcome` are the single definition consumed by R.1, R.2, and R.3. A
surface may add presentation fields around this contract but may not rename,
split, or silently collapse these conditions.

### Cross-surface fixture ownership

The canonical cross-surface fixture source is
`crates/sc-composer-beads/tests/fixtures/beads/`. It contains the request and
formula inputs used by the R.1 library, R.2 CLI, and R.3 Python contract tests.
R.1 owns creation and updates whenever the versioned request, receipt, error,
or formula contract changes; R.2 and R.3 load the canonical files directly (or
through a documented, deterministic generation step) and must not maintain
copies.

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
