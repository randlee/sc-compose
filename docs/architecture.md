# SC-Compose Architecture

> Status: Active Release Baseline
> Product: `sc-composer` (library) and `sc-compose` (CLI)
> Document role: Normative release architecture for both crates

This document supersedes the prior high-level placeholder. It is the normative
release architecture baseline for `sc-compose` v1.0.

## 1. Architectural Intent

This document defines the required architecture of `sc-composer` and
`sc-compose` for release work. It is not a description of the current
implementation.

The goals are:

- one implementation of prompt and template composition semantics,
- deterministic outputs and diagnostics,
- runtime-agnostic library behavior,
- a thin CLI over library APIs,
- clear separation between reusable core logic and integration-specific edges.

## 2. Boundary Diagram

```text
                    outside this repo
   +-----------------------------------------------+
   | ATM adapter / other host integration          |
   | - builds ComposeRequest                       |
   | - calls render_template() or Renderer         |
   | - may inject an observer implementation       |
   +-------------------------+---------------------+
                             |
                             v
                  +-------------------------------+
                  |          sc-compose           |
                  |   CLI / UX / logger wiring    |
                  +-----------+-------------------+
                              | uses concrete logger
                              v
                  +-------------------------------+
                  |       sc-observability        |
                  |   Logger + file/console sinks |
                  +-------------------------------+
                              ^
                              | injects CLI-owned observer adapter
                              |
                  +-----------+-------------------+
                  |         sc-composer           |
                  | core composition API +        |
                  | local observer hook layer     |
                  +-----------+-------------------+
```

ATM-specific integration attaches above the two-crate boundary. `sc-composer`
never imports ATM types, defines its observer hooks locally, and receives
concrete logging behavior through trait injection rather than a direct
dependency on `sc-observability`.

## 3. Crate Layout

### 3.1 `sc-composer`

`sc-composer` is the core library crate. It owns:

- template parsing,
- frontmatter parsing and normalization,
- include expansion,
- variable discovery and validation,
- resolver policy evaluation,
- rendering,
- composition pipeline assembly,
- diagnostics production,
- reusable workspace helpers for initialization tasks.

### 3.2 `sc-compose`

`sc-compose` is the CLI binary crate. It owns:

- argument parsing,
- command dispatch,
- output formatting,
- exit codes,
- file-writing UX,
- CLI-facing observability wiring,
- bundled example-pack discovery,
- user template-pack discovery and storage,
- pack metadata parsing,
- templates add workflows.

### 3.3 Dependency Direction

Required dependency direction:

- `sc-compose` -> `sc-composer`
- `sc-compose` -> `sc-observability`
- `sc-observability` -> `sc-observability-types`

Required observability split:

- `sc-observability` is the concrete logging integration target for the CLI.
- `sc-composer` keeps its observer interfaces local.

Forbidden dependency direction:

- `sc-composer` -> `sc-compose`
- `sc-composer` -> `sc-observability`
- `sc-composer` -> orchestration-specific runtime crates
- `sc-composer` -> mailbox helpers, daemon helpers, team-state helpers, or
  runtime-specific home-resolution helpers

### 3.4 ATM Integration Model

ATM integration is an adapter concern outside this repository.

- An ATM adapter depends on `sc-composer` or `sc-compose`; this repository does
  not depend on ATM crates.
- The adapter constructs `ComposeRequest` values and calls either
  `render_template()` for one-shot usage or the `Renderer` API for
  repeated rendering.
- If ATM needs telemetry, the adapter or CLI injects a sink implementation
  through the library's trait-based observability hooks.
- `sc-composer` never imports ATM types, mailbox abstractions, spool paths, or
  runtime-management helpers.

## 4. Module Architecture

`sc-composer` should be organized around these modules:

- `frontmatter`
  - parses YAML frontmatter,
  - normalizes omitted fields to schema defaults,
  - exposes typed frontmatter structures.
- `resolver`
  - resolves explicit file paths and profile-mode prompt lookup,
  - records search traces,
  - applies resolver policy.
- `include`
  - expands `@<path>` directives,
  - enforces path confinement,
  - tracks include stack,
  - detects cycles and depth overflow.
- `validation` (context merge and token discovery implemented here, not as
  separate `context.rs`/`tokens.rs` files)
  - merges explicit variables, environment variables, and defaults
    in precedence order (explicit > env > frontmatter defaults),
  - tracks variable origin,
  - applies unknown-variable policy,
  - discovers referenced template tokens,
  - distinguishes declared, undeclared, missing, and extra variables.
- `render`
  - configures the template engine,
  - exposes the long-lived `Renderer` session type as the primary API for
    repeated rendering,
  - keeps `render_template()` as a one-shot convenience wrapper,
  - renders template content under normal or strict undeclared-token policy.
- `validate`
  - produces validation reports and diagnostics without writing output.
- `error`
  - defines crate-owned error types and shared recovery-hint structures,
  - maps lower-level failures into stable public categories.
- `diagnostics`
  - defines diagnostic types,
  - defines the JSON diagnostic schema contract.
- `workspace`
  - implements `frontmatter-init` and `init` logic for reuse by the CLI and any
    future embedded callers.
- `observer`
  - defines the local observer and sink traits used by embedded hosts and the
    CLI,
  - owns the no-op observer used when no caller injects a concrete
    implementation,
  - emits structured composition-stage events,
  - never binds directly to `sc-observability`.

## 5. Resolver Path Policy (FR-5)

Resolver policy must be data-driven and not embedded in CLI-only conditionals.

The policy model must express:

- runtime name,
- profile kind,
- ordered candidate directories,
- ordered filename probes,
- ambiguity rules when runtime is omitted.

### 5.1 Runtime-Specific Directories

- `.claude/agents/`
- `.claude/commands/`
- `.claude/skills/`
- `.codex/agents/`
- `.codex/commands/`
- `.codex/skills/`
- `.gemini/agents/`
- `.gemini/commands/`
- `.gemini/skills/`
- `.opencode/agents/`
- `.opencode/commands/`
- `.opencode/skills/`

### 5.2 Shared Directories

- `.agents/agents/`
- `.agents/commands/`
- `.agents/skills/`

There is no flat shared fallback such as `.agents/<name>`.

### 5.3 Probe Rules

For agent and command prompts, candidate probe order within a directory is:

1. `<name>.md.j2`
2. `<name>.md`
3. `<name>.j2`

For skills, candidate probe order within a directory is:

1. `<name>/SKILL.md.j2`
2. `<name>/SKILL.md`
3. `<name>/SKILL.j2`

### 5.4 Ambiguity Rules

- If a runtime is explicitly provided, only that runtime chain is evaluated.
- If a runtime is omitted, all runtime and shared roots are evaluated.
- If multiple candidates match, resolution fails with an ambiguity diagnostic.
- If exactly one candidate matches, it may be selected without an explicit
  runtime.

## 6. Frontmatter Model (FR-1, FR-2)

Frontmatter is a first-class typed structure.

Target frontmatter shape:

```text
Frontmatter {
  required_variables: Vec<String>,
  defaults: Map<String, InputValue>,
  metadata: Map<String, MetadataValue>,
}
```

Normalization rules:

- If frontmatter exists but omits `required_variables`, normalize to `[]`.
- If frontmatter exists but omits `defaults`, normalize to `{}`.
- If frontmatter uses `input_defaults`, normalize it into `defaults`.
- If frontmatter exists but omits `metadata`, normalize to `{}`.
- If no frontmatter exists, the document has no declarations and no defaults.
- If both `defaults` and `input_defaults` appear, merge both maps, let
  `input_defaults` override overlapping keys, and emit
  `WARN_VAL_CONFLICTING_DEFAULT_SECTIONS`.

Semantic rules:

- `required_variables` declares variables that must exist after context merge.
- `defaults` supplies optional values that may satisfy a required variable.
- `metadata` is descriptive only and does not affect render semantics in the
  initial design.
- An empty sequence is a valid `InputValue` and may satisfy a required
  variable.
- When a referenced or required variable is satisfied by a default instead of
  explicit caller input, validation emits `INFO_VAL_DEFAULT_USED`.

`InputValue` for the initial release means one of:

- string
- number
- boolean
- null
- sequence of scalar values

Rust type contract:

- `InputValue` is represented as `serde_json::Value`,
- object values are rejected at parse time,
- nested sequences are rejected at parse time,
- only scalar values and arrays of scalar values may cross the CLI-to-library
  boundary.

Sequence values remain narrow in the initial release:

- sequence members may contain only scalar values,
- nested sequences are not supported,
- mapping values are not supported in render-context inputs.

Planned post-`1.0` extension:

- The follow-on structured-input design is tracked in
  [docs/html-sprint-report-plan.md](html-sprint-report-plan.md).
- That design extends `InputValue` beyond the initial-release boundary so
  templates can consume report-shaped data instead of preflattened strings.
- Planned allowed shapes:
  - scalar,
  - array of scalars,
  - object/map with string keys,
  - array of objects,
  - nested object trees whose leaves are scalars or arrays of scalars.
- Planned continued exclusions:
  - arrays of arrays as a first-class input shape,
  - object trees that themselves contain nested arrays,
  - arbitrary mixed recursive data without explicit validation rules.
- The design motivation is HTML/XHTML report composition where one render input
  needs repeated structured sections such as `sprints` plus nested object
  fields for report metadata and links.

`MetadataValue` may be any YAML value:

- scalar
- sequence
- mapping

Supporting public newtypes:

- `VariableName`
  - validated variable identifier used by `required_variables`, `defaults`,
    diagnostics, and variable-source maps,
  - prevents accidental use of arbitrary strings in the public API.
- `IncludeDepth`
  - non-negative bounded include-depth value used by include policy and errors.
- `ConfiningRoot`
  - canonicalized root path newtype used by path-confinement checks and
    configuration validation.

## 7. Variable and Token Semantics (FR-2)

The architecture must distinguish these cases:

- declared required variable,
- declared optional variable with a default,
- undeclared referenced token,
- extra provided input variable.

### 7.1 Default Mode

In default mode:

- undeclared referenced tokens are preserved in rendered output,
- undeclared referenced tokens produce diagnostics,
- undeclared referenced tokens are not implicitly promoted to required
  variables.

### 7.2 Strict Mode

In strict mode:

- undeclared referenced tokens are fatal during validation,
- undeclared referenced tokens are fatal during rendering.

### 7.3 Missing Required Variables

Missing required variables remain a separate diagnostic class:

- they fail validation and rendering,
- they are reported with file, line and column when available, and include
  chain.

## 8. Public API Shape (FR-6, FR-7)

The library API should expose explicit request and result types.

Required library surface:

- `resolve_profile(request) -> ResolveResult`
- `compose(request) -> ComposeResult`
- `validate(request) -> ValidationReport`
- `init_workspace(root, options) -> InitResult`
- `frontmatter_init(path, options) -> FrontmatterInitResult`
- `Renderer::render(compiled, context) -> Result<String, RenderError>` as the
  primary repeated-render API

Primary render-entrypoint decision:

- `Renderer` is the primary long-lived rendering API because it can retain a
  pre-built `minijinja::Environment` across multiple render operations.
- `render_template()` remains a stable convenience API for one-shot rendering
  and simple callers.
- Callers rendering the same template or environment repeatedly should use
  `Renderer` once implemented rather than paying per-call environment setup and
  AST re-parse cost.

### 8.1 API Ownership Matrix

The rendering and composition surfaces have distinct responsibilities.

| Surface | Owns | Does not own |
| --- | --- | --- |
| `Renderer` | template loading, include resolution, variable expansion, validation, rendering | CLI argument parsing, output formatting, repository bootstrap |
| `compose()` | top-level convenience orchestration; calls `Renderer` internally for end-to-end composition and block assembly | direct CLI UX decisions |
| `render_template()` | lower-level rendering entry point for callers that pre-supply resolved template content | profile resolution, repository scanning, workspace bootstrap |
| `validate()` | validation phase only; returns structured diagnostics without writing output | output generation or file writing |
| `frontmatter_init()` | frontmatter discovery and rewrite helper | template composition pipeline execution |
| `init_workspace()` | repository bootstrap helper | template composition pipeline execution |

### 8.2 Core Request Types

`ComposeRequest`

- `runtime: Option<RuntimeKind>`
- `mode: ComposeMode`
- `root: ConfiningRoot`
- `vars_input: Map<VariableName, InputValue>`
- `vars_env: Map<VariableName, InputValue>`
- `vars_defaults: Map<VariableName, InputValue>`
- `guidance_block: Option<String>`
- `user_prompt: Option<String>`
- `policy: ComposePolicy`

`ComposeMode`

- `Profile { kind: ProfileKind, name: String }`
- `File { template_path: PathBuf }`

Semantics:

- `runtime = None` is valid and enables the omit-runtime search behavior
  defined in the requirements.
- `ComposeMode` is variant-specific and must not be represented as a bag of
  unrelated optional fields.
- In `File` mode, `runtime` may be `None` and is ignored unless a caller wants
  to attach runtime context for logging or policy selection.

`ComposePolicy`

- `strict_undeclared_variables: bool`
- `unknown_variable_policy: UnknownVariablePolicy`
- `max_include_depth: IncludeDepth`
- `allowed_roots: Vec<ConfiningRoot>`
- `resolver_policy: ResolverPolicy`

### 8.3 Core Result Types

`ResolveResult`

- `resolved_path: PathBuf`
- `attempted_paths: Vec<PathBuf>`
- `ambiguity_candidates: Vec<PathBuf>`

`ComposeResult`

- `rendered_text: String`
- `resolved_files: Vec<PathBuf>`
- `resolve_result: ResolveResult`
- `variable_sources: Map<VariableName, VariableSource>`
- `warnings: Vec<Diagnostic>`

`ValidationReport`

- `ok: bool`
- `warnings: Vec<Diagnostic>`
- `errors: Vec<Diagnostic>`
- `resolve_result: ResolveResult`

`ComposeError`

- `Resolve(ResolveError)`
- `Include(IncludeError)`
- `Validation(ValidationError)`
- `Render(RenderError)`
- `Config(ConfigError)`

`FrontmatterInitResult`

- `target_path: PathBuf`
- `frontmatter_text: String`
- `discovered_variables: Vec<VariableName>`
- `changed: bool`
- `would_change: bool`

`InitResult`

- `prompts_dir: PathBuf`
- `gitignore_updated: bool`
- `scanned_templates: Vec<PathBuf>`
- `recommendations: Vec<Diagnostic>`
- `validation_passed: bool`

Entrypoint contract:

- `compose(request) -> Result<ComposeResult, ComposeError>`
- `validate(request) -> Result<ValidationReport, ComposeError>`
- `resolve_profile(request) -> Result<ResolveResult, ComposeError>`
- `Diagnostic` is not a failure type. Diagnostics describe warnings and
  user-actionable validation findings; `ComposeError` describes operation
  failure.

## 9. Include and Frontmatter Merge Rules (FR-3)

The include graph is evaluated deterministically.

Merge behavior:

- required-variable declarations from included files participate in validation
  of the overall composition result,
- defaults from included files participate in context construction,
- parent-file defaults override defaults from included files,
- environment-derived variables override all defaults,
- explicit input variables override environment-derived values and defaults.

Metadata behavior:

- metadata from included files does not affect rendering,
- metadata may be retained in trace structures in a future API, but metadata is
  not part of current render semantics.

## 10. Diagnostics Model (FR-8)

Diagnostics are structured records used by both the library and CLI.

Required fields:

- `code`
- `message`
- `path`
- `line`
- `column`
- `include_chain`
- `severity`

The JSON representation must be versioned. The version belongs to the schema
contract, not to any single CLI command.

Top-level diagnostics envelope (payload fields are command-specific; `"valid"`
shown here matches the `validate` command — see §13.1 for per-command schemas):

```json
{
  "schema_version": "1",
  "payload": {
    "valid": false
  },
  "diagnostics": [
    {
      "severity": "error",
      "code": "ERR_VAL_MISSING_REQUIRED",
      "message": "missing required variable: name",
      "path": "templates/example.md.j2",
      "line": 12,
      "column": 4,
      "include_chain": []
    }
  ]
}
```

Minimal diagnostic record:

```json
{
  "severity": "info",
  "code": "INFO_VAL_DEFAULT_USED",
  "message": "variable name not provided, using default: \"world\"",
  "location": "templates/example.md.j2"
}
```

## 11. Error Model (FR-7, FR-8)

`sc-composer` must expose crate-owned canonical public error types.

Required error structs:

- `ResolveError`
- `IncludeError`
- `ValidationError`
- `RenderError`
- `ConfigError`

Error requirements:

- every canonical error carries an underlying `source()` cause chain when one
  exists,
- include-related errors carry the include chain when applicable,
- configuration and validation failures may carry structured recovery hints,
- recovery hints must remain structured data rather than prose-only strings.

CLI boundary rule:

- `sc-compose` may wrap library errors with `anyhow` or `eyre` at the command
  boundary,
- `sc-composer` public APIs must return the canonical error types defined in
  this document, not `anyhow::Error` or third-party engine error types.

## 12. Request Lifecycle (FR-2, FR-3, FR-6)

For `compose` and `validate`, the target lifecycle is:

1. Resolve explicit path or profile path.
2. Read the root template file.
3. Parse frontmatter and body.
4. Expand includes while enforcing path and depth policy.
5. Merge frontmatter declarations and include-derived declarations.
6. Discover referenced variables from the expanded template graph.
7. Merge context in precedence order:
   - explicit input,
   - environment,
   - defaults.
8. Apply validation policy:
   - missing required variables,
   - undeclared referenced tokens,
   - extra provided variables.
9. Render in normal or strict mode according to policy.
10. Assemble final output blocks.
11. Return composed output or validation report with diagnostics and trace data.

Internal lifecycle encoding:

- the composition pipeline must preserve the documented ordering of resolve,
  parse, include expansion, validation, render, and output assembly,
- internal helpers may use staged data structures to make ordering violations
  difficult to represent,
- the initial release does not expose a public typestate API or a public
  `pipeline` module.

## 13. CLI Command Architecture (FR-6, FR-7)

`sc-compose` should be a command router over library operations.

Command mapping:

- `render` -> `compose`
- `resolve` -> `resolve_profile`
- `validate` -> `validate`
- `frontmatter-init` -> `frontmatter_init`
- `init` -> `init_workspace`
- `observability-health` -> CLI logger initialization, then `Logger::health()`
- `examples list` -> list bundled example packs
- `examples <name>` -> resolve the bundled example-pack file, merge pack
  `input_defaults`, then `compose`
- `templates list` -> list user template packs
- `templates add` -> copy a source file or directory into the user template
  root as one pack
- `templates <name>` -> resolve the user pack entry template, merge pack
  `input_defaults`, then `compose`

The CLI must not reimplement core composition semantics. If a command requires
logic useful to non-CLI callers, that logic belongs in the library.

Command-specific rules:

- `render`
  - accepts `file` mode and `profile` mode,
  - requires `--file <path>` in file mode,
  - accepts optional guidance and user prompt blocks,
  - writes to stdout by default unless an output path is chosen.
- `resolve`
  - is defined for `profile` mode only,
  - fails for `file` mode.
- `validate`
  - uses the same resolver and include graph as `render`,
  - never writes rendered output.
- `frontmatter-init`
  - rewrites or inserts frontmatter for a single target file,
  - uses token discovery but does not render the file.
- `init`
  - performs repository bootstrap and validation-oriented scanning.
- `observability-health`
  - reads logger health state without mutating composition behavior,
  - prints a human-readable health summary by default,
  - emits `LoggingHealthReport` under `--json` as defined in section 19.3.
- `examples list` and `templates list`
  - enumerate entries under their respective roots,
  - surface normalized flat example names or template directory names as pack
    names,
  - emit stable JSON payloads containing `name` and absolute `path`,
  - may append `template.json` `description` and `version` in human-readable
    text output for templates when present.
- `templates add`
  - accepts a single file or directory source,
  - creates one pack directory in the user template root,
  - uses the explicit `[name]` when provided,
  - otherwise uses the source directory name for directory input or the
    normalized template filename for file input,
  - fails if the target pack name already exists,
  - does not merge into an existing pack in the initial release.
- `examples <name>` and `templates <name>`
  - treat the command namespace as the pack root selector,
  - support the same render flags and output semantics as `render`,
  - are defined only when the target pack has exactly one root-level `*.j2`
    file.

Guidance and prompt input model:

- `--guidance <text>` and `--guidance-file <path|->` feed `guidance_block`.
- `--prompt <text>` and `--prompt-file <path|->` feed `user_prompt`.
- The CLI rejects ambiguous attempts to read both blocks from the same stdin
  stream in one invocation.

CLI alias model:

- `--agent-type` is a CLI alias for `--agent`.
- `--ai` is a CLI alias for `--runtime`.
- Aliases are normalized in the CLI before constructing `ComposeRequest`.

### 13.1 Command Output Schemas

The CLI owns final output shaping. Library result types may be richer than the
command-facing JSON contract.

All `--json` command output uses the versioned `DiagnosticEnvelope` transport:

```json
{
  "schema_version": "1",
  "payload": {},
  "diagnostics": []
}
```

The schemas below define the `payload` shape for each command.

`render --json`

```json
{
  "output_path": "stdout",
  "bytes_written": 123,
  "template": "path/to/template.md.j2"
}
```

`render --dry-run --json`

```json
{
  "would_write": ".prompts/example-01HXYZ.md",
  "would_change": true,
  "template": "path/to/template.md.j2",
  "rendered_preview": "preview text"
}
```

`resolve --json`

```json
{
  "resolved_path": ".claude/agents/example.md.j2",
  "search_trace": [
    ".claude/agents/example.md.j2",
    ".agents/agents/example.md.j2"
  ],
  "found": true
}
```

`validate --json`

```json
{
  "valid": false
}
```

`init --json`

```json
{
  "workspace_root": "/repo",
  "created_files": [
    ".prompts/",
    ".gitignore"
  ]
}
```

`init --dry-run --json`

```json
{
  "action": "init",
  "would_affect": [
    ".prompts/",
    ".gitignore"
  ],
  "changed": false,
  "would_change": true,
  "skipped": false
}
```

`examples list --json` and `templates list --json`

```json
{
  "packs": [
    {
      "name": "hello",
      "path": "/path/to/share/sc-compose/examples/hello.md.j2"
    }
  ]
}
```

`templates add --json`

```json
{
  "name": "pytest-fixture",
  "source": "/path/from",
  "destination": "/path/to",
  "changed": true
}
```

Named render through `examples <name>` and `templates <name>` reuses the
`render` and `render --dry-run` payload schemas.

`observability-health --json`

```json
{
  "logging": {
    "state": "Healthy",
    "dropped_events_total": 0,
    "flush_errors_total": 0,
    "active_log_path": "<log_root>/logs/sc-compose.log.jsonl",
    "sink_statuses": [],
    "last_error": null,
    "query": null
  }
}
```

`frontmatter-init --json`

```json
{
  "template_path": "templates/example.md.j2",
  "frontmatter_added": true,
  "would_change": true,
  "vars": [
    "name",
    "role"
  ]
}
```

Non-render `--dry-run --json`

```json
{
  "action": "frontmatter-init",
  "would_affect": [
    "templates/example.md.j2"
  ],
  "changed": false,
  "would_change": true,
  "vars": [
    "name",
    "role"
  ],
  "skipped": false
}
```

Schema notes:

- `search_trace` is the CLI serialization of the library resolver search path
  trace.
- `location` is a single string field in CLI JSON even when the library tracks
  path, line, and column separately.
- `rendered_preview` is the dry-run preview string.
- `payload.logging.query` is `null` when query/follow health is unavailable and
  otherwise contains a `QueryHealthReport`.
- `active_log_path` is derived from the configured log root and service name
  using the `LOG-008` layout `<log_root>/logs/<service>.log.jsonl`.
- The concrete path is platform-dependent; on Windows it may be drive-qualified.

## 14. Output Path Policy (FR-7)

File-writing behavior should be centralized rather than duplicated per command.

Policy requirements:

- file mode strips the final `.j2` suffix,
- profile mode writes to `.prompts/<name>-<ulid>.md`,
- explicit `--output` overrides derived behavior,
- dry-run returns the same derived target information without writing files.

## 15. Template Pack Architecture (FR-1d, FR-2, FR-7)

Template packs are CLI-owned assets. They do not change the core
`sc-composer` composition semantics.

Root resolution:

- bundled examples root:
  1. `SC_COMPOSE_DATA_DIR/examples`
  2. install-relative `../share/sc-compose/examples/`
- user templates root:
  1. `SC_COMPOSE_TEMPLATE_DIR`
  2. platform user-data directory joined with `sc-compose/templates/`

Layout rules:

- examples are flat `*.j2` files stored directly under the bundled examples
  root,
- example names are derived from the filename by removing the trailing `.j2`
  suffix and then one remaining source extension when present,
- normalized example names must remain unique after that derivation step,
- templates are one subdirectory per template under the user templates root,
- template names are directory names,
- template directories may contain one or more files,
- template directories may contain non-template assets retained verbatim when a
  directory source is imported with `templates add`,
- template directories may contain an optional `template.json`.

`TemplateStore`

- `TemplateStore` is a `sc-compose` CLI-layer abstraction and does not exist in
  `sc-composer`,
- it owns discovery, named lookup, and user-template import for one source
  root,
- concrete store roots are selected by `StoreKind::{Examples, Templates}`,
- minimum field shape:
  - `source_dir: PathBuf`
  - `kind: StoreKind`
- `TemplateMeta` carries:
  - `name: String`
  - `path: PathBuf`
  - `description: Option<String>`
  - `version: Option<String>`
- `TemplatePack` carries:
  - `root: PathBuf`
  - `template_path: PathBuf`
  - `input_defaults: Map<VariableName, InputValue>`
- `TemplateAddResult` carries:
  - `name: String`
  - `source: PathBuf`
  - `destination: PathBuf`
  - `changed: bool`
- required methods:
  - `list() -> Result<Vec<TemplateMeta>>`
  - `get_example(name: &str) -> Result<Option<TemplatePack>>`
  - `get_template(name: &str) -> Result<Option<TemplatePack>, GetTemplateError>`
  - `add(source: &Path, requested_name: Option<&str>) -> Result<TemplateAddResult, AddError>`
- examples and templates use the same abstraction with different layout rules:
  - examples list and named lookup operate on flat `*.j2` files,
  - templates list and named lookup operate on subdirectories and resolve the
    single root-level `*.j2` entry file when renderable,
  - `AddError::AlreadyExists` is the structured duplicate-import path,
  - `GetTemplateError::NotRenderable` is reserved for zero-or-many root-level
    `*.j2` files,
  - `GetTemplateError::Parse` covers manifest and filesystem read failures.

Command extraction:

- `src/commands/examples.rs` owns `run_examples_list` and
  `run_examples_render`,
- `src/commands/templates.rs` owns `run_templates_list`, `run_templates_add`,
  and `run_templates_render`,
- `main.rs` retains the top-level CLI shape and dispatch only.

`template.json` is intentionally narrow and user-facing:

```json
{
  "description": "Minimal greeting example",
  "version": "1.0.0",
  "input_defaults": {
    "name": "world"
  }
}
```

Manifest rules:

- `description` is for list and help output,
- `version` is pack metadata only,
- `input_defaults` contributes pack-level default inputs,
- user-template input defaults merge with request inputs using the precedence
  defined in the requirements:
  1. explicit input variables
  2. environment-derived variables
  3. `template.json` `input_defaults`
  4. frontmatter defaults
- `input_defaults` values use the same `InputValue` contract as other caller
  inputs:
  - scalars,
  - arrays of scalars,
  - empty arrays are valid,
  - objects and nested arrays are rejected
- no manifest field selects entrypoints, paths, hooks, or alternate execution
  behavior in the initial release.

Implicit named render convention:

- `examples <name>` resolves the flat example file with matching stem,
- `templates <name>` resolves the single root-level `*.j2` file in the named
  template directory,
- if a template directory contains zero or multiple root-level `*.j2` files,
  it remains listable but is not implicitly renderable by name,
- supporting assets remain available for directory-import workflows and future
  expansion, but they do not change the initial render resolution rules.

## 16. `init` Command Behavior (FR-7)

`init_workspace` and the CLI `init` command must:

- create `.prompts/` if needed,
- ensure `.prompts/` is ignored by Git,
- scan repository templates,
- validate discovered templates,
- return recommendations for missing or weak frontmatter,
- fail when invalid templates are found.

This keeps the repository bootstrap step useful as an early correctness check,
not just as directory creation.

Variable-file behavior:

- `--var-file` loads a JSON or YAML object,
- keys are strings,
- values are `InputValue`,
- sequence values may contain only scalar values,
- nested sequences and objects are invalid in the initial release.

## 17. Safety Model (FR-4)

- Default deny for out-of-root file access
- No shell execution inside the composition pipeline
- No evaluation of arbitrary host code from templates
- No hook execution from template packs or `template.json`
- Include stack tracked for all include-related diagnostics
- Deterministic failure semantics for path escape, missing include, cycle, and
  depth overflow

Follow-on boundary note:

- Browser-open/post-render behavior for HTML report workflows remains outside
  `sc-compose` itself and belongs in wrapper tooling such as the
  `/sprint-report` skill.
- The follow-on HTML report plan intentionally separates structured-input
  support in `sc-compose` from workflow orchestration around multiple render
  calls.

## 18. Error and Exit Semantics (FR-7, FR-8)

The library should expose typed errors with stable categories. Validation
results should remain structured and not collapse into string-only errors.

Target CLI exit semantics:

- `0` success
- `2` validation or render failure
- `3` usage, configuration, or contract error

### 18.1 Failure Mode Matrix

Canonical failures must map to stable error families and stable codes.

| Failure condition | Error type | Stable code |
| --- | --- | --- |
| Template not found | `ResolveError` | `ERR_RESOLVE_NOT_FOUND` |
| Ambiguous template match | `ResolveError` | `ERR_RESOLVE_AMBIGUOUS` |
| Include target not found | `IncludeError` | `ERR_INCLUDE_NOT_FOUND` |
| Include path escapes confinement root | `IncludeError` | `ERR_INCLUDE_ESCAPE` |
| Include cycle detected | `IncludeError` | `ERR_INCLUDE_CYCLE` |
| Include depth exceeds limit | `IncludeError` | `ERR_INCLUDE_DEPTH` |
| Variable type mismatch or invalid render-context value | `ValidationError` | `ERR_VAL_TYPE` |
| Duplicate frontmatter variable | `ValidationError` | `ERR_VAL_DUPLICATE` |
| Empty template body | `ValidationError` | `ERR_VAL_EMPTY` |
| Root template has no frontmatter block | `ValidationError` | `ERR_VAL_MISSING_FRONTMATTER` |
| Required variable not satisfied after context merge | `ValidationError` | `ERR_VAL_MISSING_REQUIRED` |
| Undeclared referenced token in strict validation or render mode | `ValidationError` | `ERR_VAL_UNDECLARED_TOKEN` |
| Extra provided variable when policy is `error` | `ValidationError` | `ERR_VAL_EXTRA_INPUT` |
| Stdin read attempted twice | `RenderError` | `ERR_RENDER_STDIN_DOUBLE_READ` |
| Output write failure | `RenderError` | `ERR_RENDER_WRITE` |
| Frontmatter rewrite refused on read-only target | `ConfigError` | `ERR_CONFIG_READONLY` |
| Command or helper invoked in incompatible mode | `ConfigError` | `ERR_CONFIG_MODE` |
| Config file missing or malformed | `ConfigError` | `ERR_CONFIG_PARSE` |
| Invalid var-file shape | `ConfigError` | `ERR_CONFIG_VARFILE` |
| Malformed object from structured input source | `ValidationError` | `ERR_VAL_OBJECT_SHAPE` |
| Nested required path expects an object but receives a scalar, or vice versa | `ValidationError` | `ERR_VAL_SHAPE_MISMATCH` |
| Nested required field absent inside a present object or array member | `ValidationError` | `ERR_VAL_MISSING_NESTED_FIELD` |
| Nested array supplied where H1/H2 only allow objects and arrays of scalars or objects | `ValidationError` | `ERR_VAL_NESTED_ARRAY_UNSUPPORTED` |
| Example or template pack name not found | `ConfigError` | `ERR_CONFIG_PACK_NOT_FOUND` |
| Named pack is not renderable because a bundled example name is ambiguous or a template pack has zero or multiple root-level `*.j2` files | `ConfigError` | `ERR_CONFIG_PACK_NOT_RENDERABLE` |
| `templates add` target name already exists | `ConfigError` | `ERR_CONFIG_TEMPLATE_EXISTS` |

## 19. Observability Integration (FR-9, FR-10, FR-11)

Architecture rules:

- `sc-composer` emits composition telemetry through its local
  `sc_composer::observer` hook layer.
- `sc-compose` provides the canonical concrete binding to the full
  `sc-observability` `Logger`.
- The initial release scope is logging-only:
  - structured log events
  - logger health reporting
  - graceful shutdown
  - downstream extension through the local observer hook model
- If no observer is provided, library and CLI behavior degrade to a no-op
  observability path rather than failing.
- Library observability hooks must remain usable by embedded consumers.
- Default sink paths for standalone CLI behavior must be tool-scoped.
- Observer and sink traits must be object-safe and `dyn`-compatible.
- Observer and sink adapters are intentionally public and unsealed so embedded
  hosts can
  provide their own implementations.
- `sc-observe` and `sc-observability-otlp` are not part of this initial
  release architecture.

### 19.1 Dependency Graph

The observability dependency chain is intentionally split so the library stays
runtime-agnostic:

```text
sc-compose -----> sc-composer
     |
     v
sc-observability -----> sc-observability-types
```

- `sc-composer` defines its own `ObservationEvent`, `ObservationSink`, and
  `CompositionObserver` hook types locally.
- `sc-observability` depends on `sc-observability-types` and owns `Logger`,
  `LogSink`, file sinks, console sinks, `LoggingHealthReport`,
  `QueryHealthReport`, and `QueryHealthState` through its public re-export
  surface.
- `sc-compose` depends on both `sc-composer` and `sc-observability`.

### 19.2 Library Injection Pattern

`sc-composer` exposes a caller-provided observer/sink injection path through
its local `observer` module:

```rust
use sc_composer::observer::{
    CompositionObserver, ObservationEvent, ObservationSink,
};

pub enum ObservationEvent {
    ResolveAttempt(ResolveAttemptEvent),
    ResolveOutcome(ResolveOutcomeEvent),
    IncludeExpandOutcome(IncludeOutcomeEvent),
    ValidationOutcome(ValidationOutcomeEvent),
    RenderOutcome(RenderOutcomeEvent),
}

pub trait ObservationSink {
    fn emit(&mut self, event: &ObservationEvent);
}

pub trait CompositionObserver {
    fn on_resolve_attempt(&mut self, event: &ResolveAttemptEvent) {}
    fn on_resolve_outcome(&mut self, event: &ResolveOutcomeEvent) {}
    fn on_include_outcome(&mut self, event: &IncludeOutcomeEvent) {}
    fn on_validation_outcome(&mut self, event: &ValidationOutcomeEvent) {}
    fn on_render_outcome(&mut self, event: &RenderOutcomeEvent) {}
}

pub fn compose(request: &ComposeRequest) -> Result<ComposeResult, ComposeError>;
pub fn compose_with_observer(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
) -> Result<ComposeResult, ComposeError>;
```

Required library behavior:

- `Renderer::new(...)` and `compose()` install the built-in no-op observer
  unless a caller supplies an explicit observer.
- `compose_with_observer(...)` is the public end-to-end observability
  injection entry point.
- `ObservationSink` and `CompositionObserver` remain the local extension points
  for embedded hosts that do not opt into the CLI.
- `ObservationSink::emit()` is the host-facing single-event adapter surface.
  Internal composition code emits through the typed `CompositionObserver`
  callbacks rather than routing through `emit()`.
- The approved minimum library-owned variant set is:
  - `ResolveAttempt`
  - `ResolveOutcome`
  - `IncludeExpandOutcome`
  - `ValidationOutcome`
  - `RenderOutcome`
- The observer surface remains object-safe and callable through
  `&mut dyn CompositionObserver`.
- Command lifecycle events remain CLI-owned and must not be defined in
  `sc-composer`.

### 19.3 CLI Wiring

`sc-compose` constructs `sc-observability::Logger` during CLI startup, wraps it
in a CLI-owned adapter that implements `sc_composer::observer::ObservationSink`
or `sc_composer::observer::CompositionObserver`, then passes that adapter into
`compose_with_observer(...)`.

CLI wiring rules:

- normal terminal execution enables both file and console sinks,
- `--json` execution disables the console sink so command stdout remains valid
  machine-readable output,
- command lifecycle logging remains CLI-owned and emits:
  - command start
  - command completion
  - command failure
- `observability-health` initializes the logger using the same configuration
  path as a normal CLI process, reads `Logger::health()`, prints a
  human-readable summary by default, and serializes the returned
  `LoggingHealthReport` under `--json`,
- `observability-health` reports process-local logger state only and does not
  depend on any daemon or background runtime,
- CLI shutdown calls the logger's `shutdown()` path so registered sinks flush
  before process exit.

### 19.4 Public API Paths

The normative public API paths for this design are:

- `sc_composer::compose`
- `sc_composer::compose_with_observer`
- `sc_composer::Renderer`
- `sc_composer::observer::ObservationEvent`
- `sc_composer::observer::CompositionObserver`
- `sc_composer::observer::ObservationSink`

### 19.5 Event Shape and Emission Points

The composition pipeline emits `ObservationEvent` values through the local
observer hook layer. The CLI adapter maps those events into concrete logger
records with stable `target`, `action`, and `message` fields that describe:

Message rules:

- `message` is a short human-readable summary of the event outcome.
- Structured fields, not `message`, carry schema-relevant details.
- `message` wording must remain stable enough for operator-facing logs and test
  assertions.

The CLI also emits command lifecycle events with stable `target`, `action`,
and `message` fields for:

- command start,
- command completion,
- command failure.

The adapter-owned mapping is:

| `sc-compose` event source | `LogEvent.target` | `LogEvent.action` | `LogEvent.message` | Other `LogEvent` fields |
| --- | --- | --- | --- | --- |
| command start | `compose.command` | `started` | human-readable summary such as `render started` | `fields` include command name and relevant mode flags |
| command end, success | `compose.command` | `completed` | human-readable summary such as `render completed` | `fields` include command name, elapsed time, and output mode; `outcome` is success |
| command end, failure | `compose.command` | `failed` | human-readable summary such as `render failed` | `fields` include command name, exit code, elapsed time, and output mode; `outcome` is failure; `diagnostic` is attached when available |
| resolve attempt or outcome | `compose.resolve` | phase-specific action such as `attempt`, `resolved`, or `failed` | concise resolver summary sentence | `outcome` reflects success/failure; `diagnostic` is attached for failures; resolver traces or selected paths live in `fields` |
| include-expand outcome | `compose.include_expand` | phase-specific action such as `expanded` or `failed` | concise include-expansion summary sentence | include stack and path details live in `fields`; failures attach `diagnostic` |
| validation outcome | `compose.validate` | phase-specific action such as `completed` or `failed` | concise validation summary sentence | validation counts and policy decisions live in `fields`; failures attach `diagnostic` |
| render outcome | `compose.render` | phase-specific action such as `completed` or `failed` | concise render summary sentence | render metadata lives in `fields`; `outcome` and `diagnostic` reflect success/failure |

This mapping is intentionally adapter-owned so `sc-observability` preserves a
generic logging contract and command lifecycle events remain CLI-owned.

## 20. Extensibility

The release architecture keeps room for future extensions without destabilizing
the core behavior.

Expected extension points:

- typed variable schemas,
- remote include providers,
- template caching,
- custom resolver policies,
- richer frontmatter metadata consumers,
- template-pack lifecycle commands beyond `add`,
- named render for multi-template packs.

Trait openness decisions:

- sink traits are open extension points for embedded hosts,
- `ResolverPolicy` is open because caller-specific path policy is an explicit
  product requirement,
- value-model and metadata extension points remain narrow by design:
  scalar values plus simple sequences are open in the initial release, but
  hooks, arbitrary manifest-driven behavior, and nested mappings remain
  deferred.

## 21. Post-`1.0` Structured Input And HTML Report Architecture

This section is a forward design track only. It does not redefine the shipped
`1.0` implementation.

### 21.1 Input Model Expansion

The follow-on structured-input track expands `InputValue` to support:

- object/map values with string keys,
- arrays of objects,
- nested object trees needed for report composition,
- repeated report sections such as `sprints`.

Nested arrays remain out of scope for H1 and H2. Examples such as
`sprints[].checks[]` are illustrative prose for later design space, not H1/H2
input grammar.

### 21.2 Variable Resolver Behavior

Resolver and merge behavior for structured inputs must remain consistent with
the existing precedence model:

1. explicit input variables
2. environment-derived variables
3. `template.json` `input_defaults`
4. frontmatter defaults

Additional structured-input rules:

- `VariableName` remains a top-level key only. Discovery of `{{ pr.number }}`
  yields `pr`, not `pr.number`.
- Required nested references use a separate `VariablePath` concept rather than
  reusing `VariableName`.
- `VariablePath` grammar is dotted segments of alphanumeric, underscore, and
  hyphen characters such as `pr.number` or `report.plan_url`.
- H1/H2 `VariablePath` does not support bracket notation. Any prose examples
  using `[]` describe future shape, not the H1/H2 path grammar.
- Nested required-variable satisfaction walks the `InputValue` tree by path
  segment.
- The traversal semantics are:
  - missing top-level key -> `ERR_VAL_MISSING_REQUIRED`
  - missing nested segment inside a present object -> `ERR_VAL_MISSING_NESTED_FIELD`
  - scalar where an object is required for the next segment -> `ERR_VAL_SHAPE_MISMATCH`
- Structured variable defaults are replaced, not deep-merged, at the top-level
  variable boundary. When an explicit input and a frontmatter default both
  provide the same top-level variable key, the explicit input replaces the
  entire default value.
- Extra-variable policy from FR-2b applies at the top-level variable boundary
  only. Fields inside a provided object that the template never accesses are
  always accepted and do not trigger extra-input diagnostics.
- Extra-input detection therefore operates on discovered top-level keys such as
  `pr` and `sprints`, not on nested paths such as `pr.number`.

### 21.3 Structured Input Sources

`--var-file`

- remains the primary structured-input ingress,
- parses JSON or YAML objects,
- carries object values and arrays of objects in this phase.

`--var key=value`

- remains string-only,
- does not gain ad hoc object parsing or dotted-key assembly in this phase.

Frontmatter defaults

- gain structured-value support in H1 using the same `InputValue` type and the
  same validation gate as `--var-file`,
- accept objects and arrays of scalars after H1,
- extend to arrays of objects in H2.

`template.json` `input_defaults`

- gain structured-value support under the same rules as frontmatter defaults:
  objects and arrays of scalars in H1, arrays of objects in H2.

### 21.4 Validation Impact

`validate`

- must report missing nested field paths,
- must reject malformed objects and unsupported nesting with stable diagnostics,
- may validate field presence and supported shape without growing into a full
  schema language.

### 21.5 Frontmatter And Token Discovery Impact

`required_variables`

- remains the declaration surface for required inputs,
- must support nested field paths such as `pr.number` and `report.plan_url`.

`frontmatter-init`

- must discover nested references such as `{{ pr.number }}`,
- must discover loop-body references such as
  `{% for sprint in sprints %}{{ sprint.id }}{% endfor %}`,
- must attribute `{{ sprint.id }}` inside that loop to the array variable
  `sprints`, not to `sprint` or `sprint.id`,
- requires scope-aware token scanning. A regex identifier sweep without scope
  tracking cannot distinguish loop-bound names from context variables,
- begins H2 with a spike comparing MiniJinja AST access against a hand-rolled
  `for`/`endfor` scope tracker. The chosen approach must be documented in this
  section before the remaining H2 work proceeds,
- must emit understandable generated field paths instead of opaque flattened
  names.

### 21.6 HTML Report Track Boundaries

The HTML sprint-report track uses the structured-input expansion for a bundled
`sprint-report-html` example and later wrapper integration.

Architectural boundaries:

- `sc-compose` owns rendering,
- the example/template pack owns the HTML structure,
- H3 keeps the bundled example as a single flat file
  `examples/sprint-report-html.html.j2`,
- directory-based example layout is deferred to H4 or a later architecture
  amendment,
- `sc-compose` does not enable MiniJinja auto-escaping for `.html.j2`
  templates; the bundled example documentation must call this out explicitly,
- wrapper tooling such as `/sprint-report` owns open/display behavior,
- no hook execution is added to `sc-compose` for this phase.
