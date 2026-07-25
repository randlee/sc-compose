# sc-compose — Repowise Code Health Analysis

This directory contains generated code health artifacts produced by
[repowise](https://github.com/nicholasgriffintn/repowise), a domain-aware
codebase intelligence tool. The analysis uses a repo-specific config
(`.sc/repowise.yaml`) to avoid false positives in prototype code, FFI
bindings, and utility scripts.

## Analysis Pipeline

The full pipeline runs in three phases:

### Phase 1 — Index (prerequisite)

```bash
repowise init --index-only -y
```

Builds the knowledge graph, AST index, and git history. Must be re-run
after significant code changes before refreshing any derived data.

### Phase 2 — Raw data export

| Command | Output | What It Produces |
|---|---|---|
| `repowise health --format json --module crates/sc-compose` | `.sc/repowise/data/repowise-health-compose.json` | Per-file scores, KPIs, biomarker findings |
| `repowise health --format json --refactoring-targets` | `.sc/repowise/data/repowise-refactoring-targets.json` | Ranked refactoring targets with plans and ROI |
| `repowise dead-code --format json` | `.sc/repowise/data/repowise-dead-code.json` | Unreachable files, unused exports, zombie packages |
| `repowise health --badge` | `docs/repowise/badge.md` | Markdown health badge (for README embedding) |
| `repowise risk --format json` | `docs/repowise/risk.json` | HEAD vs recent commit risk delta |

### Phase 3 — Compile reports

| Command | Output | What It Produces |
|---|---|---|
| `python3 .sc/repowise/generate-report.py` | `docs/repowise/health.md` | Comprehensive report: scores, worst files, biomarkers, refactoring targets, dead code, recommendations |
| `python3 .sc/repowise/generate-architecture.py` | `site/repowise/architecture.html` | Crate dependency diagram (reads `.repowise/knowledge-graph.json` and `Cargo.toml`, resolves version from git) |
| `repowise init` (full, with LLM) then `repowise export --format html -o site/repowise/wiki/` | `site/repowise/wiki/` | Per-file LLM-generated documentation (requires API key configured in repowise; optional) |

## Output Artifacts

| File | Phase | Description |
|---|---|---|
| `health.md` | 3 | Comprehensive report with interpretation and recommendations |
| `risk.json` | 2 | Risk assessment at HEAD |
| `badge.md` | 2 | Markdown health badge |
| `../site/repowise/architecture.html` | 3 | Crate dependency diagram (web-facing) |
| `../site/repowise/wiki/` | 3 (LLM) | Per-file documentation (69 pages; optional) |

## Regeneration (full pipeline)

```bash
cd <worktree-root>

# Phase 1: Re-index (after code changes)
repowise init --index-only -y

# Phase 2: Export raw data
repowise health --format json --module crates/sc-compose \
  > .sc/repowise/data/repowise-health-compose.json 2>&1
repowise health --format json --refactoring-targets \
  > .sc/repowise/data/repowise-refactoring-targets.json 2>&1
repowise dead-code --format json \
  > .sc/repowise/data/repowise-dead-code.json 2>&1
repowise health --badge > docs/repowise/badge.md 2>&1
repowise risk --format json > docs/repowise/risk.json 2>&1

# Phase 3: Compile reports
python3 .sc/repowise/generate-report.py
python3 .sc/repowise/generate-architecture.py

# Phase 3 (optional, requires LLM): Regenerate wiki
repowise init                    # full init with LLM page generation
repowise export --format html -o site/repowise/wiki/
```

The `generate-report.py` script resolves version, commit, and date from git
at run time — no hardcoded metadata.

## Config

See `.sc/repowise.yaml` for module scope, annotated paths, and diagram
specs. The `annotated` section marks directories that are intentionally
excluded from the reachability analysis (Python FFI bindings, prototypes,
utility scripts).
