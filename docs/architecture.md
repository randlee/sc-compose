# SC-Compose Architecture

> Status: Draft
> Product: `sc-composer` (library) and `sc-compose` (CLI)
> Document role: Normative target architecture for the redesign of both crates

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

## 2. Crate Layout

### 2.1 `sc-composer`

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

### 2.2 `sc-compose`

`sc-compose` is the CLI binary crate. It owns:

- argument parsing,
- command dispatch,
- output formatting,
- exit codes,
- file-writing UX,
- CLI-facing observability wiring.

### 2.3 Dependency Direction

Required dependency direction:

- `sc-compose` -> `sc-composer`
- `sc-compose` -> `sc-observability`

Allowed library dependency:

- `sc-composer` may depend on `sc-observability` only through a stable
  observability integration boundary appropriate for embedded consumers.

Forbidden dependency direction:

- `sc-composer` -> `sc-compose`
- `sc-composer` -> orchestration-specific runtime crates
- `sc-composer` -> mailbox helpers, daemon helpers, team-state helpers, or
  runtime-specific home-resolution helpers

## 3. Module Architecture

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
  - renders template content under normal or strict undeclared-token policy.
- `validate`
  - produces validation reports and diagnostics without writing output.
- `pipeline`
  - concatenates resolved profile, guidance, and user prompt blocks in fixed
    order.
- `diagnostics`
  - defines diagnostic types,
  - defines the JSON diagnostic schema contract.
- `workspace`
  - implements `frontmatter-init` and `init` logic for reuse by the CLI and any
    future embedded callers.
- `observability`
  - exposes event types and integration points,
  - delegates implementation to `sc-observability`.

## 4. Resolver Path Policy

Resolver policy must be data-driven and not embedded in CLI-only conditionals.

The policy model must express:

- runtime name,
- profile kind,
- ordered candidate directories,
- ordered filename probes,
- ambiguity rules when runtime is omitted.

### 4.1 Runtime-Specific Directories

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

### 4.2 Shared Directories

- `.agents/agents/`
- `.agents/commands/`
- `.agents/skills/`

There is no flat shared fallback such as `.agents/<name>`.

### 4.3 Probe Rules

For agent and command prompts, candidate probe order within a directory is:

1. `<name>.md.j2`
2. `<name>.md`
3. `<name>.j2`

For skills, candidate probe order within a directory is:

1. `<name>/SKILL.md.j2`
2. `<name>/SKILL.md`
3. `<name>/SKILL.j2`

### 4.4 Ambiguity Rules

- If a runtime is explicitly provided, only that runtime chain is evaluated.
- If a runtime is omitted, all runtime and shared roots are evaluated.
- If multiple candidates match, resolution fails with an ambiguity diagnostic.
- If exactly one candidate matches, it may be selected without an explicit
  runtime.

## 5. Frontmatter Model

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

## 6. Variable and Token Semantics

The architecture must distinguish these cases:

- declared required variable,
- declared optional variable with a default,
- undeclared referenced token,
- extra provided input variable.

### 6.1 Default Mode

In default mode:

- undeclared referenced tokens are preserved in rendered output,
- undeclared referenced tokens produce diagnostics,
- undeclared referenced tokens are not implicitly promoted to required
  variables.

### 6.2 Strict Mode

In strict mode:

- undeclared referenced tokens are fatal during validation,
- undeclared referenced tokens are fatal during rendering.

### 6.3 Missing Required Variables

Missing required variables remain a separate diagnostic class:

- they fail validation and rendering,
- they are reported with file, line and column when available, and include
  chain.

## 7. Public API Shape

The library API should expose explicit request and result types.

Required library surface:

- `resolve_profile(request) -> ResolveResult`
- `compose(request) -> ComposeResult`
- `validate(request) -> ValidationReport`
- `init_workspace(root, options) -> InitResult`
- `frontmatter_init(path, options) -> FrontmatterInitResult`

### 7.1 Core Request Types

`ComposeRequest`

- `runtime: RuntimeKind`
- `mode: ComposeMode`
- `kind: Option<ProfileKind>`
- `agent: Option<String>`
- `root: PathBuf`
- `template_path: Option<PathBuf>`
- `vars_input: Map<String, ScalarValue>`
- `vars_env: Map<String, ScalarValue>`
- `guidance_block: Option<String>`
- `user_prompt: Option<String>`
- `policy: ComposePolicy`

`ComposePolicy`

- `strict_undeclared_variables: bool`
- `unknown_variable_policy: UnknownVariablePolicy`
- `max_include_depth: usize`
- `allowed_roots: Vec<PathBuf>`
- `resolver_policy: ResolverPolicy`

### 7.2 Core Result Types

`ResolveResult`

- `resolved_path: PathBuf`
- `attempted_paths: Vec<PathBuf>`
- `ambiguity_candidates: Vec<PathBuf>`

`ComposeResult`

- `rendered_text: String`
- `resolved_files: Vec<PathBuf>`
- `search_trace: Vec<PathBuf>`
- `variable_sources: Map<String, VariableSource>`
- `warnings: Vec<Diagnostic>`

`ValidationReport`

- `ok: bool`
- `warnings: Vec<Diagnostic>`
- `errors: Vec<Diagnostic>`
- `search_trace: Vec<PathBuf>`

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

## 8. Include and Frontmatter Merge Rules

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

## 9. Diagnostics Model

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

## 10. Request Lifecycle

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

## 11. CLI Command Architecture

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

## 12. Output Path Policy

File-writing behavior should be centralized rather than duplicated per command.

Policy requirements:

- file mode strips the final `.j2` suffix,
- profile mode writes to `.prompts/<name>-<ulid>.md`,
- explicit `--output` overrides derived behavior,
- dry-run returns the same derived target information without writing files.

## 13. `init` Command Behavior

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

## 14. Safety Model

- Default deny for out-of-root file access
- No shell execution inside the composition pipeline
- No evaluation of arbitrary host code from templates
- Include stack tracked for all include-related diagnostics
- Deterministic failure semantics for path escape, missing include, cycle, and
  depth overflow

## 15. Error and Exit Semantics

The library should expose typed errors with stable categories. Validation
results should remain structured and not collapse into string-only errors.

Target CLI exit semantics:

- `0` success
- `2` validation or render failure
- `3` usage, configuration, or contract error

## 16. Observability Integration

Architecture rules:

- `sc-observability` is the canonical implementation.
- Event emission points should cover:
  - command start and end,
  - resolve attempts and outcomes,
  - include expansion outcomes,
  - validation outcomes,
  - render outcomes.
- Library observability hooks must remain usable by embedded consumers.
- Default sink paths for standalone CLI behavior must be tool-scoped.

## 17. Extensibility

The redesign should keep room for future extensions without destabilizing the
core behavior.

Expected extension points:

- typed variable schemas,
- remote include providers,
- template caching,
- custom resolver policies,
- richer frontmatter metadata consumers.
