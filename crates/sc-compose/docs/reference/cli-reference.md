# sc-compose CLI Reference

Version 1.7.0

Generated from the CLI command tree. Do not hand-edit.

## `sc-compose`

Standalone template composition CLI

```text
Usage: sc-compose <COMMAND>
```

Detailed feature manuals ship with this CLI — run `sc-compose help` (or `sc-compose help <topic>`) to read them, starting from the exit-code contract.

### `sc-compose bead`

Render, validate, preview, or persist a Beads formula request

```text
Usage: sc-compose bead <COMMAND>
```

#### `sc-compose bead attach`

Render, validate, and persist authorized Beads graph attachment

```text
Usage: sc-compose bead attach [OPTIONS] [FORMULA]
```

**Arguments:**

- `formula` — Formula template path or registry name for direct pour

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the Beads receipt in the standard diagnostic JSON envelope
- `--render-by <RENDER_BY>` — default: `sc-compose` — choices: `sc-compose, bd` — Choose the formula renderer
- `--request <JSON>` — Complete sc-compose/beads/v1 request file
- `--strict <STRICT>` — choices: `true, false` — Enable schema-scoped undeclared-variable validation
- `--var <VARS>` — Supply a direct pour variable as key=value

#### `sc-compose bead pour`

Render, validate, and persist an authorized Beads formula pour

```text
Usage: sc-compose bead pour [OPTIONS] [FORMULA]
```

**Arguments:**

- `formula` — Formula template path or registry name for direct pour

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the Beads receipt in the standard diagnostic JSON envelope
- `--render-by <RENDER_BY>` — default: `sc-compose` — choices: `sc-compose, bd` — Choose the formula renderer
- `--request <JSON>` — Complete sc-compose/beads/v1 request file
- `--strict <STRICT>` — choices: `true, false` — Enable schema-scoped undeclared-variable validation
- `--var <VARS>` — Supply a direct pour variable as key=value

#### `sc-compose bead preview-attach`

Render, validate, and preview Beads graph attachment

```text
Usage: sc-compose bead preview-attach [OPTIONS] [FORMULA]
```

**Arguments:**

- `formula` — Formula template path or registry name for direct pour

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the Beads receipt in the standard diagnostic JSON envelope
- `--render-by <RENDER_BY>` — default: `sc-compose` — choices: `sc-compose, bd` — Choose the formula renderer
- `--request <JSON>` — Complete sc-compose/beads/v1 request file
- `--strict <STRICT>` — choices: `true, false` — Enable schema-scoped undeclared-variable validation
- `--var <VARS>` — Supply a direct pour variable as key=value

#### `sc-compose bead preview-pour`

Render, validate, and preview a Beads formula pour

```text
Usage: sc-compose bead preview-pour [OPTIONS] [FORMULA]
```

**Arguments:**

- `formula` — Formula template path or registry name for direct pour

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the Beads receipt in the standard diagnostic JSON envelope
- `--render-by <RENDER_BY>` — default: `sc-compose` — choices: `sc-compose, bd` — Choose the formula renderer
- `--request <JSON>` — Complete sc-compose/beads/v1 request file
- `--strict <STRICT>` — choices: `true, false` — Enable schema-scoped undeclared-variable validation
- `--var <VARS>` — Supply a direct pour variable as key=value

#### `sc-compose bead render`

Render a Beads formula request without invoking bd

```text
Usage: sc-compose bead render [OPTIONS] [FORMULA]
```

**Arguments:**

- `formula` — Formula template path or registry name for direct pour

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the Beads receipt in the standard diagnostic JSON envelope
- `--render-by <RENDER_BY>` — default: `sc-compose` — choices: `sc-compose, bd` — Choose the formula renderer
- `--request <JSON>` — Complete sc-compose/beads/v1 request file
- `--strict <STRICT>` — choices: `true, false` — Enable schema-scoped undeclared-variable validation
- `--var <VARS>` — Supply a direct pour variable as key=value

#### `sc-compose bead validate`

Render and validate a Beads formula request

```text
Usage: sc-compose bead validate [OPTIONS] [FORMULA]
```

**Arguments:**

- `formula` — Formula template path or registry name for direct pour

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the Beads receipt in the standard diagnostic JSON envelope
- `--render-by <RENDER_BY>` — default: `sc-compose` — choices: `sc-compose, bd` — Choose the formula renderer
- `--request <JSON>` — Complete sc-compose/beads/v1 request file
- `--strict <STRICT>` — choices: `true, false` — Enable schema-scoped undeclared-variable validation
- `--var <VARS>` — Supply a direct pour variable as key=value

### `sc-compose examples`

List or render bundled example templates

```text
Usage: sc-compose examples [OPTIONS] [NAME]
       sc-compose examples <COMMAND>
```

**Arguments:**

- `name` — Bundled example pack name to render

**Options:**

- `--append <APPEND>` — Append one rendered JSON object to a JSON Lines file
- `--check-render <CHECK_RENDER>` — choices: `true, false` — Check rendered output before emitting stdout or writing a file
- `--dry-run <DRY_RUN>` — choices: `true, false` — Report the derived output target without writing files
- `--env-prefix <ENV_PREFIX>` — Absorb environment variables that match the given prefix
- `--guidance <GUIDANCE>` — Append a guidance block after the rendered body
- `--guidance-file <GUIDANCE_FILE>` — Read the guidance block from a file or stdin
- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output
- `--output <OUTPUT>` — Write rendered output to the given path instead of stdout
- `--prompt <PROMPT>` — Append a user prompt block after the rendered body and guidance
- `--prompt-file <PROMPT_FILE>` — Read the user prompt block from a file or stdin
- `--strict <STRICT>` — choices: `true, false` — Treat undeclared referenced variables as errors
- `--unknown-var-mode <UNKNOWN_VAR_MODE>` — default: `ignore` — choices: `error, warn, ignore` — Control how extra caller-provided and referenced-but-unbound variables are reported
- `--var-file <VAR_FILE>` — Load input variables from a JSON or YAML object file
- `--var <VARS>` — Provide one explicit input variable as key=value

#### `sc-compose examples list`

List bundled example packs

```text
Usage: sc-compose examples list [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output

### `sc-compose extract`

Extract variables from a known template and rendered output

```text
Usage: sc-compose extract [OPTIONS] <TEMPLATE> <RENDERED>
```

**Arguments:**

- `rendered` *(required)* — Rendered output file (required)
- `template` *(required)* — Known template file (required)

**Options:**

- `--exclude <NAME>` — Exclude this variable; repeat for multiple names
- `--format <FORMAT>` — default: `xml` — choices: `xml, json, yaml, toml, raw` — Rendered format: xml, json, yaml, toml, or raw known-template text
- `--include <NAME>` — Only recover this variable; repeat for multiple names
- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output (format is XML by default)

### `sc-compose frontmatter-init`

Insert minimal frontmatter for referenced variables

```text
Usage: sc-compose frontmatter-init [OPTIONS] --file <FILE>
```

**Options:**

- `--dry-run <DRY_RUN>` — choices: `true, false`
- `--file <FILE>` *(required)*
- `--force <FORCE>` — choices: `true, false`
- `--json <JSON>` — choices: `true, false`

### `sc-compose help`

Show a feature manual, or list available manual topics

```text
Usage: sc-compose help [OPTIONS] [TOPIC]
```

**Arguments:**

- `topic` — Manual topic to display

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the manual response as a diagnostic JSON envelope
- `--list <LIST>` — choices: `true, false` — List available manual topics

### `sc-compose init`

Bootstrap a workspace for composed outputs

```text
Usage: sc-compose init [OPTIONS]
```

**Options:**

- `--dry-run <DRY_RUN>` — choices: `true, false`
- `--json <JSON>` — choices: `true, false`
- `--root <ROOT>` — default: `.`

### `sc-compose lint`

Run an allowlisted sc-lint target and write its report

```text
Usage: sc-compose lint [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the sc-lint envelope as machine-readable JSON
- `--root <ROOT>` — default: `.` — Repository root passed to sc-lint
- `--target <TARGET>` — default: `full` — Allowlisted sc-lint target

### `sc-compose observability-health`

Report process-local logging health

```text
Usage: sc-compose observability-health [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false`

### `sc-compose reference`

Export the CLI reference model

```text
Usage: sc-compose reference [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit the reference model in the diagnostic JSON envelope

### `sc-compose render`

Render a template or resolved profile

```text
Usage: sc-compose render [OPTIONS]
```

**Options:**

- `--agent <AGENT>` — Profile name in profile mode
- `--agent-type <AGENT_TYPE>` — Alias for --agent
- `--all <ALL>` — choices: `true, false` — Render all stacked template passes
- `--append <APPEND>` — Append one rendered JSON object to a JSON Lines file
- `--brace-count <BRACE_COUNT>` — Render with custom brace-count delimiters (for example 3 => {{{ }}})
- `--check-render <CHECK_RENDER>` — choices: `true, false` — Check rendered output before emitting stdout or writing a file
- `--dry-run <DRY_RUN>` — choices: `true, false` — Report the derived output target without writing files
- `--env-prefix <ENV_PREFIX>` — Absorb environment variables that match the given prefix
- `--file <FILE>` — Template path in file mode
- `--guidance <GUIDANCE>` — Append a guidance block after the rendered body
- `--guidance-file <GUIDANCE_FILE>` — Read the guidance block from a file or stdin
- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output
- `--json-escape-mode <JSON_ESCAPE_MODE>` — choices: `auto, legacy` — Select JSON interpolation mode; default is auto unless frontmatter declares one
- `--kind <KIND>` — default: `agent` — choices: `agent, command, skill` — Choose the profile kind in profile mode
- `--mode <MODE>` — default: `file` — choices: `profile, file` — Choose file or profile resolution mode
- `--output <OUTPUT>` — Write rendered output to the given path instead of stdout
- `--prompt <PROMPT>` — Append a user prompt block after the rendered body and guidance
- `--prompt-file <PROMPT_FILE>` — Read the user prompt block from a file or stdin
- `--root <ROOT>` — default: `.` — Workspace root for resolution and confinement
- `--runtime <RUNTIME>` — choices: `claude, hermes, codex, gemini, opencode` — Optional runtime selector in profile mode
- `--strict <STRICT>` — choices: `true, false` — Treat undeclared referenced variables as errors
- `--unknown-var-mode <UNKNOWN_VAR_MODE>` — default: `ignore` — choices: `error, warn, ignore` — Control how extra caller-provided and referenced-but-unbound variables are reported
- `--var-file <VAR_FILE>` — Load input variables from a JSON or YAML object file
- `--variable-delimiters <OPEN>` — Render with explicit variable delimiters
- `--var <VARS>` — Provide one explicit input variable as key=value

### `sc-compose reports`

Initialize and run shared report scaffolds

```text
Usage: sc-compose reports <COMMAND>
```

#### `sc-compose reports finalize`

Write shared report metadata and archive outputs for one producer result

```text
Usage: sc-compose reports finalize [OPTIONS] --report-id <REPORT_ID> --kind <KIND> --entrypoint <ENTRYPOINT>
```

**Options:**

- `--archive <ARCHIVE>` — choices: `true, false`
- `--artifact <ARTIFACTS>`
- `--entrypoint <ENTRYPOINT>` *(required)*
- `--json <JSON>` — choices: `true, false`
- `--kind <KIND>` *(required)*
- `--report-id <REPORT_ID>` *(required)*
- `--root <ROOT>` — default: `.`
- `--status <STATUS>` — default: `pass`

#### `sc-compose reports index`

Summarize latest report entrypoints and sidecars

```text
Usage: sc-compose reports index [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false`
- `--root <ROOT>` — default: `.`

#### `sc-compose reports init`

Initialize the shared reports scaffold

```text
Usage: sc-compose reports init [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false`
- `--root <ROOT>` — default: `.`

#### `sc-compose reports publish-manifest`

Write one machine-readable publish manifest for current report outputs

```text
Usage: sc-compose reports publish-manifest [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false`
- `--root <ROOT>` — default: `.`

#### `sc-compose reports render-spec`

Render one semantic diagram spec to Mermaid output

```text
Usage: sc-compose reports render-spec [OPTIONS] --spec <SPEC_PATH>
```

**Options:**

- `--archive <ARCHIVE>` — choices: `true, false`
- `--json <JSON>` — choices: `true, false`
- `--root <ROOT>` — default: `.`
- `--spec <SPEC_PATH>` *(required)*

#### `sc-compose reports smoke`

Run the shared smoke-report fixture harness

```text
Usage: sc-compose reports smoke [OPTIONS] --fixture <FIXTURE> --vars <VARS>
```

**Options:**

- `--archive <ARCHIVE>` — choices: `true, false`
- `--fixture <FIXTURE>` *(required)*
- `--json <JSON>` — choices: `true, false`
- `--root <ROOT>` — default: `.`
- `--vars <VARS>` *(required)*

#### `sc-compose reports verify`

Verify required report evidence from the catalog

```text
Usage: sc-compose reports verify [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false`
- `--root <ROOT>` — default: `.`

### `sc-compose resolve`

Resolve a profile name to a concrete template path

```text
Usage: sc-compose resolve [OPTIONS]
```

**Options:**

- `--agent <AGENT>` — Profile name in profile mode
- `--agent-type <AGENT_TYPE>` — Alias for --agent
- `--env-prefix <ENV_PREFIX>` — Absorb environment variables that match the given prefix
- `--file <FILE>` — Template path in file mode
- `--json <JSON>` — choices: `true, false`
- `--json-escape-mode <JSON_ESCAPE_MODE>` — choices: `auto, legacy` — Select JSON interpolation mode; default is auto unless frontmatter declares one
- `--kind <KIND>` — default: `agent` — choices: `agent, command, skill` — Choose the profile kind in profile mode
- `--mode <MODE>` — default: `file` — choices: `profile, file` — Choose file or profile resolution mode
- `--root <ROOT>` — default: `.` — Workspace root for resolution and confinement
- `--runtime <RUNTIME>` — choices: `claude, hermes, codex, gemini, opencode` — Optional runtime selector in profile mode
- `--strict <STRICT>` — choices: `true, false` — Treat undeclared referenced variables as errors
- `--unknown-var-mode <UNKNOWN_VAR_MODE>` — default: `ignore` — choices: `error, warn, ignore` — Control how extra caller-provided and referenced-but-unbound variables are reported
- `--var-file <VAR_FILE>` — Load input variables from a JSON or YAML object file
- `--var <VARS>` — Provide one explicit input variable as key=value

### `sc-compose template-init`

Convert a concrete file into a template using pass-scoped replacements

```text
Usage: sc-compose template-init [OPTIONS] <FILE>
```

**Arguments:**

- `file` *(required)* — Concrete file to convert into a template

**Options:**

- `--dry-run <DRY_RUN>` — choices: `true, false`
- `--force <FORCE>` — choices: `true, false`
- `--json <JSON>` — choices: `true, false`

### `sc-compose templates`

List, add, or render user template packs

```text
Usage: sc-compose templates [OPTIONS] [NAME]
       sc-compose templates <COMMAND>
```

**Arguments:**

- `name` — User template pack name to render

**Options:**

- `--append <APPEND>` — Append one rendered JSON object to a JSON Lines file
- `--check-render <CHECK_RENDER>` — choices: `true, false` — Check rendered output before emitting stdout or writing a file
- `--dry-run <DRY_RUN>` — choices: `true, false` — Report the derived output target without writing files
- `--env-prefix <ENV_PREFIX>` — Absorb environment variables that match the given prefix
- `--guidance <GUIDANCE>` — Append a guidance block after the rendered body
- `--guidance-file <GUIDANCE_FILE>` — Read the guidance block from a file or stdin
- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output
- `--output <OUTPUT>` — Write rendered output to the given path instead of stdout
- `--prompt <PROMPT>` — Append a user prompt block after the rendered body and guidance
- `--prompt-file <PROMPT_FILE>` — Read the user prompt block from a file or stdin
- `--strict <STRICT>` — choices: `true, false` — Treat undeclared referenced variables as errors
- `--unknown-var-mode <UNKNOWN_VAR_MODE>` — default: `ignore` — choices: `error, warn, ignore` — Control how extra caller-provided and referenced-but-unbound variables are reported
- `--var-file <VAR_FILE>` — Load input variables from a JSON or YAML object file
- `--var <VARS>` — Provide one explicit input variable as key=value

#### `sc-compose templates add`

Import a file or directory as one user template pack

```text
Usage: sc-compose templates add [OPTIONS] <SRC> [NAME]
```

**Arguments:**

- `name` — Optional pack name override
- `src` *(required)* — Source file or directory to import as one template pack

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output

#### `sc-compose templates list`

List user template packs

```text
Usage: sc-compose templates list [OPTIONS]
```

**Options:**

- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output

### `sc-compose validate`

Validate templates without rendering output

```text
Usage: sc-compose validate [OPTIONS]
```

**Options:**

- `--agent <AGENT>` — Profile name in profile mode
- `--agent-type <AGENT_TYPE>` — Alias for --agent
- `--all <ALL>` — choices: `true, false` — Validate all stacked template passes
- `--check-render <CHECK_RENDER>` — choices: `true, false` — Render in memory and check the exact output without emitting a body or file
- `--env-prefix <ENV_PREFIX>` — Absorb environment variables that match the given prefix
- `--file <FILE>` — Template path in file mode
- `--json <JSON>` — choices: `true, false`
- `--json-escape-mode <JSON_ESCAPE_MODE>` — choices: `auto, legacy` — Select JSON interpolation mode; default is auto unless frontmatter declares one
- `--kind <KIND>` — default: `agent` — choices: `agent, command, skill` — Choose the profile kind in profile mode
- `--lint <LINT>` — choices: `true, false` — Report redundant filter chains and other lint findings with source locations
- `--mode <MODE>` — default: `file` — choices: `profile, file` — Choose file or profile resolution mode
- `--root <ROOT>` — default: `.` — Workspace root for resolution and confinement
- `--runtime <RUNTIME>` — choices: `claude, hermes, codex, gemini, opencode` — Optional runtime selector in profile mode
- `--strict <STRICT>` — choices: `true, false` — Treat undeclared referenced variables as errors
- `--unknown-var-mode <UNKNOWN_VAR_MODE>` — default: `ignore` — choices: `error, warn, ignore` — Control how extra caller-provided and referenced-but-unbound variables are reported
- `--var-file <VAR_FILE>` — Load input variables from a JSON or YAML object file
- `--var <VARS>` — Provide one explicit input variable as key=value

### `sc-compose verify`

Verify deployed output matches a rendered template

```text
Usage: sc-compose verify [OPTIONS] <DEPLOYED>
```

**Arguments:**

- `deployed` *(required)* — Concrete deployed file to compare against

**Options:**

- `--against <AGAINST>` — Template path in file mode
- `--agent <AGENT>` — Profile name in profile mode
- `--agent-type <AGENT_TYPE>` — Alias for --agent
- `--all <ALL>` — choices: `true, false` — Verify all stacked template passes
- `--builtin-var <BUILTIN_VARS>` — Override one builtin variable as key=value for deterministic verification
- `--env-prefix <ENV_PREFIX>` — Absorb environment variables that match the given prefix
- `--file <FILE>` — Template path in file mode
- `--json <JSON>` — choices: `true, false` — Emit machine-readable JSON output
- `--json-escape-mode <JSON_ESCAPE_MODE>` — choices: `auto, legacy` — Select JSON interpolation mode; default is auto unless frontmatter declares one
- `--kind <KIND>` — default: `agent` — choices: `agent, command, skill` — Choose the profile kind in profile mode
- `--mode <MODE>` — default: `file` — choices: `profile, file` — Choose file or profile resolution mode
- `--quiet <QUIET>` — choices: `true, false` — Suppress diff output when drift is detected
- `--root <ROOT>` — default: `.` — Workspace root for resolution and confinement
- `--runtime <RUNTIME>` — choices: `claude, hermes, codex, gemini, opencode` — Optional runtime selector in profile mode
- `--strict <STRICT>` — choices: `true, false` — Treat undeclared referenced variables as errors
- `--unknown-var-mode <UNKNOWN_VAR_MODE>` — default: `ignore` — choices: `error, warn, ignore` — Control how extra caller-provided and referenced-but-unbound variables are reported
- `--var-file <VAR_FILE>` — Load input variables from a JSON or YAML object file
- `--var <VARS>` — Provide one explicit input variable as key=value
