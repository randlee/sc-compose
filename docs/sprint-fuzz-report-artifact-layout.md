---
id: fuzz-report-artifact-layout
title: Per-Report Artifact Subdirectory Layout For html-report Packages
status: complete
branch: feature/fuzz-run-report-template
worktree: ../sc-compose-worktrees/feature/fuzz-run-report-template
target: develop
---

# Sprint — Per-Report Artifact Subdirectory Layout

## Goal

The user is building a historical, browsable record of generated reports
under `site/reports/`. Reviewing the current flat layout, they observed that
every report's main HTML file sits directly in `site/reports/`, but its
supporting artifacts (the JSON sidecar and per-worker XHTML fragments) also
sit directly in `site/reports/`, flat and interleaved with every other
report's files. This makes it easy to mistake "one report's real content" for
"the only content present" when browsing the directory, and makes it harder
to link a report's full regeneration data as one coherent unit from a future
website index page.

This sprint changes the artifact layout so that for any report named
`<report-name>.html` (e.g. `20260729-3-fuzz-report.html`):

- the top-level `<report-name>.html` file stays directly under
  `site/reports/` (this is the browsable/linkable entry point for a future
  website index)
- every supporting artifact needed for full regeneration — the JSON sidecar
  and all per-worker XHTML fragments — moves into
  `site/reports/<report-name>/`

This is a generic change to the `html-report` output contract, not a
fuzz-report-specific one — the user explicitly wants the same convention
applied uniformly as they go through and normalize every existing report
family under `site/reports/`.

## Exact Targets

1. `~/.claude/skills/html-report/SKILL.md` — update the Input Contract /
   Template Guidance sections describing `output_path`, `json_output_path`,
   and `xhtml_path` so the documented convention is:
   - `output_path` — `site/reports/<report-name>.html` (unchanged, top level)
   - `json_output_path` — `site/reports/<report-name>/<report-name>.json`
   - each section's `xhtml_path` — `site/reports/<report-name>/<report-name>-<section-id>.xhtml`
2. `.claude/agents/html-report-generator.md` (or wherever the generator's
   output-path derivation logic/instructions live) — update to derive and
   create the `<report-name>/` subdirectory from `output_path`'s stem and
   write the JSON sidecar and XHTML fragments there instead of flat in
   `site/reports/`.
3. `.claude/skills/adversarial-fuzzing/SKILL.md`'s report-generation step
   (added in the `fuzz-run-report-template` sprint) — confirm/update the
   filename-convention note so it reflects the new subdirectory placement for
   the sidecar and worker panels.
4. The rendered HTML shell's internal links to XHTML fragments (currently
   relative hrefs like
   `href="20260729-3-fuzz-report-boundary-probe.xhtml"` — see
   `site/reports/20260729-3-fuzz-report.html` around each section's "open
   fragment" control) must be updated to the new relative path,
   `href="20260729-3-fuzz-report/20260729-3-fuzz-report-boundary-probe.xhtml"`,
   so links keep resolving after the move. Confirm whether this href is
   template-driven (preferred fix location) or generator-assembled.
5. Migrate the 3 existing report bundles already present in `site/reports/`
   (`20260729-1-fuzz-report.*`, `20260729-2-fuzz-report.*`,
   `20260729-3-fuzz-report.*`) into the new layout:
   `site/reports/20260729-1-fuzz-report.html` stays; its `.json` and 4
   `.xhtml` files move into `site/reports/20260729-1-fuzz-report/`, and so on
   for -2 and -3. Update each migrated HTML file's internal fragment hrefs to
   match.

## Non-Goals

- Do not change the JSON sidecar's internal schema, field names, or the
  worker-panel template's rendered markup/content. This is purely an
  artifact-layout change: which directory a file lives in, and the href/path
  strings that point at it.
- Do not build the website history/index page itself — that is separate,
  future work the user is preparing this layout for.
- Do not touch any report family outside `site/reports/` in this worktree.

## Acceptance Criteria

- For each of the 3 existing fuzz reports, `site/reports/<name>.html` exists
  at the top level, and `site/reports/<name>/<name>.json` plus all 4
  `site/reports/<name>/<name>-<worker>.xhtml` files exist, with no leftover
  copies of the moved files remaining flat in `site/reports/`.
- Each migrated `<name>.html`'s in-page links to its worker XHTML fragments
  resolve to the new subdirectory path.
- `xmllint --noout` passes on every migrated `.xhtml` file in its new
  location; `html-validate` passes on every migrated `.html` file.
- `~/.claude/skills/html-report/SKILL.md` documents the new
  `json_output_path` / `xhtml_path` subdirectory convention so a future
  report generation run (for this or any other report family using this
  skill) produces the new layout without hand-fixing.
- A fresh end-to-end regeneration (same mechanism used to produce
  `20260729-3-fuzz-report.html`) confirms the generator itself now writes
  directly into the new layout, not just that existing files were manually
  moved.
- sprint doc deliverables gate:
  - this doc exists
  - this doc's frontmatter sets `status: complete`
- this doc's frontmatter sets `branch:` and `worktree:`

## Completion Evidence

The report generator was run from staged copies of the three original
bundles. It derived each artifact directory from the top-level HTML stem and
wrote fresh HTML, JSON, and four XHTML panel files per session before the
legacy flat sidecars and panels were moved out of `site/reports/`.

- `site/reports/20260729-1-fuzz-report.html` plus its 5 supporting artifacts
- `site/reports/20260729-2-fuzz-report.html` plus its 5 supporting artifacts
- `site/reports/20260729-3-fuzz-report.html` plus its 5 supporting artifacts
- no supporting artifacts remain directly under `site/reports/`
