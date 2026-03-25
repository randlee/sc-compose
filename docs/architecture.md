# SC-Compose Architecture

> Status: Draft
> Product: `sc-composer` (library) and `sc-compose` (CLI)
> Document role: Normative target architecture for the redesign of both crates

This document supersedes the prior high-level placeholder. It is the normative
architecture baseline for `sc-compose` v0.x.

## 1. Architectural Intent

This document describes the intended architecture of the redesigned
`sc-composer` and `sc-compose` crates. It is not a description of the current
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
   | - injects observer implementation if needed   |
   +-------------------------+---------------------+
                             |
                             v
                  +-----------------------+
                  |      sc-compose       |
                  | CLI / UX / exit codes |
                  +-----------+-----------+
                              |
                              v
                  +-----------------------+
                  |     sc-composer       |
                  | core composition API  |
                  | observer traits only  |
                  +-----------+-----------+
                              ^
                              |
                  +-----------+-----------+
                  |   sc-observability    |
                  | concrete sink/binding |
                  | injected via traits   |
                  +-----------------------+
```

ATM-specific integration attaches above the two-crate boundary. `sc-composer`
never imports ATM types, and `sc-observability` integration occurs through
trait injection rather than a direct library dependency.

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
- CLI-facing observability wiring.

### 3.3 Dependency Direction

Required dependency direction:

- `sc-compose` -> `sc-composer`
- `sc-compose` -> `sc-observability` as the intended concrete observability
  binding during implementation

Design-ahead note:

- `sc-observability` is the intended observability integration target for the
  CLI architecture, even if the dependency is not yet present in `Cargo.toml`
  at the time this document is written.

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
  `render_template()` for one-shot usage or the planned `Renderer` API for
  repeated rendering.
- If ATM needs telemetry, the adapter or CLI injects an observer implementation
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
- `context`
  - merges explicit variables, environment variables, and defaults,
  - tracks variable origin,
  - applies unknown-variable policy.
- `tokens`
  - discovers referenced template tokens,
  - distinguishes declared, undeclared, missing, and extra variables.
- `render`
  - configures the template engine,
  - exposes the planned long-lived `Renderer` session type as the primary API
    for repeated rendering,
  - keeps `render_template()` as a one-shot convenience wrapper,
  - renders template content under normal or strict undeclared-token policy.
- `validate`
  - produces validation reports and diagnostics without writing output.
- `pipeline`
  - concatenates resolved profile, guidance, and user prompt blocks in fixed
    order,
  - models internal stage transitions as typestate markers.
- `error`
  - defines crate-owned error types and shared recovery-hint structures,
  - maps lower-level failures into stable public categories.
- `diagnostics`
  - defines diagnostic types,
  - defines the JSON diagnostic schema contract.
- `workspace`
  - implements `frontmatter-init` and `init` logic for reuse by the CLI and any
    future embedded callers.
- `observability`
  - defines event payloads and observer traits,
  - never binds directly to `sc-observability`,
  - allows the CLI or embedded hosts to inject concrete implementations.

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
  defaults: Map<String, ScalarValue>,
  metadata: Map<String, MetadataValue>,
}
```

Normalization rules:

- If frontmatter exists but omits `required_variables`, normalize to `[]`.
- If frontmatter exists but omits `defaults`, normalize to `{}`.
- If frontmatter exists but omits `metadata`, normalize to `{}`.
- If no frontmatter exists, the document has no declarations and no defaults.

Semantic rules:

- `required_variables` declares variables that must exist after context merge.
- `defaults` supplies optional values that may satisfy a required variable.
- `metadata` is descriptive only and does not affect render semantics in the
  initial design.

`ScalarValue` for the initial release means one of:

- string
- number
- boolean
- null

Sequence and mapping values are out of scope for render-context variables in
the initial release.

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
  planned primary repeated-render API

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
- `vars_input: Map<VariableName, ScalarValue>`
- `vars_env: Map<VariableName, ScalarValue>`
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
- `discovered_variables: Vec<String>`
- `changed: bool`

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

Top-level diagnostics envelope:

```json
{
  "schema_version": "1",
  "payload": {
    "ok": false
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
  "severity": "error",
  "code": "ERR_VAL_MISSING_REQUIRED",
  "message": "missing required variable: name",
  "location": "templates/example.md.j2:12:4"
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

Typestate encoding:

- the pipeline should be modeled as state transitions over
  `Document<Parsed>`, `Document<Expanded>`, `Document<Validated>`, and
  `Document<Rendered>`,
- each transition consumes the previous state and returns the next state or a
  canonical error,
- the typestate design exists to make ordering violations unrepresentable in
  internal code, not to force callers to manipulate state markers directly.

## 13. CLI Command Architecture (FR-6, FR-7)

`sc-compose` should be a command router over library operations.

Command mapping:

- `render` -> `compose`
- `resolve` -> `resolve_profile`
- `validate` -> `validate`
- `frontmatter-init` -> `frontmatter_init`
- `init` -> `init_workspace`

The CLI must not reimplement core composition semantics. If a command requires
logic useful to non-CLI callers, that logic belongs in the library.

Command-specific rules:

- `render`
  - accepts `file` mode and `profile` mode,
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
  "valid": false,
  "diagnostics": [
    {
      "severity": "error",
      "code": "ERR_VAL_MISSING_REQUIRED",
      "message": "missing required variable: name",
      "location": "templates/example.md.j2:12:4"
    }
  ]
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

`frontmatter-init --json`

```json
{
  "template_path": "templates/example.md.j2",
  "frontmatter_added": true,
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
  "skipped": false
}
```

Schema notes:

- `search_trace` is the CLI serialization of the library resolver search path
  trace.
- `location` is a single string field in CLI JSON even when the library tracks
  path, line, and column separately.
- `rendered_preview` may be `null` when preview emission is suppressed.

## 14. Output Path Policy (FR-7)

File-writing behavior should be centralized rather than duplicated per command.

Policy requirements:

- file mode strips the final `.j2` suffix,
- profile mode writes to `.prompts/<name>-<ulid>.md`,
- explicit `--output` overrides derived behavior,
- dry-run returns the same derived target information without writing files.

## 15. `init` Command Behavior (FR-7)

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
- values are `ScalarValue`,
- nested arrays and objects are invalid in the initial release.

## 16. Safety Model (FR-4)

- Default deny for out-of-root file access
- No shell execution inside the composition pipeline
- No evaluation of arbitrary host code from templates
- Include stack tracked for all include-related diagnostics
- Deterministic failure semantics for path escape, missing include, cycle, and
  depth overflow

## 17. Error and Exit Semantics (FR-7, FR-8)

The library should expose typed errors with stable categories. Validation
results should remain structured and not collapse into string-only errors.

Target CLI exit semantics:

- `0` success
- `2` validation or render failure
- `3` usage, configuration, or contract error

### 17.1 Failure Mode Matrix

Canonical failures must map to stable error families and stable codes.

| Failure condition | Error type | Stable code |
| --- | --- | --- |
| Template not found | `ResolveError` | `ERR_RESOLVE_NOT_FOUND` |
| Ambiguous template match | `ResolveError` | `ERR_RESOLVE_AMBIGUOUS` |
| Include path escapes confinement root | `IncludeError` | `ERR_INCLUDE_ESCAPE` |
| Include depth exceeds limit | `IncludeError` | `ERR_INCLUDE_DEPTH` |
| Variable type mismatch or invalid scalar | `ValidationError` | `ERR_VAL_TYPE` |
| Duplicate frontmatter variable | `ValidationError` | `ERR_VAL_DUPLICATE` |
| Empty template body | `ValidationError` | `ERR_VAL_EMPTY` |
| Required variable not satisfied after context merge | `ValidationError` | `ERR_VAL_MISSING_REQUIRED` |
| Undeclared referenced token in strict validation or render mode | `ValidationError` | `ERR_VAL_UNDECLARED_TOKEN` |
| Extra provided variable when policy is `error` | `ValidationError` | `ERR_VAL_EXTRA_INPUT` |
| Stdin read attempted twice | `RenderError` | `ERR_RENDER_STDIN_DOUBLE_READ` |
| Output write failure | `RenderError` | `ERR_RENDER_WRITE` |
| Frontmatter rewrite refused on read-only target | `ConfigError` | `ERR_CONFIG_READONLY` |
| Config file missing or malformed | `ConfigError` | `ERR_CONFIG_PARSE` |
| Invalid var-file shape | `ConfigError` | `ERR_CONFIG_VARFILE` |

## 18. Observability Integration (FR-9)

Architecture rules:

- `sc-composer` exposes observer traits or sink traits and emits events through
  that abstraction only.
- `sc-compose` provides the canonical concrete binding to `sc-observability`.
- If no observer is provided, library and CLI behavior degrade to a no-op
  observability path rather than failing.
- Event emission points should cover:
  - command start and end,
  - resolve attempts and outcomes,
  - include expansion outcomes,
  - validation outcomes,
  - render outcomes.
- Library observability hooks must remain usable by embedded consumers.
- Default sink paths for standalone CLI behavior must be tool-scoped.
- Observer and sink traits must be object-safe and `dyn`-compatible.
- Observer and sink traits are intentionally public and unsealed so embedded
  hosts can provide their own implementations.

### 18.1 Host Injection Pattern

Embedded hosts integrate by implementing the public observer or sink traits and
passing those implementations into `sc-composer` or `sc-compose`.

- ATM and other hosts are expected to provide their own concrete observer or
  sink implementations when they need custom telemetry projection.
- A built-in no-op observer remains the default behavior when no host-provided
  implementation is supplied.
- Host injection is an intentional extension point, not a temporary exception.

## 19. Extensibility

The redesign should keep room for future extensions without destabilizing the
core behavior.

Expected extension points:

- typed variable schemas,
- remote include providers,
- template caching,
- custom resolver policies,
- richer frontmatter metadata consumers.

Trait openness decisions:

- observer and sink traits are open extension points for embedded hosts,
- `ResolverPolicy` is open because caller-specific path policy is an explicit
  product requirement,
- value-model and metadata extension points remain closed until a future
  requirement broadens the v1 scalar contract.
