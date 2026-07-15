---
name: sprint-report
description: Generate a sprint status report for the current phase. Default is --table.
---

# Sprint Report Skill

Build fenced JSON and pipe to the Jinja2 template. `mode` controls table vs detailed.

## Usage

```
/sprint-report [--table | --detailed | --html]
```

Default: `--table`

---

## Data Source

**Always use `atm gh pr list` first** — single call, returns all open PRs with CI and merge state:

```bash
atm gh pr list
```

This is faster and sufficient for populating `sprint_rows` and `integration_row`. Only drill into individual `gh run view` calls if you need failure details for a specific job.

**Dogfooding rule**: If `atm gh pr list` output is missing information needed to fill the report (e.g., no per-job failure detail, no QA state, truncated CI summary), **file a GitHub issue** describing what field or format change would make it sufficient, then improve the command. Do not silently work around gaps with extra `gh` CLI calls — surface them as product issues.

## Render Command

The template path is relative — must run from the **main repo root** (not a worktree).
The `CLAUDE_PROJECT_DIR` fallback here assumes it points at the main repo root
when you are operating from a worktree; if that environment variable is unset,
the `git worktree list | head -1` fallback is used instead.

```bash
cd "${CLAUDE_PROJECT_DIR:-$(git worktree list | head -1 | awk '{print $1}')}"
echo '<json>' > /tmp/sprint-report.json
sc-compose render --file .claude/skills/sprint-report/report.md.j2 --var-file /tmp/sprint-report.json
```

## --html

`--html` uses wrapper-owned orchestration:

1. Build one structured JSON payload.
2. Render the bundled example with `sc-compose examples sprint-report-html`.
3. Let wrapper logic write and optionally open the generated file unless `--write-only` is set.

`sc-compose` itself remains a single-render tool. It does not gain hooks or
browser-open behavior for this flow.

Example structured payload:

```json
{
  "report": {
    "title": "HTML Sprint Report",
    "phase": "Phase HTML-Report",
    "generated_at": "2026-04-20T05:30:00Z",
    "repository": "randlee/sc-compose"
  },
  "summary": {
    "completed": 2,
    "in_review": 1,
    "blocked": 0
  },
  "pr": {
    "number": 47,
    "url": "https://github.com/randlee/sc-compose/pull/47",
    "merge_url": "https://github.com/randlee/sc-compose/pull/47"
  },
  "ci": {
    "run_name": "CI #118",
    "run_url": "https://github.com/randlee/sc-compose/actions/runs/118",
    "status": "PASS",
    "summary": "fmt, clippy, and workspace tests are green."
  },
  "links": {
    "plan_url": "https://github.com/randlee/sc-compose/blob/develop/docs/project-plan.md",
    "findings_url": "https://github.com/randlee/sc-compose/blob/develop/docs/html-sprint-report-plan.md"
  },
  "sprints": [
    {
      "id": "H1",
      "title": "Structured object inputs",
      "stage": "merged",
      "qa": "PASS",
      "ci": "PASS",
      "pr_url": "https://github.com/randlee/sc-compose/pull/45",
      "note": "Object values and nested field diagnostics landed."
    }
  ]
}
```

Recommended wrapper flow:

```bash
cd "${CLAUDE_PROJECT_DIR:-$(git worktree list | head -1 | awk '{print $1}')}"
OUTPUT_PATH="${SPRINT_REPORT_HTML_OUT:-/tmp/sprint-report.html}"
echo '<json>' > /tmp/sprint-report-html.json
sc-compose examples sprint-report-html \
  --var-file /tmp/sprint-report-html.json \
  --output "${OUTPUT_PATH}"
python3 - <<'PY' "${OUTPUT_PATH}"
import pathlib
import sys
import webbrowser

path = pathlib.Path(sys.argv[1]).resolve()
webbrowser.open(path.as_uri())
PY
```

When `--write-only` is requested, skip the `python3 -m webbrowser` step and
just report the output path.
## --table (default)

```json
{
  "mode": "table",
  "sprint_rows": "| AK.1 | ✅ | ✅ | 🏁 | #621 |\n| AK.2 | ✅ | ✅ | 🌀 | #622 |",
  "integration_row": "| **integrate** | | — | 🌀 | — |"
}
```

## --detailed

```json
{
  "mode": "detailed",
  "sprint_rows": "Sprint: AK.1  Contract reconciliation\nPR: #621\nQA: PASS ✓ (iter 3)\nCI: Merged to integrate/phase-AK ✓\n────────────────────────────────────────\nSprint: AK.2  OTel core\nPR: #622\nQA: PASS ✓\nCI: Running (1 pending)",
  "integration_row": "Integration: integrate/phase-AK → develop\nCI: Running — pending AK.4 + AK.5"
}
```

## Icon Reference

| State | DEV | QA | CI |
|-------|-----|----|----|
| Assigned | 📥 | 📥 | |
| In progress | 🌀 | 🌀 | 🌀 |
| Done/Pass | ✅ | ✅ | ✅ |
| Findings | 🚩 | 🚩 | |
| Fixing | 🔨 | | |
| Blocked | | | 🚧 |
| Fail | | | ❌ |
| Merged | | | 🏁 |
| Ready to merge | | | 🚀 |
