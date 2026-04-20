# sc-compose

sc-compose is a standalone CLI for teams whose templates have outgrown copy-paste. It started with agent workflows — authoring agent profiles once and dispatching per-run dev and QA task assignments with declared inputs that fail loudly when missing — and the same machinery turned out to fit anywhere shared fragments should live in one place: pytest fixtures, .NET benchmark harnesses, HTML reports, service configs. Templates are Jinja2 with YAML frontmatter; shared fragments are pulled in by @-include; required inputs are declared up front and validated at render time. For AI agent workflows specifically, a single profile resolves across Claude Code, Codex, Gemini, and OpenCode through each runtime's native search chain — with a shared `.agents/` fallback so you only override the runtimes that genuinely need it.

**About this document.** This README explains what sc-compose is and how people use it. It is not a task prompt. Code blocks labelled as example template content are examples, not instructions for an AI agent.

---

## Install

### Homebrew (macOS)

```bash
brew install randlee/tap/sc-compose
```

Bundled examples are installed to `$(brew --prefix)/share/sc-compose/examples/` and discovered automatically.

### Winget (Windows)

```powershell
winget install randlee.sc-compose
```

### From source

```bash
cargo install --path crates/sc-compose
```

`cargo install` ships the binary only. Bundled examples are guaranteed in
Homebrew, `winget`, and GitHub Release installs. `SC_COMPOSE_DATA_DIR` can
override the examples location for CI, custom installs, and `cargo install`
users.

Or build without installing:

```bash
cargo build --release -p sc-compose
./target/release/sc-compose --help
```

---

## Common commands

| Command | What it does |
|---------|-------------|
| `render` | Render a template or resolved profile to stdout or a file. |
| `resolve` | Print the resolved profile path and search trace. |
| `validate` | Expand includes and analyze variables without writing output. |
| `frontmatter-init` | Discover referenced variables and prepend minimal frontmatter. |
| `init` | Create `.prompts/`, add it to `.gitignore`, and scan templates. |
| `examples list` | List bundled starter templates shipped with sc-compose. |
| `examples <name>` | Render a bundled example with optional `--var` / `--var-file`. |
| `templates list` | List your saved personal templates. |
| `templates add <src> [name]` | Save a file or directory to your local template store. |
| `templates <name>` | Render a saved template with optional `--var` / `--var-file`. |

---

## Library usage

For embedded hosts and programmatic use, depend on `sc-composer` directly:

```toml
[dependencies]
sc-composer = "1.0.0"
```

The crate root re-exports the main entry points — `compose`, `compose_with_observer`, `validate_with_observer`, `resolve_profile_with_observer`, `frontmatter_init`, `init_workspace` — plus request/result types and the diagnostic envelope. See `crates/sc-composer/src/lib.rs` and `docs/architecture.md`.

---

## Status

| | |
|-|-|
| Version | 1.0.0 |
| MSRV | Rust 1.94.1 |
| Rust edition | 2024 |
| Platforms | macOS, Linux, Windows |
| Stability | stable 1.0 release line |

## Documentation

- `docs/requirements.md` — normative behavior, JSON schemas, exit codes.
- `docs/architecture.md` — library module layout and the library/CLI boundary.
- `docs/error-code-registry.md` — stable `ERR_*` diagnostic codes.
- `docs/cross-platform-guidelines.md` — platform-specific behavior and testing rules.
- `docs/publishing.md` — release procedures for integrators.
- `docs/atm-adapter-notes.md` — adapter boundary and integration ownership.

Contributor references: `docs/git-workflows.md`, `.claude/skills/rust-development/guidelines.txt`.

---

## Why this exists

Prompt files drift across repos, tasks, and runtimes. Teams end up with several copies of the same prompt: `.claude/agents/foo.md`, `.codex/agents/foo.md`, a Slack paste, a gist, and a shell-history version. Those copies diverge. Agent behavior diverges with them. Debugging turns into prompt diffing.

`sc-compose` treats prompts as source code you compose, not text you copy. Compose once. Render deterministically. Keep shared fragments in one place and include them by reference. Pass task context as variables. Validate required inputs at render time so missing data fails fast instead of being guessed.

The workspace provides two crates:

- **sc-composer** — a Rust library with the render, include-expansion, validation, and diagnostics pipeline.
- **sc-compose** — a CLI wrapper over the library for scripts, shells, and agent-invocable workflows.

Both are standalone. Neither is coupled to any particular orchestration system.

---

## Mental model

The model is simple: templates, frontmatter, profiles, and outputs.

**Templates** are Jinja2 source files, usually named `*.md.j2` or `*.xml.j2`. The `.j2` suffix is stripped on render. Templates may contain Jinja variable references (`{{ task_id }}`), control flow (`{% if %}`), and sc-compose's include directive (`@<path>`, described below). A template can also exist without the `.j2` suffix. A plain `.md` file with no dynamic content is still valid.

**Frontmatter** is an optional YAML block at the top of a template, delimited by `---`:

```yaml
---
required_variables:
  - task_id
  - branch
defaults:
  pr_target: develop
metadata:
  owner: platform-team
---
```

- `required_variables` — names the caller must supply. Render fails loud if missing.
- `defaults` — scalar fallbacks used when a variable isn't otherwise provided.
- `metadata` — arbitrary descriptive data; does not affect the rendered output.

**Profiles** are templates stored under runtime-specific directories. They are looked up by name and kind, not path. `sc-compose render --mode profile --kind agent --agent rust-developer --runtime claude` resolves the profile through the Claude search chain and renders the winning file.

**Rendered outputs** go to stdout by default or to an explicit `--output` path. `sc-compose init` creates `.prompts/` and adds it to `.gitignore` for workflows that want a gitignored render directory. Given the same template, variables, include graph, and policy flags, the output is reproducible.

---

## Includes: the reuse lever

The include directive is a single line that begins with `@`, followed by a path relative to the including file or to the workspace root:

```
@_includes/house-style.md
```

At render time the directive is replaced by the target file's contents. Includes may nest. The engine tracks the chain, detects cycles, enforces a depth limit, and keeps every resolved path inside the workspace root. Included files can also declare frontmatter. Their `required_variables` merge upward into the caller-visible set, and their `defaults` apply behind any defaults the parent already declared.

This is the main reuse mechanism. Put your definition of done, review checklist, error conventions, or testing policy in one includable file. Reference it from every agent profile. Edit it once. Every downstream agent picks up the change.

---

## Why one engine across runtimes

Each runtime has its own prompt layout. Claude Code looks in `.claude/agents/`. Codex looks in `.codex/agents/`. Gemini and OpenCode have their own conventions. Without a shared resolver, copies drift. `sc-compose` resolves each runtime's search chain and a shared fallback under `.agents/`. Author a profile once under `.agents/agents/foo.md`. Override only the runtimes that need a specialized copy.

---

## Repo layout

A workspace that uses `sc-compose` typically looks like this:

```
your-repo/
├── .agents/                  # shared-across-runtimes fallback
│   ├── agents/
│   │   └── reviewer.md       # works for every runtime
│   ├── commands/
│   └── skills/
├── .claude/                  # Claude Code native layout
│   └── agents/
│       └── rust-developer.md
├── .codex/                   # Codex-specific overrides (optional)
├── .gemini/                  # Gemini-specific overrides (optional)
├── .opencode/                # OpenCode-specific overrides (optional)
├── _includes/                # shared fragments referenced via @-includes
│   └── house-style.md
├── .prompts/                 # gitignored render output
└── .gitignore
```

`sc-compose init` bootstraps `.prompts/` and adds it to `.gitignore`. Everything else is a convention. If you do not want the profile layout, place templates anywhere and render them with `--mode file --file <path>`.

Per-kind search chains used by the resolver (source of truth: `crates/sc-composer/src/resolver.rs`):

| Runtime | Agents | Commands | Skills |
|---------|--------|----------|--------|
| Claude | `.claude/agents`, `.agents/agents` | `.claude/commands`, `.agents/commands` | `.claude/skills`, `.agents/skills` |
| Codex | `.codex/agents`, `.agents/agents`, `.claude/agents` | `.codex/commands`, `.agents/commands`, `.claude/commands` | `.codex/skills`, `.agents/skills`, `.claude/skills` |
| Gemini | `.gemini/agents`, `.agents/agents`, `.claude/agents` | `.gemini/commands`, `.agents/commands`, `.claude/commands` | `.gemini/skills`, `.agents/skills`, `.claude/skills` |
| OpenCode | `.opencode/agents`, `.agents/agents`, `.claude/agents` | same pattern | same pattern |

Claude is the universal fallback because it is the most common author target in practice.

---

## Authoring a template

Two shapes come up most often: an agent profile in Markdown and a structured task template in XML or JSON. Both follow the same frontmatter, body, and include rules.

### Example: a static Markdown agent profile

Excerpt from `.claude/agents/rust-developer.md` (example template content):

```markdown
---
name: rust-developer
description: Implements Rust code changes by following project conventions
tools: Glob, Grep, LS, Read, Write, Edit, NotebookRead, Bash
model: sonnet
---

You are a senior Rust developer who implements code changes that are
idiomatic, safe, and aligned with project conventions.

MUST READ: `.claude/skills/rust-development/guidelines.txt` before making
changes. All code must conform to these guidelines.
```

This profile has no `required_variables`. It is static text. From sc-compose's perspective, its frontmatter is metadata and passes through untouched. Claude Code consumes those fields separately at load time. sc-compose still adds uniform rendering, include support, and validation.

### Example: a parameterized template with variables

Excerpt from `.claude/skills/codex-orchestration/dev-template.xml.j2` (example template content):

```xml
---
name: dev-task
required_variables:
  - task_id
  - sprint
  - description
  - worktree_path
  - branch
  - pr_target
  - deliverables
  - acceptance_criteria
---
<atm-task id="{{ task_id }}" sprint="{{ sprint }}">
  <description>{{ description }}</description>
  <worktree>{{ worktree_path }}</worktree>
  <branch>{{ branch }}</branch>
  <pr-target>{{ pr_target }}</pr-target>
  <deliverables>
{{ deliverables }}
  </deliverables>
  <acceptance-criteria>
{{ acceptance_criteria }}
  </acceptance-criteria>
</atm-task>
```

Any caller that invokes this template must provide all eight required variables. Miss one and the render fails with a diagnostic that names it.

### Example: a list-driven template

Templates accept arrays of scalars via `--var-file`. A bundled example generates pytest test stubs from a list of test names (example template content):

```python
{%- for name in test_names %}
def test_{{ name }}():
    ...
{%- endfor %}
```

Pass the list in a JSON var-file:

```json
{ "test_names": ["login", "logout", "signup"] }
```

```bash
sc-compose examples pytest-fixture --var-file vars.json --output tests/test_auth.py
```

Arrays of scalars are accepted in `--var-file`.

### Adding an include

Put a shared snippet in `_includes/house-style.md` and reference it from any template:

```
Before making changes, review the house style.

@_includes/house-style.md

Then proceed with the task described below.
```

At render time the `@_includes/house-style.md` line is replaced by that file's contents. Edit `house-style.md` once. Every template that includes it picks up the change on the next render.

---

## Bundled examples and personal templates

`sc-compose` ships with starter templates and a personal template store.

### Bundled examples

Installed alongside the binary:

```bash
sc-compose examples list           # show available examples
sc-compose examples pytest-fixture --var-file vars.json --output tests/test_auth.py
sc-compose examples service-config --var-file svc.json --output deploy/config.yaml
```

The examples directory is located automatically from the binary path (`../share/sc-compose/examples/` relative to the binary, following Homebrew and FHS conventions). Override with `SC_COMPOSE_DATA_DIR` if needed.

### Personal templates

Save and reuse your own templates:

```bash
sc-compose templates add my-template.md.j2             # save a single file pack
sc-compose templates add my-pack-dir my-pack           # import a directory pack
sc-compose templates list                              # list saved templates
sc-compose templates my-template --var-file data.json  # render
```

Templates are stored under the platform user-data root in `sc-compose/templates/`:

- Linux: `~/.local/share/sc-compose/templates/`
- macOS: `~/Library/Application Support/sc-compose/templates/`
- Windows: `%APPDATA%\\sc-compose\\templates\\`

Override with `SC_COMPOSE_TEMPLATE_DIR`.

---

## Using it from a host wrapper or session hook

`sc-compose` is usually one layer inside a larger agent workflow. This repo owns composition semantics. The host runtime owns session lifecycle, delivery, runtime-specific hooks, and transport.

Preferred flow:

1. Use `resolve` to inspect which profile wins for a runtime.
2. Use `validate` or `render --dry-run` to catch missing inputs before launch.
3. Use `render` to stdout or `--output` for the launcher, wrapper, or hook to consume.

Rendered output always uses the same block order: rendered profile body, optional guidance block, then optional user prompt.

Embedded hosts should depend on `sc-composer` and call it in-process. Scripts, CI jobs, agents, and humans can use the `sc-compose` CLI directly for local validation, debugging, non-embedded automation, or repo bootstrap tasks. Manual inspection or paste is a fallback, not the core integration path.

Session lifecycle and runtime hooks live outside this repo. Inside this repo, the integration seam is the observer/sink API in `sc-composer` plus the CLI's observer wiring in `sc-compose`.

---

## Passing task context in

Keep stable instructions in the profile. Pass per-run data through variables. Pass ephemeral task text through guidance and prompt blocks.

Three ways to get variables into a render, highest precedence first:

1. `--var key=value` on the command line (repeatable). Values are passed as strings.
2. `--var-file path.yaml` or `path.json`. Use `-` to read from stdin; useful for piping. Arrays of scalars are accepted.
3. `--env-prefix TASK_` to absorb any environment variables matching the prefix (e.g. `TASK_TICKET=ENG-4712` becomes variable `ticket`).

For named template renders, optional user-template `template.json` `input_defaults` fill in behind those three sources. Frontmatter defaults fill in behind `input_defaults`. `--strict` turns any referenced-but-undeclared variable into a hard error. `--unknown-var-mode error|warn|ignore` controls what happens to caller-provided variables the template does not reference.

---

## Authoring reference

### Frontmatter fields

| Field | Type | Purpose |
|-------|------|---------|
| `required_variables` | list of strings | Variables the caller must supply; render fails if any are missing. |
| `defaults` | map of scalar values or arrays of scalars | Fallback values used when the caller does not provide a value. |
| `metadata` | map of arbitrary YAML | Descriptive data; preserved by the renderer but does not affect output. |

Any other frontmatter field is preserved as metadata.

### Include syntax

- Directive: a line beginning with `@` followed by a path (for example, `@_includes/house-style.md`).
- Resolution order: first relative to the including file, then relative to the workspace root.
- Nested includes are supported. Cycles and depth overruns fail with a diagnostic.
- All resolved paths are confined to the workspace root. Paths that escape via `..` are rejected.
- Included-file frontmatter participates in validation: its `required_variables` merge upward, and its `defaults` apply unless overridden.

### Variable precedence

From highest to lowest:

1. `--var key=value` and entries loaded via `--var-file`.
2. Environment-derived variables via `--env-prefix PREFIX_`.
3. User-template `template.json` `input_defaults` for `sc-compose templates <name>`.
4. Parent-file frontmatter defaults.
5. Included-file frontmatter defaults, in include order.

For full semantics (including diagnostic codes, exit codes, and JSON schemas), see `docs/requirements.md`.

---

## CLI reference

Most-used flags:

| Flag | Purpose |
|------|---------|
| `--mode <file\|profile>` | Template lookup mode (default: `file`). |
| `--kind <agent\|command\|skill>` | Profile kind in profile mode (default: `agent`). |
| `--agent <name>` | Profile name in profile mode. |
| `--runtime <claude\|codex\|gemini\|opencode>` | Runtime selector; controls the search chain. |
| `--file <path>` | Template path in file mode. |
| `--var key=value` | Input variable; repeatable. Values are passed as strings. |
| `--var-file <path>` | JSON or YAML variable file (`-` reads stdin). Arrays of scalars accepted. |
| `--env-prefix <PREFIX_>` | Absorb env vars matching the prefix. |
| `--guidance <text>` / `--guidance-file <path>` | Append a guidance block after the rendered profile body. |
| `--prompt <text>` / `--prompt-file <path>` | Append a user prompt block after the guidance block. |
| `--strict` | Fail on undeclared referenced variables. |
| `--unknown-var-mode <error\|warn\|ignore>` | Handling of extra caller variables (default: `ignore`). |
| `--output <path>` | Write rendered output to a file (`render` only). |
| `--dry-run` | Report what would be rendered or written without modifying files. |
| `--json` | Machine-readable output with diagnostics envelope. |

Run `sc-compose <command> --help` for the full flag surface, or see `docs/requirements.md` for the normative specification.

Publishing to crates.io is tracked in `docs/publishing.md`.

---

## Contributing

`main` is protected. Create feature branches from `develop` and follow `docs/git-workflows.md` for branching and review rules. Adhere to the Pragmatic Rust Guidelines for code style.

## License

MIT. See `LICENSE`.
