# SC-Compose Requirements

> Status: Active Release Baseline
> Product: `sc-composer` (library) and `sc-compose` (CLI)
> Document role: Normative release requirements for both crates

This document supersedes the prior high-level placeholder. It is the normative
release requirements baseline for `sc-compose` v0.x.

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
- `metadata` is descriptive only. It must not directly change render semantics
  unless a future requirement explicitly assigns meaning to a metadata key.

### FR-1b: Value Types

For the initial release, the render-context value model remains intentionally
narrow even after template-pack support is added.

- Variables used by template rendering must be one of:
  - string
  - number
  - boolean
  - null
  - a sequence of scalar values
- Sequence values may contain only supported scalar value types.
- Nested sequences and mapping values are out of scope for template variables,
  `defaults`, and user-template `input_defaults` in the initial release.
- `metadata` may contain arbitrary YAML values because it is descriptive only
  and does not participate in rendering semantics.

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
- Bundled examples are flat `*.j2` files stored directly under the examples
  root.
- Example names are file stems with the trailing `.j2` removed.
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
- Named render from `sc-compose examples <name>` resolves the matching flat
  example file under the examples root.
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
  - resolves packs from the bundled examples root,
  - uses the same render flags and output semantics as `render` for implicit
    named render.
- `templates`
  - supports:
    - `templates list`
    - `templates add <src> [name]`
    - `templates <name>` for implicit named render
  - resolves packs from the user templates root,
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

- `examples` resolves packs from:
  1. `SC_COMPOSE_DATA_DIR/examples`
  2. install-relative `../share/sc-compose/examples/`
- `templates` resolves packs from:
  1. `SC_COMPOSE_TEMPLATE_DIR`
  2. the platform user-data directory joined with `sc-compose/templates/`
- `templates add` must fail if the destination pack name already exists.
- `examples` is read-only. It must not mutate the bundled examples root.

### FR-7a: Variable File Rules

- `--var-file` accepts a JSON or YAML object.
- Variable-file keys must be strings.
- Variable-file values must be supported render-context value types.
- Sequence values in variable files must contain only supported scalar values.
- Nested objects and nested sequences are invalid in the initial release.

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
- `rendered_preview` is either a preview string or `null`.

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
    "valid": false
  },
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
- Until `1.0`, breaking API changes require a minor version bump.
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
