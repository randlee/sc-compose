---
id: fuzz-run-report-template
title: Single Fuzz-Run XHTML Report Template
status: draft
branch: feature/fuzz-run-report-template
worktree: ../sc-compose-worktrees/feature/fuzz-run-report-template
target: develop
---

# Sprint — Single Fuzz-Run XHTML Report Template

## Goal

Add a reusable, self-contained sc-compose report template representing the
outcome of **one single adversarial-fuzz probe case** (not a whole campaign),
conformant to the generic reporting contract in
`~/.claude/skills/html-report/SKILL.md`. The template must render two
distinct presentation modes driven by the case's outcome:

- **successful run** — concise: a short prose summary of what was tested and
  the input variables used. No failure scaffolding.
- **failed run** — detailed: what failed, a prose description of the
  requirement/ADR the case was validating against (or an explicit statement
  that no such requirement exists), a root-cause analysis, and a recommended
  fix.

This is a template-authoring sprint, not a one-off report. The deliverable is
a reusable Jinja2 template plus a documented data contract that any future
`sc-adversarial-fuzz-probe`/coordinator run can feed real case data into.

## Hard Dependencies

- `~/.claude/skills/html-report/SKILL.md` — the authoritative generic
  reporting contract (input JSON schema, copy-button requirements, HTML +
  JSON + optional XHTML output triad, `html-validate`/`xmllint` validation
  gates). This sprint's template must be a drop-in producer of that contract's
  `sections[]` entries — specifically, it targets the `xhtml_path` /
  `fragment_source: "auto-generated"` per-section fragment, sized for one
  fuzz case.
- `docs/phase-E/evidence/e-3-adversarial-campaign.json` — real shape of a
  campaign's per-case data (`findings[]`, `classification`,
  `reproduction_count`, `minimized_repro`, etc.) to model the template's
  variable contract against. Use real field names from this file rather than
  inventing a parallel schema.
- `.claude/agents/sc-adversarial-fuzz-probe.md` and
  `.claude/agents/sc-adversarial-fuzz-coordinator.md` — confirm the new
  template's per-case variable contract is something a probe agent could
  plausibly populate directly from its own fenced-JSON finding envelope,
  without an extra translation layer.

## Exact Targets

- A new Jinja2 XHTML fragment template under
  `.claude/skills/html-report/templates/` (or the existing report template
  directory used by `html-report-generator` — inspect that agent's prompt
  file first and follow its existing template location convention; do not
  invent a second template root).
- Documentation of the per-case variable contract (required/optional fields
  for both the success and failure presentation modes), added to
  `~/.claude/skills/html-report/SKILL.md` or a sibling doc referenced from it
  — whichever this repo's existing convention favors. Do not fork the
  contract; extend it.
- Two **mocked example reports** (one successful case, one failed case),
  rendered through the real template and the real `html-report-generator`
  pipeline, written to a scratch/example location in this worktree (e.g.
  `docs/examples/fuzz-run-report/`) for team-lead and the user to review
  before this branch merges. Ground the failure mock in the real
  `boundary-001` BOM-frontmatter-leak finding from
  `docs/phase-E/evidence/e-3-adversarial-campaign.json` (or the corresponding
  entry on `integrate/phase-e` if already merged) rather than fabricated
  placeholder content. Ground the success mock in a real passing case from
  the same evidence file.
- Both mocked example reports must pass `html-validate` (main HTML) and
  `xmllint` (XHTML fragment), per the SKILL.md validation requirement, before
  being reported as ready for review.

## Requirements

1. One template, two presentation modes, selected by a `classification` or
   `status` field on the case data (do not build two separate templates that
   duplicate the shell/copy-button markup — branch inside one template).
2. Successful-run mode renders only: case identity (id, worker, campaign),
   one-paragraph summary of what was tested, and a compact table of the input
   variables used. No root-cause/fix sections should render at all for a
   passing case — not just be empty.
3. Failed-run mode renders, in addition to case identity: a failure/expected-
   vs-observed section, a requirement/ADR-trace prose section (must handle
   the "no requirement covers this" case explicitly — do not fabricate a
   requirement reference when none exists), a root-cause-analysis section,
   and a recommended-fix section.
4. Copy-JSON and copy-context icon-only buttons per section, wired to
   `json_payload` / `context_text`, per SKILL.md — reuse the existing
   report-shell copy-button partial if `html-report-generator`'s existing
   templates already have one; do not hand-roll new clipboard JS.
5. Both light and dark rendering must be legible if the surrounding report
   shell supports theming; if the existing `html-report-generator` templates
   are single-theme, match that existing convention rather than introducing
   theme support unilaterally.

## Acceptance Criteria

- Template renders both modes from real (not fabricated) case data pulled
  from `docs/phase-E/evidence/e-3-adversarial-campaign.json`.
- Two rendered mock reports (success + failure) exist under
  `docs/examples/fuzz-run-report/` in this worktree, each with its HTML and
  JSON sidecar, and the failure mock's XHTML fragment.
- `html-validate` passes on both mock HTML reports; `xmllint` passes on the
  failure mock's XHTML fragment.
- The per-case variable contract (required/optional fields for each mode) is
  documented in-repo, not only implied by the template source.
- No existing `html-report` contract fields are renamed or removed; this
  sprint only adds a new template consumer of the existing contract.

## Non-Goals

- Do not build campaign-level (multi-case) reporting — that is a distinct,
  larger sprint.
- Do not change `sc-adversarial-fuzz-coordinator`/`sc-adversarial-fuzz-probe`
  agent behavior; this sprint only defines what a per-case report looks like
  once case data exists.
