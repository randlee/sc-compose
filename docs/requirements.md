# SC-Compose Requirements

> Status: Active Release Baseline
> Product: `sc-composer` (library) and `sc-compose` (CLI)
> Document role: Normative release requirements for both crates

This document supersedes the prior high-level placeholder. It is the normative
release requirements baseline for `sc-compose` v1.0.

## 1. Intent

This document defines the required behavior of `sc-composer` and `sc-compose`.
It is the design authority for release work. If the implementation diverges
from this document, the implementation is wrong unless the document is
explicitly amended.

## 2. Problem Statement

Teams need one deterministic composition engine for prompt profiles,
instruction templates, and composed prompt output across multiple AI runtimes.
Without a shared implementation, include handling, variable validation,
discovery conventions, and diagnostics drift across callers.

`sc-composer` exists to provide one reusable implementation for:

- prompt and profile file resolution,
- Jinja2-style template rendering,
- include expansion,
- variable declaration and validation,
- deterministic composition output,
- machine-readable diagnostics.

## 3. Product Scope

The product has two deliverables:

- Library crate: `sc-composer`
- CLI binary crate: `sc-compose`

The library is the semantic source of truth. The CLI is a thin interface over
the library.

### 3.1 ATM Independence

This repository is intentionally independent from ATM and any other orchestration
runtime.

- No `ATM_HOME` environment variable may be referenced anywhere in this repo.
- No `agent-team-mail-*` crate may appear in any `Cargo.toml` in this repo.
- No ATM spool, socket, mailbox, or runtime path convention may be assumed.
- No `use atm_*::...` or `use agent_team_mail::...` imports may appear in the
  library or CLI crates.
- Any ATM integration belongs in adapters outside this repository rather than
  in `sc-composer` or `sc-compose`.

### 3.2 Boundary Rules

- `sc-composer` must remain runtime-agnostic.
- `sc-composer` must not depend on mailbox formats, daemon lifecycle behavior,
  team coordination state, or runtime-specific home-directory conventions.
- `sc-compose` must be usable as a standalone tool without any external
  orchestration runtime.
- If an external system needs integration-specific behavior, that adaptation
  must live outside this repository rather than inside the core composition
  semantics.

### 3.3 Non-Goals

The initial product explicitly does not provide:

- daemon control or process management,
- mailbox handling or message routing,
- team configuration or ATM runtime management,
- network I/O or remote template fetching,
- ATM-specific file path conventions or runtime lookup behavior.

## 4. Functional Requirements

### FR-1: Template Inputs

- The engine must support plain text and markup source files, including
  `.txt`, `.md`, and `.xml`.
- The engine must support template files ending in `.j2`, including typed
  variants such as `.md.j2`, `.txt.j2`, and `.xml.j2`.
- Any filename ending in `.j2` must be treated as a template.
- Files may begin with YAML frontmatter.
- Frontmatter is optional.

### FR-1a: Frontmatter Schema

Frontmatter must support this schema:

```yaml
required_variables:
  - variable_name
defaults:
  variable_name: value
metadata:
  key: value
```

Schema rules:

- `required_variables` is optional.
- `defaults` is optional.
- `input_defaults` is accepted as an alias for `defaults` in frontmatter.
- `metadata` is optional.
- If a frontmatter block exists and a field is omitted, it defaults to:
  - `required_variables: []`
  - `defaults: {}`
  - `metadata: {}`
- If no frontmatter block exists at all, the file is treated as having no
  declarations and no defaults.
- `required_variables` values must be unique variable names.
- `defaults` supplies optional values that become part of the render context
  unless overridden by environment-derived or explicit input values.
- If both `defaults` and `input_defaults` appear in the same frontmatter
  block, `input_defaults` wins for overlapping keys and validation emits a
  `WARN_VAL_CONFLICTING_DEFAULT_SECTIONS` warning diagnostic.
- `metadata` is descriptive only. It must not directly change render semantics
  unless a future requirement explicitly assigns meaning to a metadata key.

### FR-1b: Value Types

The render-context value model remains intentionally narrow even after H1
structured-input support lands.

- Variables used by template rendering must be one of:
  - string
  - number
  - boolean
  - null
  - an object/map with string keys; object fields may nest objects and arrays
    of scalars
  - a sequence of scalar values
  - a top-level sequence of objects
- Sequence values may contain supported scalar values or, at the top-level
  variable boundary, object values.
- Nested sequences remain out of scope.
- Arrays of objects are only supported when the array is the variable value
  itself; object fields within array members may be nested objects, but an
  object field whose value is itself an array of objects remains out of scope.
- `metadata` may contain arbitrary YAML values because it is descriptive only
  and does not participate in rendering semantics.

HTML-Report follow-on design track:

- FR-12 through FR-15 are implemented by Phase HTML-Report.
- The remaining design exploration in
  [docs/html-sprint-report-plan.md](html-sprint-report-plan.md) is limited to
  H5-and-later work such as multi-panel HTML/XHTML composition, wrapper-level
  output viewing behavior, and possible post-render-hook design that stays
  outside the core `sc-compose` contract unless explicitly accepted later.

### FR-1c: File Extension and Discovery Conventions

- Profile and prompt assets must support both plain files and template files.
- Within a candidate directory, resolver probe order for agent and command files
  must be:
  1. `<name>.md.j2`
  2. `<name>.md`
  3. `<name>.j2`
- Skill probe order must be:
  1. `<name>/SKILL.md.j2`
  2. `<name>/SKILL.md`
  3. `<name>/SKILL.j2`
- CLI `render` and `validate` must accept explicit template paths anywhere
  under the configured root, including nested skill templates.

### FR-1d: Template Pack Layout

- Bundled examples and user templates use different on-disk layouts.
- Bundled examples are stored on disk as flat `*.j2` files directly under the
  examples root.
- Example names are derived from the template filename by removing the trailing
  `.j2` suffix and then one remaining source extension when present.
  Examples:
  - `hello.md.j2` -> `hello`
  - `service-config.yaml.j2` -> `service-config`
- Derived bundled example names must be unique. If two flat example files
  normalize to the same name, the examples root is invalid until the collision
  is removed.
- User templates are stored as one subdirectory per template under the user
  templates root.
- A user template directory name is the template name.
- A user template directory may contain one or more files, including one or
  more `.j2` templates and supporting assets.
- `template.json` is optional for user template directories. If present, it is
  user-facing metadata and may contain only:
  - `description`
  - `version`
  - `input_defaults`
- `input_defaults` may provide default render inputs using supported
  render-context value types.
- `template.json` must not introduce alternate render semantics, hook
  execution, or manifest-owned entrypoint selection in the initial release.
- The CLI treats each normalized bundled example entry as a single-template
  example pack even though the on-disk layout is a flat file.
- Named render from `sc-compose examples <name>` resolves the matching flat
  example-pack file under the examples root.
- Named render from `sc-compose templates <name>` is defined only when the
  template directory contains exactly one root-level `*.j2` file.
- Template directories with zero or multiple root-level `*.j2` files remain
  listable and addable, but they are not implicitly renderable by name in the
  initial release.

### FR-2: Variable Resolution and Precedence

- Final render context precedence must be:
  1. explicit input variables,
  2. environment-derived variables,
  3. user-template `input_defaults`,
  4. frontmatter defaults.
- Frontmatter-declared `required_variables` must be evaluated after the merge.
- Variables present only in `defaults` are optional by default.
- A variable may appear in both `required_variables` and `defaults`; in that
  case the default value satisfies the requirement unless overridden.
- An empty sequence value such as `[]` is valid input and satisfies a required
  variable when provided explicitly or by defaults.
- `validate` and `render --dry-run` must emit an informational diagnostic when
  a referenced or required variable is satisfied by a default value rather than
  explicit caller input.
- Explicit CLI `--var key=value` inputs are always strings.
- Variables loaded through `--var-file` may be any supported render-context
  value type.
- Variables loaded through `--env-prefix` are always strings.
- If frontmatter is absent:
  - the engine must discover referenced variables from the template and include
    graph,
  - `validate` must emit a generated-frontmatter recommendation,
  - diagnostics must include a direct fix command:
    `sc-compose frontmatter-init <file>.j2`.

### FR-2a: Tokens Not Declared in Frontmatter

Referenced tokens that are not declared in frontmatter must follow these rules:

- Default behavior:
  - they remain preserved in rendered output,
  - they do not become implicitly required variables,
  - they produce diagnostics in both `render` and `validate`.
- Strict behavior:
  - validation fails,
  - rendering fails,
  - diagnostics identify the undeclared referenced tokens.

This behavior is distinct from missing required variables. A token that is
undeclared is not automatically treated as required unless it is explicitly
listed in `required_variables`.

### FR-2b: Missing and Extra Variables

- Missing frontmatter-declared required variables must fail rendering.
- Undefined-variable render failures and undeclared-token diagnostics must use
  distinct stable diagnostic codes.
- Missing-variable diagnostics must include:
  - the full set of missing variable names,
  - the file in which each variable became required,
  - line and column when available,
  - the include chain when applicable.
- Extra input variables not declared by the template or frontmatter must be
  policy-controlled with `error`, `warn`, or `ignore`.

### FR-3: Include Expansion

- The engine must support inline include directives in the form `@<path>`.
- Include resolution order must be:
  1. path relative to the containing file,
  2. path relative to the configured root.
- Nested includes must support:
  - cycle detection,
  - bounded maximum depth,
  - deterministic expansion order.
- Included templates must be evaluated under the same context and validation
  policy as their parent template.
- Include expansion must be applied consistently whether rendering to stdout or
  to a file.
- Include failures must produce actionable diagnostics with include-chain
  context.

### FR-3a: Frontmatter Across Includes

- A file's own frontmatter applies to that file.
- Required-variable declarations discovered from included files participate in
  validation of the overall composition result.
- Defaults declared in included files participate in context construction unless
  overridden by parent-file defaults, environment-derived variables, or
  explicit input variables.
- If multiple files declare a default for the same variable, precedence must be:
  1. explicit input variables,
  2. environment-derived variables,
  3. including file defaults,
  4. included file defaults discovered deeper in the include graph.
- `metadata` from included files must be preserved in trace data only if the
  library exposes include metadata in a future API. Metadata must not affect
  current render semantics.

### FR-4: Safety Constraints

- File reads must be confined to a configured root by default.
- Path traversal outside the allowed root set must fail.
- Callers may optionally provide additional allowed roots.
- Template rendering must not execute arbitrary host code.

### FR-5: Prompt Resolution Conventions

The resolver must support `file` mode and `profile` mode.

In `file` mode:

- the caller provides an explicit path,
- no precedence search is performed.

In `profile` mode:

- the caller provides a profile kind and name,
- the caller may provide a runtime or omit it,
- the resolver searches runtime-specific and shared locations according to a
  configured path policy.

Runtime-specific directories:

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

Shared directories:

- `.agents/agents/`
- `.agents/commands/`
- `.agents/skills/`

Default runtime search order for agents:

- `claude`: `.claude/agents/<name>` -> `.agents/agents/<name>`
- `codex`: `.codex/agents/<name>` -> `.agents/agents/<name>` -> `.claude/agents/<name>`
- `gemini`: `.gemini/agents/<name>` -> `.agents/agents/<name>` -> `.claude/agents/<name>`
- `opencode`: `.opencode/agents/<name>` -> `.agents/agents/<name>` -> `.claude/agents/<name>`

Default runtime search order for commands:

- `claude`: `.claude/commands/<name>` -> `.agents/commands/<name>`
- `codex`: `.codex/commands/<name>` -> `.agents/commands/<name>` -> `.claude/commands/<name>`
- `gemini`: `.gemini/commands/<name>` -> `.agents/commands/<name>` -> `.claude/commands/<name>`
- `opencode`: `.opencode/commands/<name>` -> `.agents/commands/<name>` -> `.claude/commands/<name>`

Default runtime search order for skills:

- `claude`: `.claude/skills/<name>/` -> `.agents/skills/<name>/`
- `codex`: `.codex/skills/<name>/` -> `.agents/skills/<name>/` -> `.claude/skills/<name>/`
- `gemini`: `.gemini/skills/<name>/` -> `.agents/skills/<name>/` -> `.claude/skills/<name>/`
- `opencode`: `.opencode/skills/<name>/` -> `.agents/skills/<name>/` -> `.claude/skills/<name>/`

Ambiguity contract:

- If a runtime is explicitly provided, only that runtime path chain is used.
- If a runtime is omitted, the resolver must evaluate all configured runtime and
  shared roots.
- If multiple candidates match, resolution must fail with an actionable
  ambiguity error requiring an explicit runtime selector.
- If exactly one candidate matches, the resolver may select it without an
  explicit runtime.

There is no flat shared fallback such as `.agents/<name>`. Shared prompts live
only under `.agents/agents/`, `.agents/commands/`, and `.agents/skills/`.

The resolver path policy must be configurable by callers and must not be
hardcoded into downstream integrations.

### FR-6: Composition Pipeline

Final composed output must concatenate blocks in this fixed order:

1. resolved profile body,
2. guidance block,
3. user prompt block.

Each block may be empty. Ordering is never caller-defined.

### FR-7: CLI Surface

`sc-compose` must provide these commands:

- `render`
- `resolve`
- `validate`
- `frontmatter-init`
- `init`
- `observability-health`
- `examples`
- `templates`

The CLI must support:

- `--mode <profile|file>`
- `--kind <agent|command|skill>`
- `--agent <name>`
- `--agent-type <name>` as an alias for `--agent`
- `--runtime <claude|codex|gemini|opencode>` as an optional runtime selector
- `--ai <claude|codex|gemini|opencode>` as an alias for `--runtime`
- `--var key=value` repeatably
- `--var-file <path|->`
- `--env-prefix <PREFIX_>`
- `--strict`
- `--unknown-var-mode <error|warn|ignore>`
- `--root <path>`
- `--file <path>`
- `--output <path>` where applicable
- `--guidance <text>`
- `--guidance-file <path|->`
- `--prompt <text>`
- `--prompt-file <path|->`
- `--json`
- `--dry-run`

Command behavior:

- `render`
  - renders one resolved template or profile,
  - writes to stdout by default,
  - may write to a file when requested,
  - must honor validation and strictness policy,
  - accepts optional guidance and user prompt blocks.
- `resolve`
  - is defined for `profile` mode,
  - prints the selected profile path,
  - reports attempted search paths,
  - fails in `file` mode.
- `validate`
  - performs full include expansion and variable analysis,
  - does not write output files,
  - exits non-zero on validation failure.
- `frontmatter-init`
  - discovers referenced variables,
  - prepends minimal frontmatter,
  - fails if frontmatter already exists unless `--force` is provided.
- `init`
  - creates `.prompts/`,
  - ensures `.prompts/` is ignored by Git,
  - scans repository templates,
  - validates discovered templates,
  - fails if invalid templates are found,
  - prints recommendations for missing or weak frontmatter.
- `observability-health`
  - reads the current CLI logger health state without mutating composition or
    log configuration,
  - prints a human-readable health summary by default,
  - emits the documented JSON schema when `--json` is provided.
- `examples`
  - supports:
    - `examples list`
    - `examples <name>` for implicit named render
  - resolves example packs from the bundled examples root,
  - uses the same render flags and output semantics as `render` for implicit
    named render.
- `templates`
  - supports:
    - `templates list`
    - `templates add <src> [name]`
    - `templates <name>` for implicit named render
  - resolves template packs from the user templates root,
  - uses the same render flags and output semantics as `render` for implicit
    named render,
  - allows `add` from either a single file or a directory source,
  - stores a file source as `<user-template-root>/<pack-name>/<original-file>`,
  - stores a directory source as `<user-template-root>/<pack-name>/...`.

`--dry-run` behavior:

- For file-writing render operations, `--dry-run` must report:
  - resolved template path,
  - resolved output path,
  - whether content would change,
  - validation and render diagnostics.
- For `frontmatter-init`, `--dry-run` must print the exact frontmatter that
  would be written.
- For `init`, `--dry-run` must print planned filesystem changes, validation
  results, and recommendations without modifying the workspace.

Guidance and prompt input rules:

- `--guidance` and `--guidance-file` are mutually exclusive.
- `--prompt` and `--prompt-file` are mutually exclusive.
- `--guidance-file -` reads guidance content from stdin.
- `--prompt-file -` reads prompt content from stdin.
- If both guidance and prompt are omitted, only the resolved profile body is
  composed.
- The CLI must reject attempts to read both guidance and prompt from the same
  stdin stream in a single invocation.
- CLI-only aliases such as `--agent-type` and `--ai` must be resolved before
  library request construction. The library API does not expose alias concepts.

Default output path policy:

- File mode removes the trailing `.j2` suffix from the template filename.
- Profile mode writes to `.prompts/<name>-<ulid>.md` unless `--output` is
  supplied.

Pack root policy:

- `examples` resolves example packs from:
  1. `SC_COMPOSE_DATA_DIR/examples`
  2. install-relative `../share/sc-compose/examples/`
- `templates` resolves template packs from:
  1. `SC_COMPOSE_TEMPLATE_DIR`
  2. the platform user-data directory joined with `sc-compose/templates/`
- `templates add` must fail if the destination pack name already exists.
- `examples` is read-only. It must not mutate the bundled examples root.

### FR-7a: Variable File Rules

- `--var-file` accepts a JSON or YAML object.
- Variable-file keys must be strings.
- Variable-file values must be supported render-context value types.
- Object/map values with string keys are valid per FR-12.
- Sequence values in variable files may contain scalar values or, at the
  top-level variable boundary, object values.
- Nested arrays remain invalid and must report
  `ERR_VAL_NESTED_ARRAY_UNSUPPORTED`.
- Arrays of objects are valid per FR-13 when the array is the variable value
  itself.

### FR-7b: Exit Codes

CLI exit codes must be:

- `0` for success
- `2` for validation or render failure
- `3` for usage, configuration, or contract error

### FR-7c: Template Whitespace Control

The template engine must enable `trim_blocks` and `lstrip_blocks` by default.
Authors may opt out for a specific block with the standard Jinja `+` modifier.

### FR-8: Determinism and Diagnostics

- The same logical inputs must produce byte-identical output.
- Diagnostics must include:
  - stable diagnostic code,
  - human-readable message,
  - source file path,
  - line and column when available,
  - include stack when applicable,
  - severity.
- JSON diagnostics must use a stable, versioned schema suitable for machine
  consumers.

### FR-8a: Command JSON and Dry-Run Schemas

CLI `--json` output must use the versioned `DiagnosticEnvelope` as the
canonical transport format:

```json
{
  "schema_version": "1",
  "payload": {},
  "diagnostics": []
}
```

Per-command schemas below describe the shape of the `payload` field within that
envelope.

`render --json`

```json
{
  "schema_version": "1",
  "payload": {
    "output_path": "stdout",
    "bytes_written": 123,
    "template": "path/to/template.md.j2"
  },
  "diagnostics": []
}
```

Schema rules:

- `output_path` is a string and uses `"stdout"` when no file is written.
- `bytes_written` is the actual byte count written to the selected output
  target; when writing to stdout it is the UTF-8 byte length emitted to stdout.
- `template` is the resolved template path as a string.

`render --dry-run --json`

```json
{
  "schema_version": "1",
  "payload": {
    "would_write": ".prompts/example-01HXYZ.md",
    "would_change": true,
    "template": "path/to/template.md.j2",
    "rendered_preview": "preview text"
  },
  "diagnostics": []
}
```

Schema rules:

- `would_write` is the derived output target as a string.
- `would_change` records whether the dry-run output differs from the current
  file content at the derived output path; missing output files count as
  `true`.
- `rendered_preview` is a preview string.

`resolve --json`

```json
{
  "schema_version": "1",
  "payload": {
    "resolved_path": ".claude/agents/example.md.j2",
    "search_trace": [
      ".claude/agents/example.md.j2",
      ".agents/agents/example.md.j2"
    ],
    "found": true
  },
  "diagnostics": []
}
```

`validate --json`

```json
{
  "schema_version": "1",
  "payload": {
    "valid": true
  },
  "diagnostics": [
    {
      "severity": "info",
      "code": "INFO_VAL_DEFAULT_USED",
      "message": "variable name not provided, using default: \"world\"",
      "location": "templates/example.md.j2"
    }
  ]
}
```

`init --json`

```json
{
  "schema_version": "1",
  "payload": {
    "workspace_root": "/repo",
    "created_files": [
      ".prompts/",
      ".gitignore"
    ]
  },
  "diagnostics": []
}
```

`observability-health --json`

```json
{
  "schema_version": "1",
  "payload": {
    "logging": {
      "state": "Healthy",
      "dropped_events_total": 0,
      "flush_errors_total": 0,
      "active_log_path": "<log_root>/logs/sc-compose.log.jsonl",
      "sink_statuses": [],
      "last_error": null,
      "query": null
    }
  },
  "diagnostics": []
}
```

Schema rules:

- `payload.logging` is the JSON serialization of
  `sc_observability::LoggingHealthReport`.
- `LoggingHealthReport` is accessed through the `sc-observability` re-export
  surface for logging-only consumers, per DOC-007 and LOG-038.
- `payload.logging.query` is `null` when query/follow health is unavailable and
  otherwise contains a `QueryHealthReport`.
- `active_log_path` is derived from the configured log root and service name
  using the `LOG-008` layout `<log_root>/logs/<service>.log.jsonl`.
- The concrete path is platform-dependent; on Windows it may be drive-qualified.
- `observability-health --json` must not emit console log lines that corrupt
  the JSON envelope written to stdout.

`frontmatter-init --json`

```json
{
  "schema_version": "1",
  "payload": {
    "template_path": "templates/example.md.j2",
    "frontmatter_added": true,
    "would_change": true,
    "vars": [
      "name",
      "role"
    ]
  },
  "diagnostics": []
}
```

`frontmatter-init --dry-run --json`

```json
{
  "schema_version": "1",
  "payload": {
    "action": "frontmatter-init",
    "would_affect": [
      "templates/example.md.j2"
    ],
    "changed": false,
    "would_change": true,
    "skipped": false,
    "vars": [
      "name",
      "role"
    ]
  },
  "diagnostics": []
}
```

`init --dry-run --json`

```json
{
  "schema_version": "1",
  "payload": {
    "action": "init",
    "would_affect": [
      ".prompts/",
      ".gitignore"
    ],
    "changed": false,
    "would_change": true,
    "skipped": false
  },
  "diagnostics": []
}
```

Schema rules:

- `action` names the command.
- `would_affect` lists the filesystem paths or logical targets that would
  change.
- `changed` remains `false` for dry-run operations because no write occurs.
- `would_change` records whether the command would modify its target if writes
  were enabled.
- `skipped` is `true` when the command decides no change is needed.

`examples list --json`

```json
{
  "schema_version": "1",
  "payload": {
    "packs": [
      {
        "name": "hello",
        "path": "/path/to/share/sc-compose/examples/hello.md.j2"
      }
    ]
  },
  "diagnostics": []
}
```

`templates list --json`

```json
{
  "schema_version": "1",
  "payload": {
    "packs": [
      {
        "name": "pytest-fixture",
        "path": "/path/to/user-data/sc-compose/templates/pytest-fixture"
      }
    ]
  },
  "diagnostics": []
}
```

`templates add --json`

```json
{
  "schema_version": "1",
  "payload": {
    "name": "pytest-fixture",
    "source": "/path/to/source/pytest-fixture.py.j2",
    "destination": "/path/to/user-data/sc-compose/templates/pytest-fixture",
    "changed": true
  },
  "diagnostics": []
}
```

Named render through `examples <name>` and `templates <name>` must emit the
same command payloads as `render` and `render --dry-run`.

### FR-9: Observability

- `sc-composer` must not depend directly on `sc-observability`.
- `sc-composer` must not depend on `sc-observability-types`.
- `sc-composer` must define host-injectable observability hooks locally without
  coupling the library to a concrete logging runtime.
- The initial release observability scope is limited to structured logging,
  health reporting, and downstream extension through the local observer hook
  model.
- `sc-compose` shall use `sc-observability` as the canonical concrete
  observability binding for CLI execution.
- `sc-composer` must emit composition pipeline events through its local
  observer/sink hook model.
- `sc-compose` must emit command lifecycle events through the same local hook
  model.
- Standalone defaults must keep `sc-compose` sink paths tool-scoped.
- Embedded use must permit host-supplied sink and path configuration.
- If no sink is injected, both crates must remain fully functional with
  observability reduced to a no-op.
- `sc-observe` and `sc-observability-otlp` remain out of scope for the initial
  release.

### FR-10: Library Log-Sink Injection

- `sc-composer` shall define its minimal observability hook layer locally in
  `sc_composer::observer`.
- The library hook surface shall remain a local sink/observer abstraction over
  `ObservationEvent` rather than importing observability contracts from
  `sc-observability-types`.
- `Renderer::new(config)` and `compose()` shall preserve no-op behavior when the
  caller does not provide an observer implementation.
- `compose_with_observer(request, &mut dyn CompositionObserver)` shall remain
  the required end-to-end injection surface for host-provided observability.
- The local observer hook surface shall remain object-safe and `dyn`-compatible
  so consuming applications can provide their own logging extensions without
  depending on CLI-specific code.
- Injected hooks shall receive structured events for the resolve,
  include-expand, validate, and render pipeline stages.
- The local observer/sink contracts shall remain usable by embedded hosts that
  do not use the CLI.

### FR-11: CLI Observability Wiring

- `sc-compose` shall construct the concrete `sc-observability` `Logger` during
  CLI startup and wire it into the `sc-composer` injection point.
- The CLI logger wiring shall register both file and console sinks during
  normal terminal execution.
- The console sink shall be suppressed whenever the active command uses the
  `--json` output mode so machine-readable command output remains clean.
- The CLI shall emit structured command lifecycle events for command start,
  command completion, and command failure.
- The CLI shall expose logger health through a dedicated
  `observability-health` command so operators can inspect sink state,
  dropped-event counts, and the active log path.
- The `observability-health` command shall initialize logger configuration the
  same way as a normal CLI process, query health from that process-local
  logger instance, and must not depend on any daemon or background runtime.
- The CLI shall perform graceful logger shutdown on process exit so pending
  events flush before termination.

### Phase HTML-Report Functional Requirements (FR-12 through FR-15)

### FR-12: Map/Object Variable Inputs

Implemented in Phase HTML-Report.

- Callers may pass structured object/map values as template variables.
- Object keys must be strings.
- Valid object fields must be accessible through normal Jinja field access such
  as:
  - `{{ pr.number }}`
  - `{{ pr.url }}`
- Bracket access remains valid when a key is not a valid dotted identifier.
- Structured inputs participate in the same precedence model as existing
  inputs:
  1. explicit input variables,
  2. environment-derived variables,
  3. user-template `input_defaults`,
  4. frontmatter defaults.
- `required_variables` may name nested field paths such as `pr.number` once
  structured inputs are implemented.
- Missing nested required fields must report the full field path, for example
  `pr.number`.
- Malformed object input must fail with stable diagnostics using
  `ERR_VAL_OBJECT_SHAPE`.
- Nested required-path traversal that encounters a scalar where an object is
  required must fail with `ERR_VAL_SHAPE_MISMATCH`.
- `--var key=value` remains string-only in this phase. Structured input comes
  from `--var-file`, frontmatter defaults, or `template.json` `input_defaults`.
- JSON and YAML var-file documents remain top-level objects. Structured values
  are carried in object fields within that top-level object.

### FR-13: Arrays Of Objects

Implemented in Phase HTML-Report.

- Callers may pass arrays whose members are objects when the array itself is
  the variable value.
- Jinja loops such as `{% for item in list %}` must support field access within
  each array member object.
- Arrays of objects are valid through:
  - `--var-file`,
  - frontmatter defaults,
  - user-template `template.json` `input_defaults`.
- Empty arrays remain valid inputs.
- Arrays of objects may contain nested object fields. Arrays of arrays remain a
  separate decision and are not implied by this requirement.
- Nested arrays are out of scope for H1 and H2. Callers who pass an array that
  contains another array, or an object that contains an array at a nested
  field, must receive `ERR_VAL_NESTED_ARRAY_UNSUPPORTED`.
- Missing nested fields inside array members must report stable field-path
  diagnostics using `ERR_VAL_MISSING_NESTED_FIELD`.
- `frontmatter-init` must discover variable references inside `for` loop
  bodies. References inside a loop body are attributed to the array variable:
  `{{ sprint.id }}` inside `{% for sprint in sprints %}` means `sprints` is a
  required variable, not `sprint` or `sprint.id`.

### FR-14: HTML Template Output

Implemented in Phase HTML-Report.

- `.html.j2` templates render like other file-mode templates.
- Output path derivation removes only the trailing `.j2` suffix and therefore
  preserves the `.html` extension.
- Rendered HTML is treated as a normal template artifact.
- `sc-compose` does not enable MiniJinja auto-escaping for `.html.j2`
  templates. Template authors remain responsible for escaping user-supplied
  values.
- Self-contained output, XHTML shape, inline CSS, and browser-viewability are
  template-author responsibilities rather than core-engine enforcement.
- Dry-run, diagnostics, validation, and output-path rules apply to HTML
  templates the same way they apply to other file-mode templates.

### FR-15: Bundled HTML Report Example

Implemented in Phase HTML-Report.

- `sc-compose` shall ship a bundled example named `sprint-report-html`.
- The example must demonstrate FR-12, FR-13, and FR-14 together using a
  self-contained HTML sprint status report.
- The H3 example is a single flat file at
  `examples/sprint-report-html.html.j2`. Directory-based example-pack layout is
  deferred to H4 or a later architecture amendment.
- The example must include realistic structured input data showing:
  - report metadata,
  - sprint entries,
  - PR metadata,
  - CI status metadata and actionable links,
  - actionable links such as PR and CI URLs.
- The example must remain renderable through the standard examples command
  surface and var-file flow.
- The example must be a credible showcase for `sc-compose`, not just a
  hand-written HTML file stored in the repo.

### Phase A Semantic Report-Spec Contract (Planning Only)

Phase A follow-on planning defines typed semantic report-spec kinds so rendered
diagram formats such as Mermaid become outputs or migration inputs rather than
the long-term source of truth.

Initial planned report-spec kinds:

- `state_machine`
- `sql_query`

Planned `state_machine` semantic fields:

- `kind`
- `id`
- `title`
- `states`
- `transitions`
- `events`
- `guards`
- `actors`
- `effects`
- optional metadata for ownership, tags, and renderer targets

Planned `sql_query` semantic fields:

- `kind`
- `id`
- `title`
- `purpose`
- `tables_read`
- `tables_written`
- `filters`
- `ordering`
- `cardinality`
- `transactional_assumptions`
- optional metadata for ownership, tags, and renderer targets

Transitional Mermaid rule:

- Mermaid may be emitted as an output renderer during migration
- Mermaid may be accepted as a migration input where repos already store it
- Mermaid is not the long-term semantic source model

Semantic QA direction:

- QA should validate structured semantic fields rather than string-compare only
  the rendered Mermaid output
- renderers may change over time without replacing the typed semantic source
  contract

Extension rule:

- repos may add new semantic report-spec kinds later without rewriting the
  shared artifact catalog or producer contracts

Boundary rules:

- the semantic source contract remains format-agnostic
- network publishing remains outside the core engine
- browser-open behavior remains outside the core engine

### Phase A Follow-On Reporting Contract (Planning Only)

Phase A follow-on planning defines reporting as a generic artifact contract,
not as a one-off HTML sprint report feature.

Planned contract shape:

- authored docs remain under `docs/`
- report source and catalog inputs live outside `docs/` under a report-specific
  tree such as:
  - `reports/catalog/`
  - `reports/specs/`
  - `reports/templates/`
- generated evidence lives outside `docs/` under generated-output paths such
  as:
  - `reports/latest/<report-id>/`
  - `reports/archive/<timestamp>/<report-id>/`
- each generated report has one machine-readable metadata sidecar such as
  `reports/latest/<report-id>/report.json`

Planned canonical report catalog fields:

- `id`
- `kind`
- `producer`
- `entrypoint`
- `metadata`

Planned ownership split:

- producer recipes such as `just lint`, `just test`, `just smoke`, and
  repo-specific producer commands own domain data gathering and report
  generation
- `sc-compose` owns rendering semantics where it is used as the report
  renderer
- consumer repos own domain-specific inputs, local producer surfaces, and
  publish destinations

Boundary rules for the follow-on line:

- network publishing remains outside the core engine
- browser-open behavior remains outside the core engine
- the artifact contract is intended to support generic lint, test, smoke,
  diagram, and custom reports through one shared metadata and filesystem shape

### Phase A Source-Driven Rendering Contract (Planning Only)

Phase A follow-on planning defines a generic source-driven rendering contract
for text assets. This mechanism is not Mermaid-only.

Planned collection-input contract:

- source collections may be declared by glob or by another stable collection
  definition
- a collection declares which source files participate in one render-many run
- collection discovery is generic across Mermaid, SVG, Markdown, and other
  text-based assets

Planned metadata-extraction contract:

- comment-prefix metadata is supported
- block-comment metadata is supported
- the raw source body remains available to templates without external
  scripting
- parsed metadata and raw body are exposed together as render inputs
- `sets` metadata is a collection-local grouping field used to tag one source
  file into one or more logical sets for selective rendering, filtering, or
  aggregate grouping
- `sets` has type `Option<Vec<String>>` or equivalent optional string-list
  representation and defaults to `None` when absent

Planned render-many contract:

- one generated output is produced per discovered source file
- output derivation is deterministic from collection membership plus source
  identity
- aggregate templates and review tooling consume a generated manifest rather
  than ad hoc wrapper state

Planned generated-manifest contract:

- each source-driven run emits a manifest describing the discovered sources and
  generated outputs
- the manifest is intended for aggregate templates and review tooling
- browser automation and hosted site behavior remain out of scope for the core
  engine

Boundary rules for the source-driven line:

- the mechanism remains generic rather than diagram-format-specific
- network publishing remains outside the core engine
- browser-open behavior remains outside the core engine

### Phase A Latest/Archive Output And Reports Aggregator Contract (Planning Only)

Phase A follow-on planning defines how producers write stable latest outputs,
how optional timestamped archive copies are named, and how `just reports`
aggregates and verifies generated evidence.

Planned output policy:

- producers overwrite the latest artifact in place at the canonical
  `reports/latest/<report-id>/...` path
- producers may also write timestamped archive copies under
  `reports/archive/<timestamp>/<report-id>/...`
- archive writes are deterministic and file-system-local

Canonical archive timestamp policy:

- timestamps use a filesystem-safe UTC form such as `2026-05-25T22-10-00Z`
- one producer run uses one stable timestamp prefix for all archive outputs
  generated in that run

Planned `just reports` contract:

- verify required evidence exists
- summarize report status across producers
- build or refresh a combined index when the repo defines one
- open or view the latest report set

Verification and failure direction:

- `just reports` is a shared aggregator and verifier, not a producer that
  reruns all evidence collection
- missing required evidence causes report verification to fail
- required-vs-optional report expectations come from the shared report catalog

Archive ownership note:

- archive directories are file-system-local
- archive directories may be consumer-managed
- archive directories may be gitignored
### Phase A Publish-Manifest And CI Handoff Contract (Planning Only)

Phase A follow-on planning defines a machine-readable handoff from generated
report artifacts to CI or wrapper-owned publication steps without moving
network or hosting behavior into `sc-compose`.

Planned publish-manifest contract:

- each generated report set may emit a machine-readable publish manifest
- the manifest lists generated artifacts and their intended publish destinations
- artifact roles remain explicit in the manifest rather than inferred by CI

Planned manifest fields include:

- report_name
- generated timestamp
- files
- per-file role
- per-file path
- per-file intended publish destination

Ownership split:

- producers and renderers create artifacts plus manifest metadata
- CI or wrapper tooling performs upload, copy, or publication steps

Boundary rules:

- the artifact contract is intended to support generic lint, test, smoke,
  diagram, and custom reports through one shared metadata and filesystem shape
- browser-open behavior remains outside the core engine
- publish transport remains outside `sc-composer` and `sc-compose`
- hosting logic remains outside `sc-composer` and `sc-compose`
- machine-readable handoff is in scope; network transport is not

### Phase A Producer Recipe Contract (Planning Only)

Phase A follow-on planning defines producer recipes as the owners of report
generation. Report generation is not centered on one catch-all `just reports`
command.

Planned standard producer surface:

- `just lint`
- `just test`
- `just smoke`
- repo-specific producer commands such as:
  - `just state-diagrams`
  - `just sql-diagrams`
  - schema, migration, or other repo-local evidence producers

Planned producer contract:

- each producer command is responsible for generating the report artifacts for
  the report ids it owns
- each producer command writes evidence in the shared report artifact shape
- each producer command updates or emits the catalog/metadata entries for the
  report ids it owns
- adding a repo-specific producer command must not require changing the shared
  report aggregation or discovery contract

Boundary rules for the producer line:

- producer recipes own domain data gathering and invocation order
- `just reports` is reserved for aggregation, verification, and opening/viewing
- network publishing remains outside the core engine
- browser-open behavior remains outside the core engine
- the report ids owned by a producer are declared through the shared report
  catalog rather than inferred from hard-coded aggregator behavior

### Phase A Template-Family And Panel-Chrome Contract (Planning Only)

Phase A follow-on planning defines shared template families and shared panel
chrome so report UI behavior does not need to be reimplemented per consumer
repo.

Initial planned template families:

- lint/test/smoke evidence reports
- public API, CLI, and ICD style reports
- diagram, state-machine, and SQL-query reports

Planned override contract:

The authoritative override contract, shared lookup namespace, consumer
activation config, template block boundary, required template variables, and
include deferral are defined in `docs/phase-A/sprint-A5.md`.

Shared panel contract:

- stable panel id
- title
- body content
- required copy-text action
- optional copy-JSON action
- optional fragment or open link

Planned ownership split:

- shared panel chrome owns panel framing and shared actions
- consumer-specific templates own the panel body content for their report
  family or repo-local override

Boundary rules:

- per-panel text copy is mandatory
- per-panel JSON copy is optional but first-class
- panel chrome remains part of shared template behavior rather than wrapper-only
  logic
- network publishing and browser-open behavior remain outside the core engine

## 5. Non-Functional Requirements

- Cross-platform support is required for macOS, Linux, and Windows.
- The product must not rely on shell-specific behavior.
- Single-template `render`, `resolve`, and `validate` operations must be fast
  enough for interactive terminal use on local repositories.
- The public library API must be stable enough for downstream integration and
  semver-governed once released.
- The library and CLI must remain separable: `sc-compose` may depend on
  `sc-composer`, but `sc-composer` must not depend on the CLI crate.
- Observability integration must emit structured events at the resolve,
  include-expand, validate, and render pipeline stages with stable target,
  action, and message conventions.
- Observability health state must be queryable without mutating composition
  behavior so operators and embedded hosts can inspect runtime health safely.
- Process shutdown must flush pending observability output and degrade
  gracefully when sink flushing reports errors.

## 6. Stability Policy

- The `sc-composer` public API is semver-governed.
- Before `1.0`, breaking API changes require a minor version bump.
- After `1.0`, patch releases contain backward-compatible bug fixes only.
- After `1.0`, minor releases contain backward-compatible new features.
- After `1.0`, major releases contain breaking changes.
- `render_template()` is a stable convenience API for one-shot rendering.
- `Renderer` is the primary stable API for repeated rendering and long-lived
  library use.

## 7. Testing Requirements

Required unit coverage includes:

- frontmatter parsing,
- frontmatter omission defaults,
- variable precedence,
- required-variable enforcement,
- undeclared-variable behavior in normal and strict modes,
- unknown-variable policy handling,
- include resolution, cycle detection, and depth limits,
- include-driven defaults and required-variable propagation,
- path confinement,
- resolver precedence.

Required integration coverage includes:

- CLI `render`,
- CLI `resolve`,
- CLI `validate`,
- CLI `frontmatter-init`,
- CLI `init`,
- CLI `observability-health`,
- CLI `examples list`,
- CLI `examples <name>`,
- CLI `templates list`,
- CLI `templates add`,
- CLI `templates <name>`,
- command lifecycle logging,
- resolve/include-expand/validate/render event emission,
- `--dry-run` no-write guarantees,
- JSON diagnostics contract,
- cross-platform path behavior,
- template-pack discovery and add semantics,
- list/array input behavior through frontmatter defaults, `template.json`
  `input_defaults` for user templates, and `--var-file`.

## 8. Out of Scope for the Initial Release

- Remote includes such as `http` or `https`
- Arbitrary plugin execution from templates
- Runtime-specific hooks and event integrations inside the core composition
  engine
- `prepare-hook` and `post-render-hook` execution
- Named render for packs with multiple root-level `*.j2` entry candidates
- Template deletion, update, sync, or remote registry features
- Nested arrays, nested array-of-object fields, multi-panel HTML/XHTML report
  expansion, and wrapper-level output viewing behavior remain deferred to the
  follow-on design track in
  [docs/html-sprint-report-plan.md](html-sprint-report-plan.md)
