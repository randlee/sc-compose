# sc-compose

<p align="center">
  <strong>Compose once. Render deterministically. Ship everywhere.</strong>
</p>

<p align="center">
  <a href="#install"><img src="https://img.shields.io/badge/install-brew%20%7C%20winget%20%7C%20cargo%20%7C%20pip-4c1" alt="Install: brew | winget | cargo | pip"></a>
  <a href="https://crates.io/crates/sc-compose"><img src="https://img.shields.io/crates/v/sc-compose?label=crates.io" alt="crates.io"></a>
  <a href="https://pypi.org/project/sc-compose/"><img src="https://img.shields.io/pypi/v/sc-compose?label=PyPI" alt="PyPI"></a>
  <a href="https://github.com/randlee/sc-compose/actions"><img src="https://img.shields.io/github/actions/workflow/status/randlee/sc-compose/ci.yml?branch=develop&label=CI" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
</p>

---

**sc-compose** is a standalone CLI and library for teams whose templates have
outgrown copy-paste. Compose templates from shared, version-controlled
fragments, declare inputs up front, and render deterministic output across any
runtime — AI agent profiles, pytest fixtures, .NET harnesses, HTML reports, and
service configs.

**One engine, everywhere.** A single Rust library (`sc-composer`) drives the
CLI (`sc-compose`), Python bindings, and any embedded host. Templates are
Jinja2 with YAML frontmatter. Shared fragments use `@`-include. Required inputs
fail loudly at render time — no guessing, no silent defaulting.

For AI agent workflows, one profile resolves across Claude Code, Codex, Gemini,
and OpenCode through each runtime's native search chain, with a shared
`.agents/` fallback so you override only the runtimes that genuinely need it.

---

## Quickstart

```bash
# Install
brew install randlee/tap/sc-compose        # macOS
winget install randlee.sc-compose           # Windows
cargo install sc-compose                    # from source
pip install sc-compose                      # Python

# Render your first template
echo 'Hello {{ name }}!' > hello.txt.j2
sc-compose render --file hello.txt.j2 --var name=World
# → Hello World!
```

---

## Feature Highlights

### Compose Templates from Shared Fragments

Place shared conventions in one file. Reference them from everywhere. Edit
once, every downstream template picks up the change.

```
@<_includes/house-style.md>
```

Includes nest, cycles are detected, paths are confined to the workspace root.

### Declare Inputs Up Front

YAML frontmatter makes required inputs explicit:

```yaml
---
required_variables:
  - task_id
  - branch
defaults:
  pr_target: develop
---
```

Missing a required variable? `sc-compose` fails with an actionable diagnostic
that names the missing variable, the file that declared it, and the include
chain.

### One Profile Across Four Runtimes

Author an agent profile once under `.agents/agents/`. Override only the
runtimes that need specialization:

```
your-repo/
├── .agents/agents/reviewer.md    ← works for Claude, Codex, Gemini, OpenCode
├── .claude/agents/               ← Claude-only overrides (optional)
├── .codex/agents/                ← Codex-only overrides (optional)
```

`sc-compose render --mode profile --kind agent --agent reviewer --runtime claude`

### Multi-Pass Nested Templates (v1.3.0)

Progressive resolution: deploy-time → install-time → invocation-time variables
in one file. Outer passes use more braces; inner passes use fewer. Shared
fragments (`@`-include) work at every pass.

```bash
# Render all three passes
sc-compose render config.yaml.2.j2 --all \
  --pass 3 --var-file deploy.json \
  --pass 2 --var-file install.json \
  --pass 1 --var-file invoke.json

# Verify deployed config hasn't drifted
sc-compose verify deployed.yaml --against config.yaml.2.j2 --all --pass ...
```

### Bundled Examples & Personal Templates

```bash
sc-compose examples list                    # discover starter templates
sc-compose examples pytest-fixture          # generate test stubs
sc-compose templates add my-template.md.j2  # save for reuse
sc-compose templates my-template            # render by name
```

### Reporting Subsystem

Produce compliance evidence from declarative specs:

```bash
sc-compose reports init              # scaffold report catalog
sc-compose reports smoke             # render smoke report fixture
sc-compose reports render-spec spec  # render from semantic spec
sc-compose reports finalize          # materialize metadata
sc-compose reports publish-manifest  # CI handoff manifest
```

### Python Bindings

```python
from sc_compose import compose, render_template, Renderer

result = render_template("Hello {{ name }}", {"name": "world"})
# Multi-pass rendering:
# compose(request) — full pipeline with ComposePolicy.passes
```

Pre-built wheels for macOS, Linux, Windows (Python 3.11+).

---

## Install Matrix

| Platform | Method | Command |
|----------|--------|---------|
| macOS | Homebrew | `brew install randlee/tap/sc-compose` |
| Windows | Winget | `winget install randlee.sc-compose` |
| Any (Rust) | crates.io | `cargo install sc-compose` |
| Any (Python) | PyPI | `pip install sc-compose` |
| Any (source) | cargo | `cargo build --release -p sc-compose` |
| Rust lib | Cargo.toml | `sc-composer = "1.3.1"` |

Bundled examples are guaranteed in Homebrew, Winget, and GitHub Release
installs. `cargo install` ships the binary only — set `SC_COMPOSE_DATA_DIR` for
examples.

---

## Status

| | |
|---|---|
| Version | 1.3.1 |
| MSRV | Rust 1.94.1 |
| Rust edition | 2024 |
| Platforms | macOS, Linux, Windows |
| Stability | stable 1.3 release line |

---

## Documentation

- [docs/requirements.md](docs/requirements.md) — normative behavior, JSON schemas, exit codes
- [docs/architecture.md](docs/architecture.md) — library module layout, crate boundary
- [docs/error-code-registry.md](docs/error-code-registry.md) — stable `ERR_*` diagnostic codes
- [docs/publishing.md](docs/publishing.md) — release procedures for integrators
- [docs/git-workflows.md](docs/git-workflows.md) — branching and review rules
- [docs/cross-platform-guidelines.md](docs/cross-platform-guidelines.md) — platform testing rules
- [docs/atm-adapter-notes.md](docs/atm-adapter-notes.md) — adapter boundary and integration
- [docs/manual/README.md](docs/manual/README.md) — bundled CLI feature manuals, also available via `sc-compose help <topic>`
- [RELEASING.md](RELEASING.md) — step-by-step release checklist
- [docs/repowise/README.md](docs/repowise/README.md) — code health analysis pipeline and regeneration

---

## Why sc-compose?

Prompt files drift across repos, tasks, and runtimes. Teams end up with several
copies of the same prompt: `.claude/agents/foo.md`, `.codex/agents/foo.md`, a
Slack paste, a gist, and a shell-history version. Those copies diverge. Agent
behavior diverges with them. Debugging turns into prompt diffing.

`sc-compose` treats prompts as source code you compose, not text you copy.
Compose once. Render deterministically. Keep shared fragments in one place and
include them by reference. Pass task context as variables. Validate required
inputs at render time so missing data fails fast instead of being guessed.

The workspace provides three packages:

- **sc-composer** — a Rust library with the render, include-expansion, validation, and diagnostics pipeline
- **sc-compose** — a CLI wrapper over the library for scripts, shells, and agent-invocable workflows
- **sc-compose (PyPI)** — Python native extension for `pip install`

All three are standalone. None is coupled to any particular orchestration
system.

---

## CLI Reference

| Command | What it does |
|---------|-------------|
| `render` | Render a template or resolved profile to stdout or a file |
| `resolve` | Print the resolved profile path and search trace |
| `validate` | Expand includes and analyze variables without writing output |
| `frontmatter-init` | Discover referenced variables and prepend minimal frontmatter |
| `init` | Create `.prompts/`, add it to `.gitignore`, and scan templates |
| `observability-health` | Report process-local structured logging health |
| `examples list` | List bundled starter templates |
| `examples <name>` | Render a bundled example with `--var` / `--var-file` |
| `templates list` | List your saved personal templates |
| `templates add <src> [name]` | Save a file or directory to your local template store |
| `templates <name>` | Render a saved template with `--var` / `--var-file` |
| `template-init` | Convert a concrete file into a multi-pass stacked template |
| `verify` | Verify a deployed file matches its multi-pass template source |
| `reports init` | Create the shared report scaffold and starter catalog |
| `reports smoke` | Render the built-in smoke report fixture |
| `reports finalize` | Materialize metadata and archives for producer-owned outputs |
| `reports render-spec` | Render a semantic report spec into shared artifacts |
| `reports index` | Summarize current latest report artifacts |
| `reports verify` | Verify required report evidence is present |
| `reports publish-manifest` | Write machine-readable publish handoff manifest |

Key flags:

| Flag | Purpose |
|------|---------|
| `--mode <file\|profile>` | Template lookup mode (default: `file`) |
| `--kind <agent\|command\|skill>` | Profile kind in profile mode |
| `--agent <name>` | Profile name in profile mode |
| `--runtime <claude\|codex\|gemini\|opencode>` | Runtime selector |
| `--file <path>` | Template path in file mode |
| `--var key=value` | Input variable (repeatable) |
| `--var-file <path>` | JSON/YAML variable file (`-` for stdin) |
| `--env-prefix <PREFIX_>` | Absorb env vars matching prefix |
| `--guidance <text>` / `--guidance-file <path>` | Append guidance block |
| `--prompt <text>` / `--prompt-file <path>` | Append user prompt block |
| `--output <path>` | Write rendered output to file |
| `--dry-run` | Report without modifying files |
| `--json` | Machine-readable output with diagnostics envelope |
| `--strict` | Fail on undeclared referenced variables |
| `--all` | Render all passes (multi-pass templates) |
| `--pass N --var ...` | Per-pass variable inputs |

Run `sc-compose <command> --help` for the full flag surface.

---

## Resolver Search Chains

| Runtime | Agents | Commands | Skills |
|---------|--------|----------|--------|
| Claude | `.claude/agents`, `.agents/agents` | `.claude/commands`, `.agents/commands` | `.claude/skills`, `.agents/skills` |
| Codex | `.codex/agents`, `.agents/agents`, `.claude/agents` | `.codex/commands`, `.agents/commands`, `.claude/commands` | `.codex/skills`, `.agents/skills`, `.claude/skills` |
| Gemini | `.gemini/agents`, `.agents/agents`, `.claude/agents` | `.gemini/commands`, `.agents/commands`, `.claude/commands` | `.gemini/skills`, `.agents/skills`, `.claude/skills` |
| OpenCode | `.opencode/agents`, `.agents/agents`, `.claude/agents` | same pattern | same pattern |

Claude is the universal fallback because it is the most common author target in
practice.

---

## Contributing

`main` is protected. Create feature branches from `develop` and follow
[docs/git-workflows.md](docs/git-workflows.md) for branching and review rules.
Adhere to the Pragmatic Rust Guidelines for code style.

## License

MIT. See [LICENSE](LICENSE).
