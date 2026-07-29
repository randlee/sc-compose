---
id: fuzz-run-report-template
title: Multi-Agent Fuzz-Session Report Template
status: complete
branch: feature/fuzz-run-report-template
worktree: ../sc-compose-worktrees/feature/fuzz-run-report-template
target: develop
---

# Sprint — Multi-Agent Fuzz-Session Report Template

## Goal

Add a reusable, self-contained sc-compose report package representing **one
fuzz session** coordinated by a primary agent and a swarm of adversarial-fuzz
workers, conformant to the generic reporting contract in
`~/.claude/skills/html-report/SKILL.md`. Each worker runs one distinct bounded
fuzz test and returns structured JSON plus one XHTML panel. The single HTML
page combines those panels and starts with a summary table. The template must
render two distinct presentation modes driven by each worker's outcome:

- **successful run** — concise: a short prose summary of the worker task, its
  iteration/pass fraction, and the concrete inputs used. No failure
  scaffolding.
- **failed run** — detailed: the failing frontmatter/template and input, what
  failed, the requirement/ADR/NFR the worker was validating against (or an
  explicit statement that no such requirement exists), a root-cause analysis,
  and a recommended fix.

This is a template-authoring sprint, not a one-off report. The deliverable is
a reusable Jinja2 worker-panel template plus a documented session data
contract that any future `sc-adversarial-fuzz-probe`/coordinator run can feed
real worker data into. When a worker finds a candidate failure, the primary
agent may deploy background explore agents to identify the relevant
requirement/ADR/NFR, root cause, and recommended change before rendering the
panel.

## Integration Point (amended)

- The report is generated **after** a run of
  `.claude/skills/adversarial-fuzzing/`, not as a standalone/manual step.
  Add a report-generation step to that skill's `## Workflow` (after step 9,
  which currently ends the workflow at "summarize confirmed bugs, promoted
  tests, unresolved candidates, and campaign limits") that emits one report
  per fuzz session using this sprint's template. The primary agent coordinates
  multiple workers, each running one fuzz test and returning structured JSON;
  the pipeline places one XHTML panel per worker into the single HTML page.
- Output location: `site/reports/` (repo-relative, not the scratch/example
  location used for this sprint's own review mocks — see below).
- Filename convention: `<datecode>-<index>-fuzz-report.html`, where
  `<datecode>` is `YYYYMMDD` and `<index>` is a per-day, per-session
  1-based sequence number reset each day. Example: `20260729-1-fuzz-report.html`.
  The JSON sidecar uses the same stem, and each worker panel uses that stem
  plus a deterministic worker suffix (e.g.
  `20260729-1-fuzz-report.json` and
  `20260729-1-fuzz-report-shape-probe.xhtml`) so the package stays associated
  by filename alone.
- This sprint's own **review mocks** (the two examples for team-lead/user
  sign-off before merge) still live under `docs/examples/fuzz-run-report/`
  as originally scoped — they are design-review artifacts, not real
  session output, and must not be written into `site/reports/`. Once the
  template is approved, the *real* integration wired into the
  adversarial-fuzzing skill is what writes to `site/reports/` using the
  datecode-index filename convention.

## Hard Dependencies

- `~/.claude/skills/html-report/SKILL.md` — the authoritative generic
  reporting contract (input JSON schema, copy-button requirements, HTML +
  JSON + optional XHTML output triad, `html-validate`/`xmllint` validation
  gates). This sprint's template must be a drop-in producer of that contract's
  `sections[]` entries — specifically, it targets the `xhtml_path` /
  `fragment_source: "auto-generated"` per-section fragment, sized for one
  worker's single fuzz test.
- `docs/phase-E/evidence/e-3-adversarial-campaign.json` — real shape of a
  campaign's per-case data (`findings[]`, `classification`,
  `reproduction_count`, `minimized_repro`, etc.) to model the template's
  variable contract against. Use real field names from this file rather than
  inventing a parallel schema.
- `.claude/agents/sc-adversarial-fuzz-probe.md` and
  `.claude/agents/sc-adversarial-fuzz-coordinator.md` — confirm the worker
  result contract is something a probe agent could plausibly populate directly
  from its own fenced-JSON finding envelope, and that the primary coordinator
  can enrich failures with background requirement/root-cause/fix research.

## Exact Targets

- `.claude/skills/adversarial-fuzzing/SKILL.md` — add the report-generation
  step to `## Workflow` as described under Integration Point above.
- A new Jinja2 XHTML fragment template under
  `.claude/skills/html-report/templates/` (or the existing report template
  directory used by `html-report-generator` — inspect that agent's prompt
  file first and follow its existing template location convention; do not
  invent a second template root).
- Documentation of the session and worker-panel variable contract
  (required/optional fields for both the success and failure presentation
  modes), added to
  `~/.claude/skills/html-report/SKILL.md` or a sibling doc referenced from it
  — whichever this repo's existing convention favors. Do not fork the
  contract; extend it.
- Two **mocked example reports** (one successful session, one failed session),
  each containing multiple worker panels, rendered through the real template
  and the real `html-report-generator` pipeline, written to a scratch/example
  location in this worktree (e.g. `docs/examples/fuzz-run-report/`) for
  team-lead and the user to review before this branch merges. Ground the
  failure panel in the real
  `boundary-001` BOM-frontmatter-leak finding from
  `docs/phase-E/evidence/e-3-adversarial-campaign.json` (or the corresponding
  entry on `integrate/phase-e` if already merged) rather than fabricated
  placeholder content. Ground the success mock in a real passing case from
  the same evidence file.
- Both mocked example reports must pass `html-validate` (main HTML) and
  `xmllint` for every XHTML worker panel, per the SKILL.md validation
  requirement, before
  being reported as ready for review.

## Requirements

1. One worker-panel template, with two presentation modes selected by a
   `classification` or `status` field on the worker data (do not build two
   separate templates that duplicate the shell/copy-button markup — branch
   inside one template).
2. The main HTML page renders a summary table before the panels, with one row
   per worker and columns for fuzz run description, iterations, pass fraction,
   and a simple PASS/FAIL result.
3. Successful-worker mode renders only worker identity, task, iteration/pass
   result, a one-paragraph summary, and a compact table of concrete inputs.
   No root-cause/fix sections should render at all for a passing worker — not
   just be empty.
4. Failed-worker mode renders, in addition to worker identity: the exact
   failing frontmatter/template and input, expected-vs-observed behavior, a
   requirement/ADR/NFR trace (including the "no requirement covers this" case),
   a requirement-gap assessment that recommends creating/updating an ADR or
   requirement only for a genuine supported-contract gap, a root-cause
   analysis, and a recommended fix for every finding.
5. Copy-JSON and copy-context icon-only buttons per section, wired to
   `json_payload` / `context_text`, per SKILL.md — reuse the existing
   report-shell copy-button partial if `html-report-generator`'s existing
   templates already have one; do not hand-roll new clipboard JS.
6. Both light and dark rendering must be legible if the surrounding report
   shell supports theming; if the existing `html-report-generator` templates
   are single-theme, match that existing convention rather than introducing
   theme support unilaterally.

## Acceptance Criteria

- Template renders both modes from real (not fabricated) worker/case data
  pulled from `docs/phase-E/evidence/e-3-adversarial-campaign.json`.
- Two rendered mock reports (success + failure) exist under
  `docs/examples/fuzz-run-report/` in this worktree. Each has one HTML page,
  one JSON sidecar, and multiple XHTML worker panels. The HTML page has the
  summary table at the top.
- `html-validate` passes on both mock HTML reports; `xmllint` passes on every
  XHTML worker panel.
- The session and worker-panel variable contract (required/optional fields
  for each mode) is documented in-repo, not only implied by the template
  source.
- No existing `html-report` contract fields are renamed or removed; this
  sprint only adds a new template consumer of the existing contract.
- `.claude/skills/adversarial-fuzzing/SKILL.md`'s workflow now includes a
  report-generation step that writes one multi-panel session report to
  `site/reports/` using the `<datecode>-<index>-fuzz-report.html` filename
  convention (with matching `.json` sidecar and one `.xhtml` panel per
  worker).

## Non-Goals

- Do not build cross-session campaign aggregation or historical trend
  reporting. A single fuzz session is intentionally multi-agent and
  multi-panel.
- Do not change the probe/coordinator execution behavior; this sprint defines
  the structured handoff and report composition after the swarm run.
