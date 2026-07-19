# Nested Templates — User Stories & Requirements

> Derived from design decisions DD-001 through DD-007 (2026-07-15)
> Prototype: `prototype/multipass/`

## User Stories

### US-1: Deploy → Install → Invoke variable resolution

**As a** team lead deploying agent configuration files across multiple repos,
**I want to** define variables that resolve at deploy-time, install-time, and invocation-time in a single template file,
**so that** I maintain one source of truth instead of three separate template tiers.

**Example:**
```yaml
---
pass: 3
required_variables: [team_name, repo_name]
defaults: { team_name: wyvern }
---
---
pass: 2
required_variables: [codex_agent]
defaults: { codex_agent: cwy }
---
---
pass: 1
required_variables: [task.title]
---
{{{{ team_name }}}} → {{{ codex_agent }}} → {{ task.title }}
```

**Acceptance criteria:**
- Template renders in 3 passes: outer first, inner last
- Each pass uses correct brace-count delimiters
- Pass 3 produces output with `{{{ codex_agent }}}` and `{{ task.title }}` intact
- Pass 2 consumes `{{{ }}}` delimiters
- Pass 1 consumes `{{ }}` delimiters
- Final output is fully resolved concrete text

### US-2: template-init converter

**As a** template author,
**I want to** convert an existing concrete file into a multi-pass stacked template by pointing at values that should become variables,
**so that** I don't have to hand-count braces or manually construct stacked YAML headers.

**Example:**
```bash
sc-compose template-init agents/quality-mgr.md \
  --pass 2 --var team_name=wyvern --var codex_agent=cwy \
  --pass 1 --var variant=claude
```

**Acceptance criteria:**
- Scans file for literal values (`wyvern`, `cwy`, `claude`)
- Replaces each with correct brace-count variables
- Generates stacked `---...---` headers with `pass: N`, `required_variables`, and `defaults`
- Longest-match-first to handle substring values (e.g., `wyvern` inside `../wyvern-worktrees`)
- Supports `--dry-run` and `--force`
- Exit 0 on success, exit 1 if values not found (or ambiguous)

### US-3: render --all across all passes

**As a** CI pipeline runner,
**I want to** render a template through all remaining passes in a single command with per-pass variable inputs,
**so that** I can produce a deployable file from a multi-pass template without manual step-by-step rendering.

**Example:**
```bash
sc-compose render template.2.j2 --all \
  --pass 2 --var-file vars/deploy.json \
  --pass 1 --var-file vars/install.json
```

**Acceptance criteria:**
- `--all` flag triggers multi-pass rendering
- `--pass N --var-file path` provides per-pass variables
- `--pass N --var key=val` provides per-pass inline variables
- Passes render in outer-to-inner order (highest pass number first)
- Final output is fully resolved
- Single-pass templates without `--all` render identically to current behavior (backward compat)

### US-4: verify drift check

**As a** skill registry operator,
**I want to** verify that a deployed file matches its multi-pass template source (no drift),
**so that** I can guarantee deployed files haven't been manually edited or corrupted.

**Example:**
```bash
sc-compose verify .claude/agents/quality-mgr.md \
  --against templates/quality-mgr.md.2.j2 \
  --pass 2 --var-file vars/deploy.json \
  --pass 1 --var-file vars/install.json
# exit 0 = clean, exit 1 = drift (unified diff)
```

**Acceptance criteria:**
- Renders template with provided per-pass variables
- Diffs rendered output against deployed file
- Exit 0 if identical, exit 1 with unified diff on mismatch
- Supports `--quiet` for CI use (no diff output, just exit code)
- Builtin variables (`RENDER_DATE`, `RENDER_TIMESTAMP`) are overridable for deterministic output

### US-5: Single-pass backward compatibility

**As an** existing sc-compose user,
**I want** all my current `.j2` templates to render identically under the nested-template system,
**so that** I can adopt multi-pass without any migration work.

**Acceptance criteria:**
- Template with single `---...---` header and no `pass` field → `pass: 1` (brace_count=2)
- `--all` flag on a single-pass template is a no-op
- `ComposeRequest` without `passes` field behaves identically to current behavior
- If a multi-pass template is reduced back to a single pass and needs to remain
  compatible with `1.2.x`, writer/converter flows omit `pass: 1` and normalize
  back to the current single-pass file shape
- All existing tests pass without modification

### US-6: Per-pass validation

**As a** template author,
**I want** validation to check each pass independently — its required variables, declared defaults, and discovered tokens,
**so that** I catch mismatches between headers and body at template-authoring time, not at deploy time.

**Acceptance criteria:**
- `discover_tokens(body, brace_count=N)` discovers variable tokens for a specific pass
- `discover_all_tokens(parsed)` returns `{pass_number → set of variable names}`
- Each pass validates: every discovered variable is either declared in `required_variables`, has a `default`, or is a builtin
- Warnings/errors are reported per-pass with pass number in the diagnostic

## Requirements (from design decisions)

### REQ-1: {N}-brace delimiter arithmetic

Pass N uses `{N+1}` braces for variable delimiters. Block delimiters `{% %}` are unchanged.

| Pass | Delimiter | Typical use |
|------|-----------|-------------|
| 1 | `{{ }}` | Invocation-time |
| 2 | `{{{ }}}` | Install-time |
| 3 | `{{{{ }}}}` | Deploy-time |

### REQ-2: Stacked YAML frontmatter

Each pass has its own `---...---` header block. Headers appear in outer-to-inner order. Each pass strips its header after rendering.

### REQ-3: pass field convention

`pass: N` in the header declares which pass the header belongs to. Absent →
`pass: 1`. `Brace_count = pass_number + 1`.

When a file is effectively single-pass and intended to remain compatible with
the shipped `1.2.x` format, emit no explicit `pass: 1` field.

### REQ-4: Frontmatter as source of truth

The YAML header's `pass` field is authoritative. File extension is a human signal only.

### REQ-5: Exact-match delimiter scanning

`discover_tokens(brace_count=N)` must NOT match `{N}` as a prefix inside `{N+1}`. `{{` inside `{{{` must be rejected.

### REQ-6: Longest-match-first for template-init

When replacing concrete values with variables during `template-init`, longest values are replaced first to prevent substring collisions.

### REQ-7: No breaking changes to current API

- `parse_template_document` for single-header templates preserves current
  behavior, and the public `frontmatter()` accessor remains the compatibility
  seam for callers that do not yet consume `passes`
- `ComposeRequest` without `passes` defaults to single pass
- All current `--var`, `--var-file`, `--strict` flags work unchanged in single-pass mode
