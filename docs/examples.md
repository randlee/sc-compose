---
layout: default
title: Examples — sc-compose
description: Bundled starter templates and workflow examples for sc-compose.
---

# Examples & Workflows

## Bundled Starter Templates

sc-compose ships with starter templates for common workflows:

```bash
# List available examples
sc-compose examples list

# Generate pytest test stubs from a list of test names
echo '{"test_names": ["login", "logout", "signup"]}' > tests.json
sc-compose examples pytest-fixture --var-file tests.json --output tests/test_auth.py

# Compose a service config
sc-compose examples service-config --var-file svc.json --output deploy/config.yaml

# Generate an HTML sprint report
sc-compose examples sprint-report-html \
  --var-file examples/sprint-report-html.sample-vars.json \
  --output sprint-report.html
```

The examples directory is located automatically from the binary path
(`../share/sc-compose/examples/` relative to the binary, following Homebrew
and FHS conventions). Override with `SC_COMPOSE_DATA_DIR`.

## Personal Templates

Save and reuse your own templates:

```bash
# Save a template for reuse
sc-compose templates add my-template.md.j2

# Import a directory as a template pack
sc-compose templates add my-pack-dir my-pack

# List saved templates
sc-compose templates list

# Render by name
sc-compose templates my-template --var-file data.json
```

Templates are stored under the platform user-data root in
`sc-compose/templates/`:

- Linux: `~/.local/share/sc-compose/templates/`
- macOS: `~/Library/Application Support/sc-compose/templates/`
- Windows: `%APPDATA%\sc-compose\templates\`

Override with `SC_COMPOSE_TEMPLATE_DIR`.

## AI Agent Profile Workflow

### 1. Initialize a workspace

```bash
sc-compose init
```

Creates `.prompts/` and adds it to `.gitignore`.

### 2. Create a shared agent profile

```markdown
<!-- .agents/agents/reviewer.md -->
---
name: reviewer
description: Reviews PRs against project conventions
tools: Glob, Grep, LS, Read, Write, Edit, Bash
---

You are a code reviewer. Before reviewing:

@<_includes/house-style.md>

Then review the PR described below.
```

### 3. Create your house style include

```markdown
<!-- _includes/house-style.md -->
- Prefer single-mechanism designs over redundant code paths
- All public functions must have docstrings
- No hardcoded paths — use shutil.which()
- Tests must pass on macOS, Linux, and Windows
```

### 4. Resolve and render for a specific runtime

```bash
# See which profile resolves for Claude
sc-compose resolve --mode profile --kind agent --agent reviewer --runtime claude

# Render to stdout
sc-compose render --mode profile --kind agent --agent reviewer --runtime claude

# Render with per-task variables
sc-compose render --mode profile --kind agent --agent reviewer --runtime claude \
  --var task_id=PR-42 --var branch=feature/login
```

### 5. Pass ephemeral task context

```bash
sc-compose render --mode profile --kind agent --agent reviewer \
  --guidance "Focus on concurrency safety in the new poll loop." \
  --prompt "PR #42: Replace SIGUSR1 with HTTP RPC for daemon wake. Check for thread safety."
```

## Multi-Pass Template Workflow (v1.3.0)

### Converting a concrete file to a template

```bash
sc-compose template-init config.yaml --pass 2 --var team=infra --pass 1 --var env=prod
```

This generates a `.2.j2` file with stacked YAML headers and the correct
brace-count variables.

### Rendering all passes

```bash
sc-compose render config.yaml.2.j2 --all \
  --pass 3 --var-file deploy.json \
  --pass 2 --var-file install.json \
  --pass 1 --var-file invoke.json
```

### Verifying deployed output

```bash
sc-compose verify /etc/app/config.yaml --against config.yaml.2.j2 --all \
  --pass 3 --var-file deploy.json \
  --pass 2 --var-file install.json
```

Exit 0 if identical, exit 1 with unified diff on mismatch.

## Reports Workflow

```bash
# Initialize report catalog
sc-compose reports init

# Render smoke report
sc-compose reports smoke

# Render from a semantic spec
sc-compose reports render-spec specs/sprint-42.yaml

# Finalize and archive
sc-compose reports finalize

# Generate CI handoff manifest
sc-compose reports publish-manifest

# Verify evidence
sc-compose reports verify
```
