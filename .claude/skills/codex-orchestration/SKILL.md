---
name: codex-orchestration
version: 0.1.0
description: Orchestrate sc-compose sprint work where team-lead coordinates, comp is the sole developer, and quality-mgr enforces the QA gate.
depends_on:
  quality-mgr: 0.x
  req-qa: 0.x
  arch-qa: 0.x
  rust-qa-agent: 0.x
  simplification-reviewer: 1.x
  test-auditor: 1.x
---

# Codex Orchestration

This skill defines the repo-local orchestration workflow for `sc-compose`.

## Model

- `team-lead` coordinates sprint sequencing, worktree assignments, and PR flow
- `comp` is the sole developer for Codex-driven implementation work
- `quality-mgr` runs the QA gate after each delivery

## Preconditions

Before starting a sprint:
1. `docs/requirements.md`, `docs/architecture.md`, and `docs/project-plan.md`
   define the sprint or phase review target.
2. A worktree exists for the sprint branch under the repo’s worktree strategy.
3. The target branch for the sprint is the phase's `integrate/phase-<x>`
   branch, per Integration Branch below — never `develop` directly.
4. The following prompts exist in `.claude/agents/`:
   - `quality-mgr.md`
   - `req-qa.md`
   - `arch-qa.md`
   - `rust-qa-agent.md`
   - `simplification-reviewer.md`
   - `test-auditor.md`
5. `sc-compose` is available from the Homebrew install path for rendering the
   bundled XML, JSON, and markdown templates.
6. An `integrate/phase-<x>` branch exists for the phase (see Integration
   Branch below). Create it off `develop` before dispatching the phase's
   first sprint if it does not already exist.

## Integration Branch

Every phase always uses an integration branch, never `develop` directly:

- `team-lead` creates `integrate/phase-<x>` off `develop` before the phase's
  first sprint is dispatched, if it does not already exist.
- Every sprint in the phase branches from, and its PR targets, that phase's
  `integrate/phase-<x>` branch — not `develop`. This applies to every sprint
  dev assignment's `pr_target` and every sprint PR's base branch, with no
  exceptions for "small" or docs-only sprints.
- Sprint PRs merge into `integrate/phase-<x>` once QA passes and CI is green,
  per the normal Sprint Flow gate below.
- `integrate/phase-<x>` should periodically merge `develop` forward
  in-phase (not just at the end) to avoid large end-of-phase conflicts.
- Only after all sprints in the phase are merged into `integrate/phase-<x>`
  does `team-lead` run the Phase-End Review (below) against that integration
  branch.
- Once phase-ending QA passes, `team-lead` opens the single
  `integrate/phase-<x>` -> `develop` PR and merges it. This is the only PR
  in the phase that targets `develop` directly.
- If a phase's plan doc declares a `branch:` value in its frontmatter, that
  value must be this phase's `integrate/phase-<x>` branch name — treat a
  mismatch or an unset value as a plan-doc defect to fix before dispatching
  sprints.

## Sprint Flow

1. `team-lead` assigns development to `comp` using `dev-template.xml.j2`,
   with `pr_target` set to the phase's `integrate/phase-<x>` branch (never
   `develop`). Every dev assignment must include the sprint-plan document
   path as `sprint_doc`, and that sprint document is the authoritative
   source for the task. Assignment prose may summarize, but it must not
   replace or weaken the sprint doc.
2. `comp` ACKs, implements, commits, pushes, and reports branch plus SHA.
3. Before QA starts, `comp` performs the normal repo validation sweep required
   by the active assignment and fixes obvious local issues before handoff.
4. `team-lead` opens or updates the PR against `integrate/phase-<x>`.
5. `team-lead` assigns QA to `quality-mgr` using `qa-template.xml.j2`.
   Every QA assignment must include `sprint_doc`, and `quality-mgr` must treat
   that sprint document as the authoritative QA scope source.
6. `quality-mgr` launches the reviewer set:
   - `rust-qa-agent`
   - `req-qa`
   - `arch-qa`
   - `simplification-reviewer`
7. All blocking findings must be routed back to `comp` via
   `fix-assignment.xml.j2` before another QA round begins.
8. If QA passes and CI is green, the sprint PR may merge into
   `integrate/phase-<x>`. This is not a merge into `develop` — see
   Integration Branch above for when `develop` actually receives the phase.
9. If QA fails, `team-lead` routes concrete fixes back to `comp` using
   `fix-assignment.xml.j2`. Fix assignments must also include `sprint_doc`,
   and the sprint document remains authoritative if the task summary omits or
   compresses details.

## Plan Review Flow

1. `team-lead` completes `/plan-hardening` steps 1 through 5.
2. `team-lead` assigns plan QA to `quality-mgr` using `qa-template.xml.j2`
   with `review_type: plan_gate`.
3. The QA assignment must include the phase-plan document as `sprint_doc`, and
   that plan document is the authoritative scope source for plan QA.
4. `quality-mgr` treats `review_type: plan_gate` as docs-only review and
   launches:
   - `req-qa`
   - `arch-qa`
5. If plan QA passes, the hardened plan is ready for implementation dispatch.
6. If plan QA fails, `team-lead` uses the normal codex-orchestration
   triage-and-fix loop to route concrete fixes back to `comp`.

## QA Coverage Rule

- `quality-mgr` must extract every deliverable, acceptance criterion, deletion
  target, required validation item, and expected artifact from `sprint_doc`
  before launching `req-qa`
- `req-qa` must independently treat `sprint_doc` as authoritative
- `req-qa` must count deliverable completion and report a completion percentage
- `arch-qa` must inspect sprint-doc structural gate artifacts directly when a
  deliverable points to a boundary, packaging, release-tracking, readiness, or
  validation gate
- QA cannot PASS unless deliverable completion is 100%

## Phase-End Review

Phase-End Review only runs once every sprint in the phase has merged into
`integrate/phase-<x>`. It is the required gate before the
`integrate/phase-<x>` -> `develop` merge described in Integration Branch
above — it does not gate individual sprint PRs, which merge into
`integrate/phase-<x>` on their own per-sprint QA pass.

For extraction-readiness or phase-close reviews, use `review-template.xml.j2`
to assign a read-only review to `comp`.

For phase-ending QA routed through `quality-mgr`, the reviewer set is
mandatory:
- `rust-qa-agent`
- `req-qa`
- `arch-qa`
- `simplification-reviewer`
- `test-auditor`

Only after this phase-ending QA passes does `team-lead` open and merge the
`integrate/phase-<x>` -> `develop` PR.

## CI

Use standard GitHub CLI:
- `gh pr checks <PR> --watch`
- `gh pr view <PR> --json mergeStateStatus,reviewDecision`

Do not assume ATM-specific PR monitoring commands exist.

## Assignment Templates

Use the templates in this skill directory:
- `dev-template.xml.j2`
- `fix-assignment.xml.j2`
- `qa-template.xml.j2`
- `review-template.xml.j2`
- `req-qa-assignment.json.j2`
- `arch-qa-assignment.json.j2`
- `flaky-test-qa-assignment.json.j2`
- `sprint-plan.md.j2`

## Required Message Sequence

Every ATM task message must follow:
1. ACK
2. Work
3. Completion summary
4. Completion ACK by receiver
