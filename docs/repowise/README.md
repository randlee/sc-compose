# sc-compose — Repowise Code Health Analysis

This directory contains generated code health artifacts produced by
[repowise](https://github.com/nicholasgriffintn/repowise), a domain-aware
codebase intelligence tool. The analysis uses a repo-specific config
(`.sc/repowise.yaml`) to avoid false positives in prototype code, FFI
bindings, and utility scripts.

## What Was Analyzed

Three repowise modes were run in sequence:

| Mode | Output | What It Produces |
|---|---|---|
| `health --format json --module` | `.sc/repowise/data/repowise-health-compose.json` | Per-file scores, KPIs, biomarker findings |
| `health --format json --refactoring-targets` | `.sc/repowise/data/repowise-refactoring-targets.json` | Ranked refactoring targets with plans and ROI |
| `dead-code --format json` | `.sc/repowise/data/repowise-dead-code.json` | Unreachable files, unused exports, zombie packages |
| `health --badge` | `badge.md` | Markdown health badge |
| `risk --format json` | `risk.json` | HEAD vs recent commit risk delta |

These raw JSON exports are then compiled into a rich markdown report by
`.sc/repowise/generate-report.py`.

## Output Artifacts

| File | Description |
|---|---|
| `health.md` | Comprehensive report: scores, worst files, biomarkers, refactoring targets, dead code, recommendations |
| `risk.json` | Risk assessment at HEAD |
| `badge.md` | Markdown health badge for README embedding |
| `architecture.html` | Crate dependency diagram (also at `site/repowise/architecture.html`) |
| `wiki/` | Per-file LLM-generated documentation (70 pages) |

## Regeneration

### Refresh raw data (after code changes)

```bash
cd <worktree-root>
repowise health --format json --module crates/sc-compose > .sc/repowise/data/repowise-health-compose.json 2>&1
repowise health --format json --refactoring-targets > .sc/repowise/data/repowise-refactoring-targets.json 2>&1
repowise dead-code --format json > .sc/repowise/data/repowise-dead-code.json 2>&1
```

### Regenerate the report from raw data

```bash
python3 .sc/repowise/generate-report.py
```

This reads the JSON files in `.sc/repowise/data/` and writes `health.md`.
Version, commit, and date are resolved from git at run time.

## Config

See `.sc/repowise.yaml` for module scope, annotated paths, and diagram
specs. The `annotated` section marks directories that are intentionally
excluded from the reachability analysis (Python FFI bindings, prototypes,
utility scripts).
