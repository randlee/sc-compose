# O.5 Release Corpus and Fuzz Gate — Closure Checklist

This is the local execution record for
`docs/phase-O/sprint-o-5-release-corpus-fuzz-gate.md`. The campaign is
read-only against external repositories; external findings are handed off to
their owners.

## Initial audit

- [x] O5-001 — Pin the actual sc-compose O.4 root/commit and the available
  `atm-core` root/commit in `release-corpus-roots.txt`.
- [x] O5-002 — Enumerate every hidden `.json.j2`/`.json.jinja` template in
  both pinned roots and record the actual count and paths.
- [x] O5-003 — Classify quoted placeholders, bare placeholders, explicit
  `auto`, explicit `legacy`, and contract-negative fixtures separately.
- [x] O5-004 — Run the six O.4 hostile-value renders and parse every complete
  successful body with `serde_json`.
- [x] O5-005 — Reproduce the 1.4.0 quoted-placeholder regression as a
  fail-closed auto-mode case and verify explicit legacy compatibility.
- [x] O5-006 — Exercise nested/conditional values, hostile strings,
  malformed-output handling, and template-init/validate contracts using the
  existing O.2/O.3/O.4 tests; do not create a duplicate parser.
- [x] O5-007 — Generate the dated multi-worker HTML, JSON, and XHTML report
  package under `site/reports/` through the checked-in report templates.
- [x] O5-008 — Record the atm-core legacy-template finding, missing target
  descriptor, owner, exact paths, and handoff action without editing atm-core.
- [x] O5-009 — Add release-readiness, deprecation, migration, changelog, and
  ATM-core handoff documentation.
- [x] O5-009a — Add the pinned cpo, raptor, sc-lint, and synaptic-canvas roots
  supplied during campaign exploration; record their actual counts and
  read-only legacy findings.

## Re-audit findings and closure

- [x] O5-010 — The initial inventory used only visible paths; rerun with
  hidden-path enumeration so `.claude` templates are included. The final
  counts are 11 sc-compose, 7 atm-core, 7 cpo, 3 raptor, 6 sc-lint, and 3
  synaptic-canvas paths (37 total).
- [x] O5-011 — The report must not call atm-core clean when its six quoted
  production templates are unannotated. Mark them owned/actionable and make
  the release recommendation conditional on migration or an explicit legacy
  pin.
- [x] O5-012 — Keep expected negative fixtures in the evidence while
  separating them from production findings; repository-wide lint is not a
  release pass when intentional negative fixtures are present.
- [x] O5-013 — Validate report output with `html-validate` and every XHTML
  panel with `xmllint --noout`; retain the machine-readable sidecar used for
  the release decision.
- [x] O5-014 — Re-read the sprint plan and verify the campaign acceptance
  criteria; record the one remaining repository-level gate blocker rather
  than claiming a clean release.
- [ ] O5-015 — Resolve `O5-SC-LINT-BOOTSTRAP-001`: the full `just lint`
  wrapper cannot find `sc-lint-boundary` in this standalone workspace. This
  is intentionally handed off for a separate fix worktree/PR; no suppression
  is an acceptable closure.

## Final verification

- [x] All campaign cases have bounded iteration counts and deterministic
  worker identifiers.
- [x] Successful JSON cases have parser-backed evidence; expected rejected
  cases have fail-closed evidence and no emitted body.
- [x] The release recommendation is `CONDITIONAL`, not an unsupported clean
  release claim, because the pinned atm-core corpus needs owner action.
- [x] Workspace gates are run after the evidence/report files are complete;
  results are recorded in `evidence/o5-release-corpus.md` and the dated
  report sidecar.
- [x] `just test`, workspace tests, formatting, clippy, fast lint, direct
  sc-boundary, report validation, and diff checks pass.
- [ ] Full `just lint` is green: blocked by the missing `sc-lint-boundary`
  package in this standalone workspace (`O5-SC-LINT-BOOTSTRAP-001`). This
  requires a separate team-lead/sc-lint fix worktree/PR and a rerun after
  merge; no lint suppression was added.
