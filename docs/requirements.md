# SC-Compose Requirements

> Status: Active Release Baseline
> Product: `sc-composer` (library) and `sc-compose` (CLI)
> Document role: Normative release requirements for both crates

This document supersedes the prior high-level placeholder. It is the normative
release requirements baseline for `sc-compose` v1.4.1.

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

### 3.1a Phase P Go module distribution

The Phase P generated-Go strategy is governed by
[ADR-0020: Generated Go Binding Strategy](adrs/0020-generated-go-binding-strategy.md),
which is **Accepted**. The native libraries are distributed as the
target-specific release bundles documented by the Go integration guide; a Go
module download alone does not include those native libraries.

The repository MUST publish a generated Go module for the canonical `sc-sha`
hash operations so external consumers can use the shared hash contract without
copying its implementation. The module MUST use the documented submodule path
`github.com/randlee/sc-compose/bindings/sc-sha-go`, generated UniFFI source, and
the matching packaged native artifact for each supported target (Linux/amd64 and arm64, macOS/amd64 and arm64,
and Windows/amd64 and arm64), as required by FR-25 and ADR-0025. Release validation MUST cover generated
source drift, native artifact selection, conformance vectors, and an
independent Go consumer. The adapter MUST depend only on `sc-sha` and MUST NOT
add ATM, resolver, filesystem, or duplicate hash logic.

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
pass: 1
required_variables:
  - variable_name
variables:
  variable_name:
    required: true
defaults:
  variable_name: value
input_defaults:
  variable_name: fallback
metadata:
  key: value
json_escape_mode: auto
```

Schema rules:

- `required_variables` is optional.
- `pass` is optional and identifies an explicit pass number for stacked
  frontmatter blocks.
- `defaults` is optional.
- `input_defaults` is accepted as an alias for `defaults` in frontmatter.
- For compatibility with existing template metadata, a frontmatter
  `variables` map with `{ required: true }` declarations is accepted as an
  equivalent spelling of `required_variables`.
- `metadata` is optional.
- `json_escape_mode` is optional for JSON templates and accepts only `auto` or
  `legacy`. `auto` is the default; `legacy` is an explicit compatibility mode
  for manually quoted string placeholders and emits a migration warning.
- The recognized top-level frontmatter keys are `pass`,
  `required_variables`, `variables`, `defaults`, `input_defaults`,
  `metadata`, and `json_escape_mode`. When scanning stacked frontmatter, a later block containing an
  unrecognized top-level key is treated as template body content.
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

### FR-1b-json: JSON interpolation contract

- JSON templates use complete-value `auto` interpolation by default. Bare
  placeholders own their JSON quoting and preserve scalar, object, array, and
  null types.
- An explicit `legacy` mode safely escapes string contents without adding
  surrounding quotes already present in manually quoted source.
- CLI mode selection takes precedence over root frontmatter, followed by the
  default `auto` mode.
- `validate` and `validate --lint` emit the migration-directed warning for
  legacy mode or quoted placeholders detected in JSON context:
  `Template uses legacy JSON escape mode. Migrate to bare placeholders (auto
  mode) to avoid double-quoting issues. See docs/migration/json-escape-mode.md`
- A JSON render must not emit output until the complete body parses
  successfully.
- The parser gate applies to ordinary `render` as well as `render --check-render`
  and runs before stdout, file, dry-run preview, or JSON-envelope emission.
- `validate` remains static-only and must report that state explicitly. The
  `validate --check-render` variant renders in memory with the supplied exact
  context, reports the checked-render state, and emits no body or file.
- A malformed rendered JSON body must fail closed with the stable
  `ERR_RENDER_JSON_MALFORMED` diagnostic. The diagnostic includes template,
  line, column, and byte offset, but never echoes rendered values.
- Source lint uses the canonical `ERR_JSON_MODE_CONTRACT` diagnostic for an
  auto-mode quoted scalar placeholder. Ambiguous quoted expressions use
  `WARN_JSON_QUOTED_PLACEHOLDER` as a conservative finding instead of being
  silently treated as safe. The existing
  `WARN_JSON_LEGACY_ESCAPE_MODE` migration warning remains emitted by static
  validation for legacy mode or detected quoted placeholders.

The render-context value model accepts any finite JSON/YAML-compatible tree
that the existing `serde_json::Value` and Minijinja context can represent.

The six Phase O.4 repository assignment templates are explicit `auto`-mode
consumers of this contract. Their scalar interpolation slots and scalar loop
elements are bare; `carry_forward_findings_json` is the only reviewed raw-JSON
fragment and must contain a validated JSON array. The O.4 semantic fixture
corpus is the acceptance evidence for quotes, backslashes, Unicode, newlines,
empty and optional values, arrays, objects, null branches, and injection-safe
control characters. The migration matrix and legacy exception are maintained
in `docs/migration/json-escape-mode.md`.

The Phase O.5 release gate extends this contract across consumer repositories:

- every release-candidate campaign reads a source-of-truth inventory of
  repository roots and pinned commits, verifies each commit before scanning,
  and reports the actual JSON-template count and every path;
- an unavailable or unpinned root blocks an unconditional release claim;
- a successful JSON render is a valid campaign PASS only after the complete
  emitted body is parsed as one JSON document; parser failure, partial output,
  timeout, or success-status/body mismatch is fail-closed evidence;
- the original 1.4.0 quoted-placeholder shape remains a permanent negative
  fixture: auto mode rejects it before emission, while explicit legacy mode
  produces one safely escaped string and one deprecation diagnostic;
- external findings are owned by the external repository and require a
  separately merged migration/fix before the release gate can become green.

- Variables used by template rendering must be one of:
  - string
  - number
  - boolean
  - null
  - an object/map with string keys, recursively containing supported values
  - a sequence recursively containing supported values, including arrays,
    objects, scalars, `null`, and jagged shapes
- The top-level `--var-file` document remains a JSON/YAML object and YAML map
  keys remain strings; these ingress boundaries are independent of nesting
  depth.
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
  3. built-in render-context variables,
  4. user-template `input_defaults`,
  5. frontmatter defaults.
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
- This string-only behavior is intentional: CLI text inputs are not coerced
  based on their spelling. Callers that need numeric, boolean, null, object,
  or sequence values must use `--var-file` or template-owned defaults.
- Variables loaded through `--var-file` may be any supported render-context
  value type.
- Variables loaded through `--env-prefix` are always strings.
- After the final context merge, every referenced variable path must either
  have a value binding or be handled by the unbound-variable policy described
  in FR-2a. A missing binding is distinct from an undeclared token and from an
  extra caller-provided variable.
- `ComposePolicy.unbound_variable_policy` controls referenced-but-unbound
  paths with `error`, `warn`, or `ignore` severity. When unset, it inherits
  `unknown_variable_policy` for compatibility; an explicit value keeps the
  two policy axes independent.
- If frontmatter is absent:
  - the engine must discover referenced variables from the template and include
    graph,
  - `validate` must emit a generated-frontmatter recommendation,
  - diagnostics must include a direct fix command:
    `sc-compose frontmatter-init <file>.j2`.

### FR-2c: Built-In Render-Context Variables

- Every render context must inject these built-in variables when the caller
  does not supply them:
  - `TEMPLATE_NAME`
  - `HOSTNAME`
  - `USERNAME`
  - `RENDER_DATE`
  - `RENDER_TIMESTAMP`
- Built-ins sit below explicit caller inputs and environment-derived values,
  and above template-owned defaults.
- `TEMPLATE_NAME` must reflect the resolved template filename actually rendered,
  not a caller alias.
- Caller-provided values always win:
  - explicit input values override built-ins,
  - environment-derived values override built-ins,
  - template-owned defaults do not override built-ins.

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

Referenced-but-unbound tokens are a separate axis from undeclared tokens:

- A referenced token with no binding after the final context/default merge
  emits `ERR_VAL_UNBOUND_VARIABLE` according to
  `unbound_variable_policy`.
- A referenced token may be declared in frontmatter and still be unbound; the
  unbound-variable policy applies independently of `strict_undeclared_variables`.
- A referenced token may be undeclared but bound by caller input; the strict
  undeclared-token policy applies independently and the value is not reported
  as unbound.
- Loop locals, `{% set %}` locals, built-in render-context variables, and
  nested paths satisfied by a merged object value count as bindings.

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
- `.hermes/agents/`
- `.hermes/commands/`
- `.hermes/skills/`
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
- `hermes`: `.hermes/agents/<name>` -> `.agents/agents/<name>` -> `.claude/agents/<name>`
- `codex`: `.codex/agents/<name>` -> `.agents/agents/<name>` -> `.claude/agents/<name>`
- `gemini`: `.gemini/agents/<name>` -> `.agents/agents/<name>` -> `.claude/agents/<name>`
- `opencode`: `.opencode/agents/<name>` -> `.agents/agents/<name>` -> `.claude/agents/<name>`

Default runtime search order for commands:

- `claude`: `.claude/commands/<name>` -> `.agents/commands/<name>`
- `hermes`: `.hermes/commands/<name>` -> `.agents/commands/<name>` -> `.claude/commands/<name>`
- `codex`: `.codex/commands/<name>` -> `.agents/commands/<name>` -> `.claude/commands/<name>`
- `gemini`: `.gemini/commands/<name>` -> `.agents/commands/<name>` -> `.claude/commands/<name>`
- `opencode`: `.opencode/commands/<name>` -> `.agents/commands/<name>` -> `.claude/commands/<name>`

Default runtime search order for skills:

- `claude`: `.claude/skills/<name>/` -> `.agents/skills/<name>/`
- `hermes`: `.hermes/skills/<name>/` -> `.agents/skills/<name>/` -> `.claude/skills/<name>/`
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
- `bead {render|validate|preview-pour|pour|preview-attach|attach} --request <JSON> --json`
  (`preview-attach` / `attach`: see FR-23)
- `frontmatter-init`
- `template-init`
- `init`
- `verify`
- `extract`
- `observability-health`
- `examples`
- `templates`
- `reports`
- `help` (see FR-22)

The CLI must support:

- `--mode <profile|file>`
- `--kind <agent|command|skill>`
- `--agent <name>`
- `--agent-type <name>` as an alias for `--agent`
- `--runtime <claude|hermes|codex|gemini|opencode>` as an optional runtime selector
- `--ai <claude|hermes|codex|gemini|opencode>` as an alias for `--runtime`
- `--var key=value` repeatably
- `--var-file <path|->`
- `--env-prefix <PREFIX_>`
- `--strict`
- `--unknown-var-mode <error|warn|ignore>`
  - controls both extra caller-provided variables and referenced-but-unbound
    variables; the CLI maps the selected mode to both policy axes.
- `--root <path>`
- `--file <path>`
- `--output <path>` where applicable
- `--append <path>` on `render` only: appends the rendered JSON object as one
  compact line to a JSONL file (see `render` below); mutually exclusive with
  `--output` and `--dry-run`; on `examples` or `templates` it is a usage error
  (exit `3`)
- `--guidance <text>`
- `--guidance-file <path|->`
- `--prompt <text>`
- `--prompt-file <path|->`
- `--json`
- `--dry-run`
- `--force` where applicable
- `--pass N` repeatably where applicable
- `--against <path>` where applicable
- `--quiet` where applicable
- `--builtin-var KEY=VALUE` repeatably where applicable

Command behavior:

- `render`
  - renders one resolved template or profile,
  - writes to stdout by default,
  - may write to a file when requested,
  - must honor validation and strictness policy,
  - accepts optional guidance and user prompt blocks,
  - with `--append <path>`, renders and validates first, requires the output
    to be one JSON object, then appends it as one compact JSON line plus
    `\n` under an exclusive file lock. A non-empty destination whose last
    byte is not `\n` is refused unchanged. Every failure before the write
    leaves existing content byte-identical (a destination that did not exist
    may remain as an empty file after an open or lock failure); a write that
    fails part-way is truncated back to the pre-write length, or reported as
    possibly leaving a partial last line if that truncation also fails.
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
- `template-init`
  - converts a concrete file into a templated file using one or more pass
    groups declared with `--pass N`,
  - accepts `--var key=value` replacements within each pass group,
  - accepts `--force` to overwrite an existing frontmatter/template header,
  - supports `--dry-run` without writing the rewritten file,
  - exits with code `3` when declared literal values are not found in the
    source file because that outcome is a usage/configuration failure rather
    than a successful drift result.
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
- `reports`
  - supports:
    - `reports init`
    - `reports smoke`
    - `reports finalize`
    - `reports render-spec`
    - `reports index`
    - `reports verify`
    - `reports publish-manifest`
  - owns the shared reporting runtime surface rather than repo-specific
    producer command bodies,
  - keeps publish upload and browser-open behavior outside the command family.
- `verify`
  - compares a deployed file against the rendered output of `--against
    <template>`,
  - supports pass-scoped `--pass N` groups with per-pass `--var` and
    `--var-file` inputs when `--all` is used,
  - accepts `--builtin-var KEY=VALUE` for deterministic builtin overrides,
  - accepts `--quiet` to suppress diff body output while preserving exit
    status,
  - exits `0` when clean, `1` when drift is detected, and `2` or `3` for
    genuine validation/render or usage/configuration failures.

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
- Sequence values in variable files may contain scalars, objects, arrays, and
  any finite combination of those values at any nesting depth.
- Historical H2 note (2026-07-29): [ADR-E1](architecture.md#61-adr-e1-recursive-structured-input-contract-2026-07-29)
  supersedes this restriction; the current phase index names the
  recursive-input implementation Sprint E.1 to avoid colliding with completed
  Phase D identifiers.
- Arrays of objects are valid per FR-13 when the array is the variable value
  itself.

### FR-7b: Exit Codes

CLI exit codes must be:

- `0` for success
- `1` reserved exclusively for `verify` when drift is detected after a
  successful comparison run
- `2` for validation or render failure
- `3` for usage, configuration, or contract error

All other commands, including `template-init`, continue to use only `0`, `2`,
and `3`.

`render --append` failures exit `2`: `ERR_RENDER_JSON_MALFORMED` (output is
not JSON), `ERR_RENDER_JSON_DEPTH_LIMIT` (more than 127 nested JSON objects or
arrays), `ERR_RENDER_APPEND_NOT_OBJECT` (JSON but not one object),
`ERR_RENDER_APPEND_NO_FINAL_NEWLINE` (non-empty destination not ending in
`\n`) and `ERR_RENDER_WRITE` (lock or write failure). Its usage errors exit `3`.

`bead preview-attach` / `bead attach` and graph-mode `bead preview-pour` /
`bead pour` (FR-23): `0` succeeded; `2` for a refused or failed receipt (every
`BEADS_GRAPH_*` code); `3` for request errors.

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
- Checked rendering must expose a state-shaped result with the states
  `static_only`, `contract_invalid`, `context_required`, `render_invalid`, and
  `render_checked`. Only `render_checked` authorizes a caller to send or cache
  the exact checked output.
- `render --json` must place checked-render parser failures in the standard
  `DiagnosticEnvelope`; it must not print a malformed body as a successful
  payload.

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

`render --append --json` uses the `render --json` payload with
`output_path` = the destination, `bytes_written` = the bytes appended
(including the final `\n`), `appended: true`, and no `body`.

`bead ... --json` emits the `sc-compose/beads/v1` receipt (ADR-0021). The
receipt additions of FR-23 (`pour_mode`, `graph`) are optional fields, omitted
when absent.

`render --json`

```json
{
  "schema_version": "1",
  "payload": {
    "output_path": "stdout",
    "bytes_written": 123,
    "template": "path/to/template.md.j2",
    "body": "rendered document text"
  },
  "diagnostics": []
}
```

Schema rules:

- `output_path` is a string and uses `"stdout"` when no file is written.
- `bytes_written` is the actual byte count written to the selected output
  target; when writing to stdout it is the UTF-8 byte length emitted to stdout.
- `template` is the resolved template path as a string.
- `body` is present only for non-dry-run stdout renders and contains the full
  rendered document. It is omitted when `--output <file>` is supplied because
  the file is the source of truth.
- For JSON templates, ordinary render performs the complete-body parser gate
  before producing this payload. On parser failure the payload is `{}` and
  `diagnostics` contains `ERR_RENDER_JSON_MALFORMED`; no body or file is
  emitted. A successful checked JSON render includes:

  ```json
  "render_check": {
    "state": "render_checked",
    "template": "path/to/assignment.json.j2",
    "output_format": "json",
    "json_escape_mode": "auto",
    "checked_context": "caller-defined exact context summary",
    "diagnostics": []
  }
  ```

- `render --check-render` adds the same `render_check` object for text output
  while preserving the existing body/output behavior after a successful gate.

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

`template-init --json`

```json
{
  "schema_version": "1",
  "payload": {
    "template_path": "path/to/template.md",
    "template_added": true,
    "would_change": true,
    "vars": ["task"]
  },
  "diagnostics": []
}
```

Schema rules:

- `template_path` is the rewritten template path as a string.
- `template_added` is `true` when the command wrote a changed template to disk.
- `would_change` records whether the generated template differed from the
  original file content.
- `vars` is the ordered list of discovered variable names used in the rewrite.

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

Schema rules:

- Plain `validate` is static-only and includes
  `"state": "static_only"` alongside `valid`; it does not render.
- `validate --check-render` renders in memory and returns one of the
  checked-render states. It never includes a rendered body or output-file
  field. Only `render_checked` has permission to send or cache the exact
  context-specific result.
- `validate --lint --check-render` combines lint and render diagnostics in the
  same envelope.
- `validate --lint` reports source locations and stable mode-lint codes. An
  auto-mode contract error returns exit code `2`; warning-only legacy or
  ambiguous-expression findings preserve exit code `0`.

### FR-8b: Repository template-contract lint

- `sc-compose lint --target template-contracts --json` uses the same
  library-owned source scanner as `validate --lint` and reports the effective
  mode, template path, source location, diagnostic code, migration
  recommendation, and `context_backed_render` state for every finding.
- The target is allowlisted through `.sc/sc-lint/targets/template-contracts.toml`
  and remains a local sc-compose capability; it must not duplicate scanner
  logic in Python or shell. Missing roots, unreadable templates, and include
  failures are explicit configuration failures, never green passes.

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
- The current follow-on observability uplift targets `sc-observability`
  `1.5.0` and `sc-observability-types` `1.5.0`, both pinned exactly from
  crates.io with `default-features = false` so the v1 facade is disabled.
- Logger construction shall use `sc_observability::v2::Logger` and typed sink
  registration. Builder and build failures shall return `CommandError::usage`
  (exit 3) with context naming the configured log root.
- The CLI lifecycle adapter shall prefer `Logger::log(...)` for blocking queue
  admission and may use `Logger::try_log(...)` only where non-blocking
  admission is explicitly required.
- `Logger::emit(...)` remains a deprecated compatibility path only; any
  retained use must carry an explicit compatibility rationale in
  `docs/migration-notes.md`.
- The CLI shutdown path shall use the v2 `Logger::shutdown(&self)` result while
  preserving post-shutdown health inspection on the retained logger. Shutdown
  failures shall remain visible in health without aborting command completion.
- `sc-composer` must emit composition pipeline events through its local
  observer/sink hook model.
- `sc-compose` must emit command lifecycle events through the same local hook
  model.
- Standalone defaults must keep `sc-compose` sink paths tool-scoped.
- Embedded use must permit host-supplied sink and path configuration.
- If no sink is injected, both crates must remain fully functional with
  observability reduced to a no-op.
- `sc-compose` shall keep direct `sc-observability` logger construction and
  sink registration rather than adding the `sc-observe` facade at the CLI seam.
- `sc-observe` and `sc-observability-otlp` remain out of scope for the initial
  release.

### FR-10: Library Log-Sink Injection

- `sc-composer` shall define its minimal observability hook layer locally in
  `sc_composer::observer`.
- The library hook surface shall remain a local sink/observer abstraction over
  `ObservationEvent` rather than importing observability contracts from
  `sc-observability-types`.
- `Renderer::new()` shall remain a pure renderer constructor with no observer
  dependency.
- `compose()` shall preserve no-op behavior when the caller does not provide an
  observer implementation.
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
  dropped-event counts, retained-log maintenance state, and the active log
  path.
- The `observability-health` command shall initialize logger configuration the
  same way as a normal CLI process, query health from that process-local
  logger instance, and must not depend on any daemon or background runtime.
- The CLI logger configuration shall keep logger-managed retained-log
  maintenance enabled through `RetainedLogPolicy::default()` so rotation,
  pruning, and maintenance cadence stay owned by `sc-observability` rather
  than duplicated in `sc-compose`.
- The CLI shall perform graceful logger shutdown on process exit so pending
  events flush before termination.

### FR-16: Known-Template Reverse Extraction (v1.1, Phase G.1)

This versioned requirement establishes the Phase G reverse-extraction
contract. The contract is defined by
[ADR-0011](adrs/0011-reverse-extract-known-template-contract.md).

- Reverse extraction accepts a known template and its rendered output as
  in-memory text; the core library performs no file I/O, CLI parsing, or
  network access for this operation.
- The first supported format is XML, and only a documented reversible XML
  subset is in scope. Extracted values are rendered strings, not reconstructed
  typed values.
- Reports retain structural occurrence paths, source evidence, confidence in
  the closed range `0.0..=1.0`, and structured diagnostics. Repeated variable
  occurrences that cannot be identified unambiguously must produce an
  ambiguity diagnostic rather than silently overwrite a value.
- The initial feature explicitly does not identify unknown templates or
  reconstruct loops, branches, JSON, Markdown, or source value types.
- Invalid requests, malformed XML, unsupported syntax, and ambiguous
  structure remain distinct error categories with stable diagnostic codes.
- A dotted expression (for example `{{ user.name }}`) is object-field access,
  not a literal variable identifier. Extraction only supports the scalar
  (flat) variable subset and has no object/nested-value extraction capability,
  so any dotted expression is unsupported syntax and must be rejected with
  `ERR_EXTRACT_UNSUPPORTED` rather than accepted as a literal variable name
  (Phase G.7).

The CLI adapter exposes the same known-template contract through the
read-only command `sc-compose extract TEMPLATE RENDERED [--format xml|json]`.
It supports repeatable `--include NAME` and `--exclude NAME` filters, uses XML
as the backward-compatible default, and accepts `--json` for the standard
diagnostics envelope. The command must not identify unknown templates, invoke
the renderer, scan directories, or write output files.

This is a from-scratch product capability informed by prior reverse-extraction
research. The committed Rust, Python, and CLI corpus is the regression
evidence for the supported known-template/XML-first contract; the earlier
research harness is not a product interface or runtime dependency.

### Phase-H Extension Planning Boundary

Issue #193 records real customer use cases for JSON, YAML, and TOML rendered
output, XML mixed-content blocks, and narrowly defined non-XML prefixes before
rendered XML. Phase H is deliberately limited to the three file-format
extensions: JSON, YAML, and TOML. The XML mixed-content and dirty-prefix
findings remain outside Phase H and are owned by
[Phase I](phase-I/phase-I-plan.md).
[ADR-0012](adrs/0012-phase-h-reverse-extraction-extension-gates.md) and [the
Phase-H plan](phase-H/phase-H-plan.md) require each in-scope format to receive
explicit semantics, diagnostics, and cross-surface tests before
implementation. H.1 is now accepted: its JSON/YAML/TOML contract, diagnostic
inventory, and shared `match_raw_text` core are the binding Phase-H design.
The Phase-G XML scalar-only contract and its fail-closed malformed-input
behavior remain authoritative for existing XML behavior, while H.2 through
H.8 remain gated on their own implementation, hardening, and closure criteria
before runtime behavior changes are claimed.

The accepted H.1 design also plans the migration of the format-neutral
value-matching logic from the current XML extraction path into one shared
internal raw-text matching core. XML structural traversal and format-specific
provenance remain
owned by XML; delimiter scanning, template-segment parsing, static
prefix/suffix matching, capture boundaries, and adjacent-variable ambiguity
handling are shared operations. JSON, YAML, and TOML must delegate to that
core rather than implement independent text matchers. This is an internal
architecture seam in Phase H; Phase I.2 promotes it to a customer-facing
raw-text feature without changing the shared matcher.

#### Phase-H Boundary and Phase-I Follow-on

Phase H did not expose either mode. Phase I.1 now accepts the customer-facing
cross-format raw-text mode for known templates, including Markdown, and Phase
I.2 owns its implementation. A best-effort/degraded-parse mode that recovers
values from structurally modified or partially corrupt documents remains
future work beyond Phase I and must not be inferred from the raw-text contract.

#### Phase-H Closure Evidence

The in-scope issue #193 extensions are closed by the committed H.2-H.8
implementation, hardening, cross-surface tests, and bounded campaign evidence in
[`docs/phase-H/evidence/h-6-cross-format-campaign.json`](phase-H/evidence/h-6-cross-format-campaign.json).
The H.6 campaign covers JSON, YAML, and TOML with 36/36 expected outcomes,
including malformed and unsupported inputs as intentional boundaries. This is
bounded local evidence from four workers because Agent Runner was unavailable;
it is not evidence of a distributed adversarial-agent campaign. XML
mixed-content extraction and dirty-prefix stripping remain outside Phase H and
are owned by Phase I; template identification remains future-phase work. They
are not silently treated as H.6 failures.

### Phase-I Extension Requirements

Phase I.1 accepts the following follow-on requirements. They remain separate
from the completed Phase-H delivery and are implemented by the numbered Phase-I
sprints.

### FR-17: Customer-Facing Raw-Text Extraction (Phase I.2)

- `ExtractFormat::Raw`, CLI `--format raw`, and Python `format="raw"` shall
  select one known-template, in-memory raw-text matcher.
- Raw mode shall reuse the shared H matcher and generic extraction report;
  it shall not parse structured formats, identify unknown templates, execute
  Jinja, reconstruct loops, or infer source types.
- Raw occurrences shall use `RawPathSegment` byte offsets and one-based
  line/column evidence, with `RawExtractionSource::TextSpan` provenance.
- Include/exclude filters shall retain the existing request semantics and
  filtered variables shall still participate in neighboring capture matching.
- The stable raw diagnostic set is
  `ERR_EXTRACT_INVALID_REQUEST`, `ERR_EXTRACT_TEMPLATE_UNSUPPORTED`,
  `ERR_EXTRACT_AMBIGUOUS`, and `WARN_EXTRACT_LOW_CONFIDENCE`.

### FR-18: XML Block and Mixed-Content Extraction (Phase I.3)

- A known XML element with one full-content placeholder may recover rendered
  text and approved child markup using deterministic canonical child
  serialization.
- Multiple placeholders, dynamic names, control-flow reconstruction,
  unmatched/truncated markup, multiple roots, post-root content, and unknown
  template identification remain unsupported.
- XML paths, source evidence, ambiguity handling, limits, CLI JSON, and Python
  reports shall remain consistent with the existing extraction model.

### FR-19: XML Dirty-Prefix Recovery (Phase I.4)

- Rendered XML may contain a leading UTF-8 text/whitespace preamble before one
  XML document. Complete comments and processing instructions in the retained
  prolog are allowed; an XML declaration is retained only when first in that
  prolog.
- Only bytes before the selected root may be removed. The report shall emit
  `WARN_EXTRACT_DIRTY_PREFIX_STRIPPED` with the removed span.
- Unmatched/truncated prefix markup, malformed suffixes, multiple roots,
  second documents, post-root content, and DTDs shall remain rejected.

### FR-20: Jinja Loop-Context Built-ins (Phase I.5)

- Strict token discovery shall treat `loop`, `loop.index`, `loop.index0`,
  `loop.revindex`, `loop.revindex0`, `loop.first`, `loop.last`,
  `loop.length`, `loop.depth`, `loop.depth0`, and `loop.cycle(...)` as
  implicit only inside an active `for` scope.
- Nested scopes shall be independent. A `loop` reference outside a `for` and
  arbitrary dotted names shall remain subject to normal undeclared-token
  policy.

### FR-21: YAML Merge-Key Var-File Safety (Phase I.6)

- YAML merge keys (`<<`) in JSON/YAML var-files shall fail closed with
  `ERR_CONFIG_VARFILE` before tagged-value unwrapping; the diagnostic shall
  identify the source line and column of the unsupported construct.
- The implementation shall not partially expand merge keys or silently lose
  inherited fields. The diagnostic shall direct callers to expand the mapping
  explicitly, which is the portable recovery.
- Valid JSON/YAML objects and existing duplicate-key, non-string-key, and
  value-shape policies shall remain unchanged.

### FR-22: CLI Conceptual Help Manuals

- The CLI shall provide a `sc-compose help [topic]` command distinct from
  clap-generated `--help`. `sc-compose help` with no argument shall print an
  index of available topics; `sc-compose help --list` shall print the same
  index in a stable, scriptable form.
- Each topic shall be a static, versioned manual page shipped inside the
  binary (no filesystem lookup, no network access) covering one CLI feature
  area: at minimum `render`, `resolve`, `validate`, `verify`, `extract`,
  `template-init`, `frontmatter-init`, `init`, `examples`, `templates`,
  `reports`, `observability-health`, and `exit-codes`. The `exit-codes` topic
  shall document the FR-7b contract (`0`/`1`/`2`/`3`) in full, since that
  contract is otherwise only discoverable by reading `docs/requirements.md`
  in the repository.
- Topic content shall be authored as one Markdown file per topic under
  `crates/sc-compose/docs/manual/` (e.g.
  `crates/sc-compose/docs/manual/render.md`), each embedded into the
  binary at compile time via a single canonical registry (one ordered
  `(topic_name, content)` array), not as hand-written Rust string constants
  and not as separate per-topic Rust modules. This registry is the single
  seam every topic-content change touches; the dispatch/command-handling
  code that reads it stays flat and topic-count-independent, so it cannot
  grow unbounded as topics are added. A scaffolding change must establish
  this registry mechanism and structure before any per-topic content is
  added, so that concurrent per-topic contributions only ever add a
  Markdown file plus one registry entry, never new Rust modules or
  dispatch branches.
- An unknown topic name shall fail closed with a usage-class exit code (per
  FR-7b, exit `3`) and shall list the valid topic names in its error output.
- The root parser shall disable clap's automatically registered `help`
  subcommand while retaining the generated `--help` flag, then register this
  conceptual `help` command explicitly. This keeps `sc-compose help` as one
  unambiguous route to the manual system and prevents clap's generated
  subcommand from colliding with it.
- Topic names are scoped to the explicit `help` command, not the root command
  namespace. A topic may therefore have the same name as a real command (for
  example, `sc-compose help render` displays the `render` manual while
  `sc-compose render` runs the renderer); topic lookup must not shadow or
  reinterpret any root command.
- `help` and `help --list` shall accept `--json` and use the versioned
  `DiagnosticEnvelope` transport. In text mode, `help --list` is a UTF-8,
  newline-delimited sequence containing exactly one canonical topic name per
  line, in registry order, with no labels or indentation; this explicit
  line-oriented schema is the stable shell-pipeline form. In JSON mode, its
  payload is `{ "topics": ["..."] }` in the standard envelope. A topic JSON
  response uses `{ "topic": "...", "manual": "..." }` as its payload.
- “Versioned” means the Markdown bytes are committed under
  `crates/sc-compose/docs/manual/`,
  embedded into the same released binary as the CLI, and identified by that
  binary's `sc-compose --version`; there is no independently fetched page or
  separate page-version field. A behavior change must update the source page
  and the corresponding CLI release together.
- The `help_topics` module and its ordered registry are exclusively owned by
  `sc-compose`; `sc-composer` and the Python bindings must not define, import,
  or mutate manual-topic metadata.
- Manual content shall be reviewed for drift against the corresponding
  command's actual flags/behavior whenever that command's CLI surface
  changes; this is a documentation-accuracy expectation, not an automated
  gate.
- The root `sc-compose --help` output shall end with a line pointing callers
  to `sc-compose help` for the full manual/topic index, so the manual system
  is discoverable without already knowing it exists.

### FR-23: Beads Attach and By-Path Pour

Normative contract: [ADR-0023](adrs/0023-beads-attach-and-by-path-pour.md).
User manual: `sc-compose help bead` (`crates/sc-compose/docs/manual/bead.md`).
Everything is additive to ADR-0021; existing operations, stages, fields, codes
and `bd` argv are unchanged, except that `bd cook` reads a private
`.sc-compose-input-*` snapshot of the rendered text instead of the requested
path (same command and flags; receipts and diagnostics show the public path;
see ADR-0023 Errata).

- **FR-23.1 Rendered formulas are final.** For the graph operations below,
  structure and values come from the sc-compose template (loops, conditionals,
  includes, `{{{ ... }}}` values). No `--var` is passed to `bd`, the formula
  declares no `vars`, and Beads substitution never runs; `{{ ... }}` left in
  rendered text is kept literally.
- **FR-23.2 Render anywhere (#615).** `bead render` shall write the rendered
  formula to any existing directory inside `working_directory`; it shall not
  require a Beads `formulas/` directory to exist.
- **FR-23.3 Pour mode (#615).** `preview-pour` / `pour` of a formula in the
  active registry shall behave exactly as before (`pour_mode: "registry"`). Any
  other rendered formula inside `working_directory` shall use the graph engine
  (`pour_mode: "graph"`): a new root bead of type `molecule` plus one child per
  step and all edges, created by one `bd create --graph` transaction.
  `preview-pour` shall write nothing to Beads.
- **FR-23.4 Attach (#613).** New operations `preview-attach` and `attach` shall
  create the steps as direct children of an existing `parent`, with ids
  `<parent>.<ref>-<step>` (a `ref` may contain `-`, a step id may not, so the
  last `-` separates them) and `sc_compose_graph` provenance metadata on each
  created bead. `preview-attach` writes nothing to Beads; `attach` requires
  `CreatePersistentBeads`, like `pour`.
- **FR-23.5 Safe repetition (#613).** Re-running an attach request whose beads
  all exist with matching provenance shall write nothing (no `bd` write and no
  plan file) and succeed. Status,
  notes, assignee, claims and other fields of existing beads are never compared
  or written; sc-compose never updates, closes, reopens or deletes a bead. A
  changed formula revision, changed relations, a bead without matching
  provenance at a planned id, a conflicting edge, or a planned edge between two
  existing beads that is absent in `bd` shall refuse the whole request before
  any write. A bead counts as absent only on `bd`'s not-found response; any
  other read failure fails the request.
- **FR-23.6 Atomic apply (#613).** Everything one request creates is written
  by a single `bd create --graph` call, so an interrupted request leaves all or
  none of its missing beads and edges; a re-run creates what is missing.
- **FR-23.7 Edges.** `needs` / `depends_on` become `blocks` edges.
  `relations[]` adds edges of any well-known `bd` dependency type except
  `parent-child`, between steps and existing beads, in either direction; an
  edge from an existing bead changes nothing else about it.
- **FR-23.8 Formula subset.** Graph operations read the formula through `bd
  cook <path> --json` and accept flat steps with the keys listed in ADR-0023
  "Formula grammar". Every construct that remains in that parse is refused as
  `BEADS_GRAPH_FORMULA_UNSUPPORTED`, never ignored. `extends`, `loop` and
  `expand` are resolved by `bd` while it parses; their output is held to the
  same rules (a `loop` always yields `.` in step ids and is refused as
  `BEADS_GRAPH_ID_INVALID`). The revision is the hash of the rendered text
  only, so it never depends on `bd`'s output format; a changed `extends` base
  is not detected (ADR-0023 "Revision").
- **FR-23.9 Receipt.** Graph receipts carry `graph`: mode, parent, ref,
  formula, revision, `plan_path` (present only when there were beads to
  create), `ids` (step -> bead id) and every node and edge with its action
  (`create`/`created`/`existing`; `add`/`added`/`existing`).
- **FR-23.10 Codes.** Every `BEADS_*` code, its class (request errors exit 3,
  execution conditions exit 2) and its trigger is in
  `docs/error-code-registry.md`; the `BEADS_GRAPH_*` codes, their stages and
  exit statuses are ADR-0023 "Errors". `BeadComposeError` is
  `#[non_exhaustive]`, so adapters match it with a wildcard arm.
- **FR-23.11 bd support.** Production `bd` v1.3.1 is supported; every command
  used exists there. No Beads fork, version probe or persisted proto is used.

### FR-25: Complete Platform Delivery and Python Stable ABI (Phase U)

FR-25.1: Deliver all six macOS/Windows/Linux × x64/arm64 targets for each
of five artifact families: Rust CLI, sc-compose wheel, sc-sha wheel,
sc-composer-beads wheel, and sc-sha-go native module (30 artifact/platform
cells). Update the Go binding native target contract explicitly; an infeasible
target blocks completion rather than disappearing from the matrix.

All 30 artifact/platform cells must build successfully: the Rust CLI, the
sc-compose, sc-sha and sc-composer-beads wheels, and the sc-sha-go native module,
each for macOS, Windows and Linux on x64 and arm64.

FR-25.2: All three Python distributions use PyO3 abi3-py311, matching
cp311-abi3 tags and requires-python >=3.11, without an artificial upper bound.
Test 3.11/3.12/3.13/3.14 on all six platforms with pip and uv; this coverage
list is not a maximum supported Python version. Inspect actual limited-API
symbols/tags and execute installed imports and behavior, not filenames alone.

FR-25.3: Preserve applicable delivery channels: Rust GitHub/Homebrew/winget/
Scoop, wheels PyPI and isolated TestPyPI rehearsal, Go target ZIP modules/
GitHub assets, and crates.io source packages. Go package/schema/helper
ownership remains ADR-0018/0020/0022; use supported canonical installers,
not a local vendored fork. No public publication occurs during Phase U.

FR-25.4: Run the full integration install matrix only via workflow_dispatch,
with real Windows x64/arm64 execution on GitHub. Normal sprint CI uses the
small cached-wheel install path and current-only byte regression command.
Before publication, require a successful matrix on the final publication
candidate/version and the exact retained artifact bytes; U8's earlier upgrade
run cannot cover later U10-U13 code changes. Check every selected release
artifact's SHA-256 against trusted successful run outputs, reject missing/
wrong/stale evidence and same-candidate changed bytes, and publish those same
files without rebuilding after gate/tag. Candidate/version/bytes changes
require a new qualifying run. Evidence survives through its publication
consumer; expired/missing evidence fails closed.

Governing decision: [ADR-0025](adrs/0025-platform-delivery-and-python-abi.md).
These requirements record the user's Phase U platform, Python-floor and
integration-only matrix rulings; they do not assert implementation completion.

### FR-24: Advanced Beads Pour (Draft for Approval)

Status: **DRAFT / not approved for implementation** (2026-11-04).
Design proposal: [ADR-0024](adrs/0024-advanced-beads-pour.md).
This draft does not authorize a port or claim tests have run.

#### Purpose and pouring ownership

Users shall be able to pour formulas more advanced than bd's native formula
engine supports. **sc-compose constructs the loops, structure and variables;
bd receives flat output and validates/creates the beads.** The user has ruled
that bd variable substitution is buggy: the advanced path must not depend on
it, even when a particular bd version appears to substitute a particular form.

```text
template + structured compose inputs + bead inputs + include defaults
                         |
             sc-compose expands source structure
             and renders the chosen expressions
                         |
            flat formula / validated graph values
       (no remaining expression for bd to evaluate;
        literal brace data needs a proven safe path)
                         |
              bd validates -> bd creates
```

Today, v1.7.0 renders triple-brace composition expressions and Jinja blocks.
Ordinary double braces survive that render. Registry pour forwards scalar
`bead_variables` as `--var`; attach/by-path graph pour instead use the FR-23
flat graph engine and reject nonempty `bead_variables`. The new explicit
advanced path must render its variables itself; these existing default paths
are not silently changed.

Sources of authority: current user rulings; [#618](https://github.com/randlee/sc-compose/issues/618),
[#785](https://github.com/randlee/sc-compose/issues/785),
[#791](https://github.com/randlee/sc-compose/issues/791); v1.7.0 behavior;
ADR-0021/ADR-0023 and FR-23. Historical commits are design/test evidence, not
approval of their strict-only policies or changed defaults.

#### bd variable semantics — observed evidence and limits

Comp ran the commands serially against one throwaway Dolt 2.3.1 server on
loopback port62058, not shared3308. Exact binary: **bd1.3.2 (dev), commit
407fe27a282a1b060b7a1ced5a5b464cda2c3963**, sha256
`0409db260bda64738f23a399367ffa791e8ee7a2d6b332024082f7bcee50fe6d`.
These are observations for that build, not a released-1.3.2 qualification.
The author read comp's report and command records; the author did not rerun
or mutate the server. Source evidence is comp's
`/Users/randlee/.config/atm/share/sc-compose/bd-132-probe/evidence.md` and
`commands.jsonl` plus preserved formula/graph inputs. Command paths below
abbreviate only that recorded throwaway workspace.

| ID | Exact command form on the recorded build | Observed result | sc-compose consequence |
|---|---|---|---|
| BD-V1 | `bd mol pour vars-probe --var x=VALUE --json`; `bd list --all -n 0 --json`; `bd cook <vars-probe.formula.toml> --mode=runtime --var x=VALUE --json` | Two pours/runtime cook substitute both tight/padded forms in title/description/notes and root description; labels remain `tight-{{x}}` / `padded-{{ x }}`. | Advanced mode must render supported fields itself; native field behavior is not uniform. The old padded-title defect is NOT reproduced here. |
| BD-V2 | `bd mol pour vars-probe --var x=VALUE --var y=OVERRIDE --json`; runtime cook with the same values; cook/pour without required x | Default y=DEFAULT works, explicit y=OVERRIDE wins in actual stored output; missing required x exits1 (`missing required variables: x` for pour). | Native `[vars]` is not sc-compose frontmatter; never use its fallback/substitution to implement advanced values. |
| BD-V3 | `bd cook <undeclared.formula.toml> --mode=runtime --var z=EXTRA --json`; `bd mol pour undeclared --var z=EXTRA --json` | Both exit0; undeclared `{{ z }}` becomes EXTRA in cooked/stored title. Extra unused undeclared keys are accepted. | Declaration policy belongs to an explicit sc-compose contract, not assumed bd rejection. This observation is not automatically an upstream bug. |
| BD-V4 | `bd cook <literals.formula.toml> --mode=compile --json`; runtime cook and `bd mol pour literals --var key=ALPHA --json` | Compile/no-var pour preserves sample text; runtime cook without key exits1; supplying key changes tight/padded to ALPHA and triple `{{{ key }}}` to `{ALPHA}`, while `{% if key %}` stays literal. Three supplied-value pours agree. `x={{y}}` stayed literal in the tested insertion sample, not a general safety guarantee. | No native substitution in advanced mode; triple braces are not a bd literal escape. |
| BD-V5 | Recovered sc-compose `bead render --request <sc-request.json> --json`, then `bd mol pour sc-probe --var x=VALUE --json`; `bd cook <flat.formula.toml> --mode=runtime --var x=EXPANDED --json` | Legacy render leaves both ordinary forms; native pour substitutes title. Cook leaves input-file SHA unchanged but changes `literal {{ x }}` to `literal EXPANDED` in returned data. | Neither an unchanged file nor successful cook proves literal-safe returned values. Do not send rendered advanced values back through runtime vars. |
| BD-V6 | `bd create --graph <graph.json> --json`; `bd create --graph <attach-graph.json> --dry-run --json`, then apply/read; recovered `bead preview-attach` / authorized `attach` | Graph title/description/notes/labels preserve tested tight/padded/triple/control strings. Dry-run changes no ids. Actual sc-compose attach preserves the tested literals after a valid source raw block; an earlier unwrapped invalid-Jinja source failed rendering and is not a graph result. | Graph handoff is a supported candidate literal-safe route for these samples; exact broader escaping/field coverage is still an acceptance obligation. |
| BD-S1 | Compile cook on extends/loop/expand; runtime cook `--var deploy=false`; actual `bd mol pour condition --var deploy=false --json`; gate pour | Compile expands inheritance, loop ids `iterate.iter1.work`/`iterate.iter2.work` and expansion `check`. Runtime cook retains false-condition deploy while actual pour omits it; gate field survives cook and native pour creates a gate node. | Advanced structure belongs to sc-compose. Source/doc alignment for the condition example is unconfirmed; do not claim all native features or exact cook/pour equivalence. |

Remaining unknowns: arbitrary nested/escaped data, every native field/feature
combination, native Windows/Linux runs, released binary qualification and the
new advanced implementation. Repeated samples are not exhaustive proofs.
`mol pour --help` alone specifies only `--var key=value  Variable substitution`.

#### Known bd variable-substitution defects / candidate upstream reports

**KBD-1 — historical padded placeholder mismatch (#618), not reproduced on the
recorded1.3.2-dev build.** Minimal report: declare required x; use title
`tight {{x}}`, description `padded {{ x }}`; run `bd mol pour <name> --var
x=ALPHA` and read the bead. Expected: both substitute. Historical bd1.3.1-dev
report/correction: tight substitutes, padded stays literal. Current BD-V1:
both substitute in title/description/notes. Do not call the old defect
reproduced on1.3.2. FR-24.1/.3/.7 avoid depending on either native result.

**KBD-2 — observed field inconsistency (candidate upstream bug).** On recorded
bd1.3.2-dev, vars-probe declares required x and has title `tight={{x}}
padded={{ x }}` plus labels `["tight-{{x}}", "padded-{{ x }}"]`. Command:
`bd mol pour vars-probe --var x=VALUE --json`, then `bd list --all -n 0 --json`.
Expected for the advanced-pour capability: all supported field values are
materialized. Observed: title/description/notes substitute; labels retain
both expressions in two pours and runtime cook. Native help does not define
uniform field coverage, so this is a concrete incompatibility plus candidate
bug, not an asserted upstream specification violation. FR-24.3/.7/.8 render
fields in sc-compose and verify real stored labels.

**KBD-3 — literal triple-brace corruption under native substitution.** On the
same build, literals formula title includes `{{{ key }}}`; `bd mol pour
literals --var key=ALPHA --json` and runtime cook produce `{ALPHA}` rather
than the intentionally literal input. Repeated runtime cooks/three pours agree.
Expected for advanced literal-data preservation: original braces/text survive.
This is not a claim bd promises triple braces as escaping. FR-24.4/.5/.7/.8
require no delegated substitution plus a tested literal-safe route.

**KBD-4 — cook/pour condition difference (needs source alignment).** Runtime
cook of the local condition example with deploy=false retains deploy; native
pour omits it. The example may not match dev commit407fe27's documentation;
expected native semantics need matching upstream source/doc before calling it
a bug. FR-24.1 owns advanced conditions rather than depending on that behavior.

These are candidate upstream reports for the user to decide. No issue has
been filed. Detailed repro inputs, command labels, exits and outputs remain
in comp's report; none is presented as an implemented advanced-mode test.

#### Numbered requirements

##### FR-24.1 — sc-compose constructs; bd validates/creates

The explicit advanced path shall finish supported loops, conditions, includes
and variable expressions before invoking bd. bd shall not be used as an
implicit second template engine for the resulting structure or values.
A brace-shaped string is data, not necessarily an unresolved expression; it
must not be discarded merely for containing braces.

- **Source:** user advanced-pour and buggy-bd rulings; FR-23.1/23.8; #618.
- **bd evidence:** BD-S1/BD-V5/BD-V6 above on the recorded1.3.2-dev binary; ADR-0023 supplies the earlier1.3.1 contract.
- **Acceptance idea:** render a loop with optional steps and dependent edges; inspect exactly what bd receives and read back the exact flat nodes/values. No leftover sc-compose loop or variable is delegated to bd.

##### FR-24.2 — unchanged legacy default and schema-less compatibility

Existing calls with no new advanced option shall retain their v1.7.0 behavior,
including schema-less rendered bytes, MiniJinja semantics, policy controls,
legacy bd argv and stable error classes, subject only to already user-approved
Phase U compatibility exceptions. This port introduces no additional default
change. The new variable-rendering behavior must
be additive opt-in; a `.j2` suffix, frontmatter or upgrade shall not silently
activate it. No strict-only global policy is introduced.

Schema-less template output must match sc-compose v1.7.0 byte for byte, except
issue-linked genuine-bug fixes recorded in
`tests/fixtures/render-regressions/approved-exceptions.json`.
Every unlisted byte difference shall fail the compatibility check.

- **Source:** hard user compatibility ruling; v1.7.0; ADR-0021/0023; #791's strict-only request is not authority to break it.
- **bd evidence:** v1.7.0 render/execute source plus BD-V5/BD-V6; no native bd behavior is changed by this requirement.
- **Acceptance idea:** unchanged Rust/CLI/Python legacy requests and a schema-less differential against v1.7.0; ordinary brace text stays literal at render time, including missing optional references and raw/comment content.

##### FR-24.3 — variables rendered before bd in the advanced path

Advanced rendering shall resolve tight and padded ordinary expressions from
caller bead inputs, alongside structured compose inputs and template control
flow. A loop-local value shall be available to expressions inside that loop;
a later render pass must not lose its scope. Titles/descriptions and every
other field supported by the chosen bd flat interface shall contain the
intended values before creation. Native formula `[vars]` is not a substitute
for this work. Input-name collisions and public selector spelling remain open
below; they must not be guessed during a port.

- **Source:** #618; user advanced-pour ruling; intended behavior in 66cfacf1.
- **bd evidence:** BD-V1/V2/V3/V5 above: actual native substitution/default/undeclared-key results on407fe27. Advanced success does not depend on them.
- **Acceptance idea:** a loop over two structured items emits two concrete step ids/titles; both `{{key}}` and `{{ key }}` receive the scalar caller value; actual bd-created fields equal the pre-bd values.

##### FR-24.4 — values are data and are never reparsed as template syntax

Supplied scalar/structured values, included default values and expression
results shall remain data when they contain `{{ ... }}`, `{{{ ... }}}`,
`{% ... %}` or comment-looking text. Output from evaluating one expression
shall not become fresh sc-compose or bd template source. This applies to
nested strings and values exposed by loops/filters, not only direct lookups.

- **Source:** f0c876ea literal-preservation intent; #618; user no-bd-substitution ruling.
- **bd evidence:** BD-V4 shows triple-brace mutation under native runtime vars; BD-V6 graph/attach preserves the tested literals on407fe27. The selected route and broader data coverage remain to be approved/proven.
- **Acceptance idea:** use values containing all delimiter/control forms, nested JSON and expression-produced braces; render, create and read back exact literal data without expansion, missing-variable errors, loss or marker leakage.

##### FR-24.5 — literal protection does not corrupt filter semantics

Ordinary filters shall operate on the intended input data, not on visible
placeholder encodings. For the historical examples, `A{BC` lowercases to
`a{bc`, has length 4 and slices to `A{B`; braces shall not inflate length,
change case or disappear. JSON/TOML escaping shall follow the established
format contract exactly once, preserving decoded field values.

- **Source:** 56be73ef and f0c876ea; #618; v1.7.0 MiniJinja/format behavior.
- **bd evidence:** BD-V5/V6 give actual407fe27 cook/graph literal readback; arbitrary quoting/nested/filter combinations remain acceptance coverage, not already proven.
- **Acceptance idea:** lower/length/slice and brace-targeting replacement cases, quotes/backslashes/newlines/non-ASCII, followed by real bd readback. A protected marker must not become a user-visible value or a filter restriction hidden in the manual.

##### FR-24.6 — include defaults have an explicit, tested contract

Advanced rendering shall consume included templates and their declared defaults
under an explicit precedence/scoping contract, preserve caller-provided values
where that contract gives them precedence, and keep default strings literal.
Missing values must not silently erase fields because a default was dropped.
Legacy include/default behavior is unchanged without the new option. Exact
root-versus-include precedence and missing-value policy are open decisions,
not facts inferred from bd defaults.

- **Source:** user's explicit include-default requirement; ADR-0021 reuse of sc-composer frontmatter/defaults; #785/#791; include-default #11 is a regression input, not an automatically ported fix.
- **bd evidence:** BD-V2/V5 observe native default/override/required behavior on407fe27; sc-compose include/frontmatter defaults are distinct and must not delegate to those native vars.
- **Acceptance idea:** root/default/caller/child-default combinations, repeated includes and a literal-brace default; compare legacy output separately, then inspect advanced pre-bd and created values.

##### FR-24.7 — bd-owned expressions stay a documented legacy boundary

The existing bd-owned path shall remain available without claiming that bd
substitution is reliable. It shall preserve ordinary expression text and the
existing registry `--var` contract. The advanced path shall not rely on that
path: any literal braces passed through must use an explicitly tested route
that bd neither substitutes, errors on nor mangles. Unverified native behavior
shall be documented as unknown, not emulated speculatively by sc-compose.

sc-compose shall offer an explicit selection that leaves ordinary formula
expressions for the bd command-line tool to evaluate. This selection shall
preserve the existing legacy default; public selector spelling remains subject
to the approved ADR-0024 interface decision.

- **Source:** user compatibility and buggy-bd rulings; v1.7.0; d754230a/cd363380/5743907a/e1c086df/5f2633ac intent, with their changed default rejected.
- **bd evidence:** BD-V1–V6 above on407fe27 plus #618's historical report: current titles accept padding but labels do not substitute; never generalize either result to all versions/fields.
- **Acceptance idea:** unchanged legacy default/explicit bd selection, including redirected registry resolution; separately prove advanced created values survive literal braces without depending on `bd --var`.

##### FR-24.8 — compose with FR-23 attach and by-path pour

When the caller explicitly selects sc-compose rendering, graph pour and attach
shall accept bead-variable inputs as data rendered by sc-compose.

An opted-in advanced operation shall retain FR-23 authorization, confinement,
stable attach ids/provenance, safe repetition, atomic graph writes, relations
and receipts. Accepting bead inputs as sc-compose data for graph/attach is a
new opt-in capability; legacy graph/attach still rejects them as before.
No value already rendered by sc-compose shall then be forwarded as `bd --var`.
The behavior for an advanced request whose output is in the active registry
is an open routing decision; it must not silently regain native substitution.

- **Source:** user flat-output ruling; FR-23/ADR-0023; #618; source distinction documented by comp's trace.
- **bd evidence:** BD-V6/BD-S1 above:407fe27 graph dry-run/apply and recovered attach preserved the tested literals; ADR-0023 supplies existing hierarchy/atomicity/grammar contracts.
- **Acceptance idea:** advanced by-path pour and attach with caller inputs, preview with no DB writes, one authorized atomic create, then repeat with existing status/notes preserved; inspect argv for absent `--var` in the advanced path.

##### FR-24.9 — consume frontmatter; strictness is explicit, not global

Frontmatter shall be removed from emitted TOML/JSON, retaining established
required/default behavior. Any new undeclared-reference enforcement shall be
explicit opt-in and limited to a template that actually declares a schema.
No schema-less template acquires a mandatory schema. Opted-in validation shall
understand the selected compose brace count and shall not treat comments,
raw bodies, Jinja tag/test/filter names or supplied literal data as variables.

- **Source:** #785 and #791 under the overriding no-strict-only user ruling; existing v1.7.0 frontmatter consumption.
- **bd evidence:** BD-V5 covers recovered rendering followed by native parsing/pour; comp separately observed body-only `formula = "a"`, exit0. That is existing behavior, not an advanced-mode implementation claim.
- **Acceptance idea:** with frontmatter, emit valid body-only formula; explicit strict triple-brace undeclared reference fails while declared/default values succeed; schema-less legacy cases and comment/raw syntax stay compatible.

##### FR-24.10 — failures have typed existing outcomes and no hidden creation

Bad request/mode/options shall use the request/usage class (exit 3); rendering
failure shall use `BEADS_RENDER_FAILED`; bd validation/graph failures retain
the ADR-0021/0023 stable codes and receipt stages (execution/refusal exit 2).
No render/validation failure may proceed to persistent creation. New error
conditions not fitting those codes require an explicit documented decision.
Preview may write its declared local artifacts but never persistent bd state;
mutation still requires `CreatePersistentBeads`.

A blocked graph-plan output path, including one occupied by a directory,
shall be refused before any bd command is invoked. This output-path preflight
rule does not prohibit bd read or validation calls needed for other graph
checks; it is not a general ban on validation reads.

- **Source:** FR-23.10, ADR-0021/0023; #785/#791; cd363380's invalid-selector request classification.
- **bd evidence:** BD-V2 records missing-required native exit1; BD-V3 accepts undeclared keys; BD-V5/V6 separate render/native/graph failures. Map actual outcomes to the existing sc-compose classes, not native exit numbers by assumption.
- **Acceptance idea:** invalid selector, missing required/default input, bad rendered formula and bd failure each show the intended code/stage and zero creation calls; permitted matching cases actually create/read beads.

##### FR-24.11 — additive API and cross-surface agreement

Existing Rust request struct literals, entrypoints and public policy fields,
Python call signatures/defaults and CLI calls shall continue to work. The new
choice shall use an additive interface, not a new required field on an
existing Rust request struct. Rust, CLI and Python opted-in operations shall
agree on rendered bytes, variable ownership, bd argv and outcomes. Exact new
API/wire/CLI syntax is open below.

- **Source:** user additive/default constraint; v1.7.0; d754230a's added public field is evidence of a compatibility hazard, not an approved API change.
- **bd evidence:** BD-V5/V6 command/created-value results are the shared adapter oracle, not adapter-specific fake substitutions.
- **Acceptance idea:** compile unchanged legacy Rust callers and run unchanged Python/CLI calls; compare equivalent explicit-mode requests through all three surfaces against the same pinned-bd result.

##### FR-24.12 — documented pouring and final regression coverage

ADR-0024 and `crates/sc-compose/docs/manual/bead.md` shall explain today's and
the proposed pipelines, ownership, each mode, worked input-to-flat-output
examples, literal/escape rules, include defaults and error behavior. Claims
about bd features/limitations shall name their version/evidence. The final
Phase U work includes the additive port, v1.7.0 differential and closing
regression exercise; earlier findings are regression inputs, never evidence
that their fixes were ported.

- **Source:** user pouring-documentation and Phase-U sequencing rulings; 61366f2a; #618/#785/#791; FR-22/FR-23.
- **bd evidence:** BD-V1–V6/BD-S1 above supplies build-qualified manual claims; broader untested values/platforms remain explicit unknowns.
- **Acceptance idea:** reviewer follows one example per supported mode through its exact bd command and created values; the closing exercise includes FUZZ-171-01..14, T-02..T-06, D-01 and include-default #11 once their original reports are available, without importing their implementation by implication. Tests use controlled inputs, not sleeps/races/load.

#### Open decisions — options and recommendations, not approvals

1. **Selector/API (needs approval).** Optional wire mode plus CLI selector and
   an additive Rust options entrypoint, OR a new versioned request envelope.
   Recommendation: additive options entrypoint, legacy default; keep the
   existing Rust request layout and old executor intact. Names are not settled.
2. **Input collisions (needs approval).** Compose values win over bead values,
   OR refuse duplicates, OR separate namespaces. Recommendation: explicit
   duplicate refusal until callers choose a declared rule; do not silently
   adopt the historical merge order merely because it existed in a commit.
3. **Advanced registry routing and literal-safe transport (needs bd probe).**
   Reuse the graph writer even for an active-registry destination, OR retain a
   proved literal-safe native formula route. Recommendation: prefer the graph
   route if BD-V6 proves final values cannot be re-expanded; do not force bd's
   substitution into the advanced path. Validate source-flatness without
   quietly adding a TOML parser/dependency or changing ADR-0023 boundaries.
4. **Include/default precedence and missing-value policy (needs approval and
   evidence).** Caller > root defaults > include defaults with scoped child
   fallback, OR explicit separate include contexts/conflict rejection.
   Recommendation: preserve caller precedence and scoped child defaults, but
   characterize v1.7.0 and include-default #11 before choosing the exact rule.
5. **Explicit schema validation interface (needs approval).** Reuse an
   explicit caller policy/strict option, OR add a dedicated per-request option.
   Recommendation: explicit per-request options; no new global strict default,
   and no implicit schema requirement when the header is absent.
6. **Literal-safe rendering strategy (needs design review).** Source-aware
   single evaluation, OR another design proving equivalent data/filter/scope
   semantics. Recommendation: evaluate original source without reparsing
   emitted text; the old sequential render/marker approach is not accepted
   merely by copying it. No implementation algorithm is authorized here.
7. **Current bd details (needs bd probe).** The completed407fe27 evidence supports only its recorded samples; released
   binaries, other platforms and broader escaping/field combinations still
   need probes before making those broader claims. The production pin remains ADR-0021/0023's declared contract unless the user approves an update; the dev probe does
   not advance it. Options: retain the declared pin and test the approved
   behavior there, or qualify a user-selected new artifact. Recommendation:
   record the chosen binary and its real-bd results before making new-version
   support claims. Confirmed candidate upstream bugs go to the user; nothing
   is filed automatically.

8. **Original regression inputs/policy (needs source/user resolution).** Original
   independent reports are not located; preserved minimized tests are at
   bdb3bdae/06fc2e2d/25e3678e, and171-10's title/repro remains missing. Options:
   recover original evidence, or obtain owner-approved equivalent requirement
   and repro. Recommendation: recover/approve before claiming coverage; do not
   invent, silently drop or mark the absent case passed. Duplicate rejection
   (171-14) and D-01's changed default are historical expectations, not current
   decisions; apply approved collision/explicit-mode rules and preserve legacy.


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
- Duplicate keys in JSON and YAML var-files must fail with
  `ERR_CONFIG_PARSE`; var-files do not use silent last-value-wins semantics.
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
- Arrays of objects may contain nested object fields and arrays, including
  jagged arrays and arrays nested inside object fields.
- Historical H2 note (2026-07-29): [ADR-E1](architecture.md#61-adr-e1-recursive-structured-input-contract-2026-07-29)
  supersedes this restriction; the current phase index names the
  recursive-input implementation Sprint E.1 to avoid colliding with completed
  Phase D identifiers.
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
- `AutoEscape::Custom("sc-compose-html")` applies to `.html.j2`, `.htm.j2`,
  `.xml.j2`, and `.xhtml.j2` templates. It automatically escapes markup and
  represents XML-illegal control bytes with the legal replacement-character
  NCR `&#xfffd;`; template authors do not need to opt in.
- Self-contained output, XHTML shape, inline CSS, and browser-viewability are
  template-author responsibilities rather than core-engine enforcement.
- Dry-run, diagnostics, validation, and output-path rules apply to HTML
  templates the same way they apply to other file-mode templates.

#### ADR-style clarification (2026-08-06, FIX-278)

The shipped renderer's filename-aware `AutoEscape::Custom("sc-compose-html")`
policy covers `.html.j2`, `.htm.j2`, `.xml.j2`, and `.xhtml.j2`. The shared
formatter escapes markup and converts XML-illegal C0 control bytes to the legal
replacement-character NCR `&#xfffd;` while leaving tab, LF, and CR intact.
This is automatic behavior and requires no author opt-in. See
[architecture §21.6](architecture.md#216-h4-wrapper-owned-orchestration-pattern)
for the corresponding boundary statement. This clarification records shipped
behavior; it does not change the existing filename-dispatch mechanism.

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

### Semantic Report-Spec Contract

`sc-compose` accepts typed TOML semantic report-spec inputs so rendered diagram
formats such as Mermaid become outputs or migration inputs rather than the
long-term source of truth.

Initial implemented report-spec kinds:

- `state_machine`
- `sql_query`

`state_machine` semantic fields:

- `kind`
- `id`
- `title`
- `states`
- `transitions`
- optional per-transition fields:
  - `event`
  - `guard`
  - `actor`
  - `effect`
- optional metadata for ownership, tags, and renderer targets

`sql_query` semantic fields:

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

Input format rule:

- semantic spec input files use TOML
- each spec file defines `[spec]`
- `sql_query` semantic fields live under `[sql_query]`

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
- shared Phase A reporting boundary rules are centralized under
  `### Report Artifact Contract (Implemented In Sprint B1)`

### Report Artifact Contract (Implemented In Sprint B1)

Sprint B1 implements reporting as a generic artifact contract, not as a
one-off HTML sprint report feature.

Planned contract shape:

- authored docs remain under `docs/`
- report source and catalog inputs live outside `docs/` under a report-specific
  tree such as:
  - `reports/catalog/`
  - `reports/specs/`
  - `reports/templates/` for repo-authored template overrides and other checked-in
    template inputs
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
- `required`
- `entrypoint`
- `metadata`

Requiredness rule:

- each catalog entry declares `required = true` or `required = false`
- `just reports` verification fails only when a report marked `required = true`
  is missing
- optional reports remain discoverable and publishable without failing shared
  verification

Planned ownership split:

- producer recipes such as `just lint`, `just test`, `just smoke`, and
  repo-specific producer commands own domain data gathering and report
  generation
- `sc-compose` owns rendering semantics where it is used as the report
  renderer
- consumer repos own domain-specific inputs, local producer surfaces, and
  publish destinations

Shared Phase A reporting boundary rule:

- network publishing remains outside the core engine
- browser-open behavior remains outside the core engine
- the artifact contract is intended to support generic lint, test, smoke,
  diagram, and custom reports through one shared metadata and filesystem shape

### Source-Driven Rendering Contract (Implemented In Sprint B3)

Sprint B3 implements a generic source-driven rendering contract for text
assets. This mechanism is not Mermaid-only.

Collection-input contract:

- source collections may be declared by glob or by another stable collection
  definition
- a collection declares which source files participate in one render-many run
- collection discovery is generic across Mermaid, SVG, Markdown, and other
  text-based assets

Metadata-extraction contract:

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

Render-many contract:

- one generated output is produced per discovered source file
- output derivation is deterministic from collection membership plus source
  identity
- aggregate templates and review tooling consume a generated manifest rather
  than ad hoc wrapper state

Generated-manifest contract:

- each source-driven run emits a manifest describing the discovered sources and
  generated outputs
- the manifest is intended for aggregate templates and review tooling
- browser automation and hosted site behavior remain out of scope for the core
  engine

Boundary rules for the source-driven line:

- the mechanism remains generic rather than diagram-format-specific
- shared Phase A reporting boundary rules are centralized under
  `### Phase A Follow-On Reporting Contract (Planning Only)`
- the scaffold owns creation of `reports/latest/smoke/`, and
  report-smoke execution writes into that prepared path rather than creating
  the output directory at runtime

### Latest/Archive Output And Reports Aggregator Contract (Implemented In Sprint B5)

Sprint B5 implements how producers write stable latest outputs, how optional
timestamped archive copies are named, and how `just reports` aggregates and
verifies generated evidence.

Output policy:

- producers overwrite the latest artifact in place at the canonical
  `reports/latest/<report-id>/...` path
- producers may also write timestamped archive copies under
  `reports/archive/<timestamp>/<report-id>/...`
- archive writes are deterministic and file-system-local

Canonical archive timestamp policy:

- timestamps use a filesystem-safe UTC form such as `2026-05-25T22-10-00Z`
- one producer run uses one stable timestamp prefix for all archive outputs
  generated in that run

`just reports` contract:

- verify required evidence exists
- summarize report status across producers
- build or refresh a combined index when the repo defines one
- print or summarize the latest report entrypoints/paths for the current latest
  report set
- optional wrapper-owned helpers may open those paths, but browser opening is
  outside the shared core contract

Verification and failure direction:

- `just reports` is a shared aggregator and verifier, not a producer that
  reruns all evidence collection
- missing required evidence causes report verification to fail
- required-vs-optional report expectations come from each shared report catalog
  entry's `required` field
- the scaffold owns creation of `reports/latest/smoke/`, and
  report-smoke execution writes into that prepared path rather than creating
  the output directory at runtime

Archive ownership note:

- archive directories are file-system-local
- archive directories may be consumer-managed
- archive directories may be gitignored
### Publish-Manifest And CI Handoff Contract

`sc-compose` emits one machine-readable handoff from generated report artifacts
to CI or wrapper-owned publication steps without moving network or hosting
behavior into the core engine.

Implemented publish-manifest contract:

- `sc-compose reports publish-manifest` writes
  `reports/latest/publish-manifest.json`
- the manifest is generated from the current report catalog plus latest
  sidecars and artifact sets, not from hard-coded per-report paths
- optional reports whose latest artifact sets are absent are skipped
- required reports whose latest artifact sets are absent make manifest
  generation fail
- artifact roles remain explicit in the manifest rather than inferred by CI

Manifest fields include:

- generated_at
- reports
- per-report report_id
- per-report kind
- per-report entrypoint
- per-report optional archive_root
- per-file role
- per-file path
- per-file intended publish destination

Identity rule:

- `report_id` must equal the canonical report catalog `id`

Ownership split:

- producers and renderers create artifacts plus manifest metadata
- CI or wrapper tooling performs upload, copy, or publication steps

Boundary rules:

- the artifact contract supports generic lint, test, smoke, diagram, and
  custom reports through one shared metadata and filesystem shape
- publish transport and hosting remain outside `sc-composer` and `sc-compose`
- machine-readable handoff is in scope; network transport is not

### Phase B Cross-Use-Case Proof Examples

Phase B implements one checked-in proof set in this repo so the shared
reporting runtime is exercised by real producer commands instead of docs only.

Implemented proof set:

- this repo ships one checked-in reference report catalog under
  `reports/catalog/reports.toml`
- this repo ships one checked-in source fixture tree under:
  - `reports/inputs/`
  - `reports/specs/`
  - `reports/smoke/`
  - `reports/vars/`
- this repo ships one reference `Justfile` producer surface for:
  - `just lint`
  - `just test`
  - `just smoke`
  - `just state-diagrams`
  - `just sql-diagrams`
  - `just reports`
  - `just reports-verify`
- generic producer-owned HTML outputs can be materialized into the shared
  report sidecar and archive shape with `sc-compose reports finalize`
- the repo proves two distinct consumer families through one shared runtime:
  - `sc-lint` style evidence reports
  - `atm-core` style diagram reports
- `atm-core` and `sc-lint` remain illustrative labels only; the template
  family key is still the runtime discriminator
- producer extension-point typing remains owned by the B1 report artifact
  runtime
- `report-evidence-summary` is a new bundled Phase B proof example
- `sprint-report-html` remains backward-compatible and stays covered by the
  shared proof harness

### Phase A Producer Recipe Contract (Planning Only)

Sprint B2 implements producer recipes as the owners of report generation.
Report generation is not centered on one catch-all `just reports` command.

Standard producer surface:

- `just lint`
- `just test`
- `just smoke`
- repo-specific producer commands such as:
  - `just state-diagrams`
  - `just sql-diagrams`
  - schema, migration, or other repo-local evidence producers

Producer contract:

- each producer command is responsible for generating the report artifacts for
  the report ids it owns
- each producer command writes evidence in the shared report artifact shape
- each producer command updates or emits the catalog/metadata entries for the
  report ids it owns
- adding a repo-specific producer command must not require changing the shared
  report aggregation or discovery contract

Boundary rules for the producer line:

- producer recipes own domain data gathering and invocation order
- `just reports` is reserved for aggregation, verification, combined-index
  refresh, and latest-entrypoint/path reporting
- optional wrapper-owned helpers such as `just reports-open` may exist locally,
  but they are not part of the shared Phase A command contract
- shared Phase A reporting boundary rules are centralized under
  `### Phase A Follow-On Reporting Contract (Planning Only)`
- the report ids owned by a producer are declared through the shared report
  catalog rather than inferred from hard-coded aggregator behavior
- the scaffold owns creation of `reports/latest/smoke/`, and
  report-smoke execution writes into that prepared path rather than creating
  the output directory at runtime

### Template-Family And Panel-Chrome Contract (Implemented In Sprint B4)

Sprint B4 implements shared template families and shared panel chrome so
report UI behavior does not need to be reimplemented per consumer repo.

Initial template families:

- lint/test/smoke evidence reports
- public API, CLI, and ICD style reports
- diagram, state-machine, and SQL-query reports

Override contract:

The authoritative override contract, shared lookup namespace, consumer
activation config, template block boundary, required template variables, and
include deferral are defined in `docs/phase-A/sprint-A5.md`.

Repo-authored versus bundled template rule:

- repo-authored template overrides live under the consumer repo's
  `reports/templates/` tree
- `shared:<family>` resolves to bundled CLI-owned assets and is not a reference
  to the consumer repo's authored `reports/templates/` tree

Shared panel contract:

- stable panel id
- title
- body content
- required copy-text action
- optional copy-JSON action
- optional fragment or open link

Ownership split:

- shared panel chrome owns panel framing and shared actions
- consumer-specific templates own the panel body content for their report
  family or repo-local override

Boundary rules:

- per-panel text copy is mandatory
- per-panel JSON copy is optional but first-class
- panel chrome remains part of shared template behavior rather than wrapper-only
  logic
- shared Phase A reporting boundary rules are centralized under
  `### Phase A Follow-On Reporting Contract (Planning Only)`

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
- Exception: the `1.3.0` change to
  `Renderer::with_delimiters(open, close) -> Result<Self, RenderError>` ships
  under the narrow carve-out documented in
  [ADR-0010](adrs/0010-renderer-delimiter-fallibility-minor-exception.md);
  this exception is limited to that one constructor change and does not weaken
  the general major-version rule.
- [ADR-0009](adrs/0009-phase-d-python-binding-parity-sequencing.md) governs
  Phase D delivery sequencing and Python parity scope; it is intentionally out
  of scope for this Section 6 stability policy.
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
- Multi-panel HTML/XHTML report expansion and wrapper-level output viewing
  behavior remain deferred to the follow-on design track in
  [docs/html-sprint-report-plan.md](html-sprint-report-plan.md)
