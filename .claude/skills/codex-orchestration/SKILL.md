---
name: codex-orchestration
description: Orchestrate multi-sprint phases where comp is the sole developer, with pipelined QA via a dedicated quality-mgr teammate.
---

# Codex Orchestration

This skill defines how `team-lead` orchestrates phases where `comp` is the sole
developer, executing sprints sequentially while QA runs in parallel via
`quality-mgr`.

## Core Rule

`quality-mgr` owns QA coordination for all three review modes:

- `plan_gate`
- `sprint_review`
- `phase_ending_review`

`team-lead` chooses the review type.
`quality-mgr` chooses and launches the reviewers according to
`.claude/agents/quality-mgr.md`.
`quality-mgr` must re-read that prompt for every assignment.

Do not hardcode reviewer selection in `team-lead` messages when using this
skill.

## Plan Is The Spec

For development assignments, the active plan in `docs/project-plan.md` is
authoritative.

`team-lead` must:

- read the active plan first
- identify the exact sprint or fix slice being assigned
- extract that sprint scope as written
- wrap it in `dev-template.xml.j2`
- send that slice to `comp`

`team-lead` must not:

- reinterpret sprint scope
- rewrite deliverables into a narrower or broader task
- adjudicate design intent inside the assignment
- replace the plan with a team-lead summary

The correct workflow is:

1. read the plan
2. extract the sprint slice
3. send that slice through the dev template

The plan is the spec.

## Task Sequencing

`team-lead` must keep `comp`'s ATM inbox preloaded during phased work.

Required execution model:

- `comp` replies immediately when a task is read
- queued tasks get a receipt message, not an `atm ack`
- `atm ack` happens only when that task becomes active and execution starts
- queued assignments execute in order received unless a task explicitly says
  `INTERRUPT CURRENT TASK`
- for phased work, fixes are handled from earliest sprint to latest sprint
  before later sprint work starts
- `team-lead` must queue the next known task as soon as the current task is
  started
- do not wait for task completion or validation before queueing the next known
  task
- failure to queue follow-on work can stall the phase and is a workflow failure
- `comp` prioritizes queued work using the assignment/template rules, not ad hoc
  nudges

## Interrupt Policy

`INTERRUPT CURRENT TASK` is rare.

Valid interrupt reasons:

- `comp` is working from incorrect instructions
- `comp` is on the wrong branch or worktree
- `comp`'s current work conflicts with another agent's work
- continuing the current task would produce invalid output because the task
  basis is wrong

Not valid interrupt reasons:

- normal dev/QA loop findings
- ordinary sprint fix work
- a new QA finding on another branch/worktree
- curiosity or a status check
- `team-lead` preference to reprioritize work already correctly queued

Do not interrupt for normal dev/QA loop work. Queue the fix and let `comp`
reach it in order.

## Nudge Text

Nudges must be short and protocol-only.

- Do not restate deliverables, acceptance criteria, or plan content in a nudge.
- Do not expand the Jinja2 task assignment into the nudge text.
- Nudges exist to restore queue/ack/start behavior, not to resend the task.
- Long narrative nudges reduce traceability and can break inbox acknowledgement
  discipline.

Typical nudge:

```bash
tmux send-keys -t sc-compose:1.2 "check atm for <TASK-ID>" Enter; sleep 0.5;
tmux send-keys -t sc-compose:1.2 "" Enter
```

Urgent nudge:

```bash
tmux send-keys -t sc-compose:1.2 "check atm IMMEDIATELY for <TASK-ID>" Enter; sleep 0.5;
tmux send-keys -t sc-compose:1.2 "" Enter
```

Use the urgent nudge rarely. It is for true interrupt conditions only, not
normal QA/fix traffic.

## Sprint Flow

1. `team-lead` sends a sprint assignment to `comp` using
   `dev-template.xml.j2`.
2. `comp` receives or ACKs according to queue state, then implements, commits,
   pushes, and reports the branch + SHA.
3. `team-lead` opens/updates the PR.
4. `team-lead` assigns QA to `quality-mgr` using `qa-template.xml.j2`.
5. `quality-mgr` launches the required reviewers for the review type.
6. If QA passes and CI is green, merge proceeds.
7. If QA fails, `team-lead` routes the fixes back to `comp`.

## CI

Use standard GitHub CLI for PR checks:

- `gh pr checks <PR> --watch`
- `gh pr view <PR> --json mergeStateStatus,reviewDecision`

Do not assume ATM-specific PR monitoring commands exist in this repo.

## Worktrees

Create sprint worktrees with normal git worktree commands when needed:

```bash
git worktree add ../<repo>-worktrees/<branch-name> -b <branch-name> <base-branch>
```

## Quality Manager Spawn

Spawn once per phase as a named teammate:

```json
{
  "subagent_type": "quality-mgr",
  "name": "quality-mgr",
  "team_name": "sc-compose",
  "prompt": "You are quality-mgr for the active sc-compose phase. You will receive plan, sprint, and phase-ending QA assignments from team-lead. Re-read .claude/agents/quality-mgr.md for every assignment. Launch every reviewer with run_in_background=true. Do not perform the review inline."
}
```

## Team-lead To quality-mgr

Always use the Jinja2 QA template and set `review_type` explicitly:

- `plan_gate`
- `sprint_review`
- `phase_ending_review`

The template must carry:

- review type
- worktree
- PR number when applicable
- deliverables
- references / design docs
- changed scope
- touched SSOT sections
- optional known findings to re-check
- optional fixed findings to confirm

## Review-Type Rules

### Plan Gate

Use for:

- requirements updates
- sprint plans
- phase plans
- checklist/status corrections

Expected reviewers:

- `req-qa`
- `arch-qa`

### Sprint Review

Use for:

- sprint completion QA
- fix-pass QA
- re-run QA after findings are addressed

Expected reviewers:

- `rust-qa-agent`
- `req-qa`
- `arch-qa`
- `simplification-reviewer`

### Phase-Ending Review

Use for:

- integration branch readiness
- whole-phase closeout review

Expected reviewers:

- `rust-qa-agent`
- `req-qa`
- `arch-qa`
- `simplification-reviewer`
- `test-auditor`

## Workflow

1. `comp` replies immediately when a new assignment is read.
2. if the assignment is not starting yet, `comp` reports it as queued behind
   the current task and continues active work.
3. when a queued task becomes active, `comp` runs `atm ack` and sends a start
   message with task id + branch/worktree.
4. as soon as `comp` starts task `N`, `team-lead` queues the next known task.
5. `comp` completes the task and reports branch + SHA.
6. `team-lead` opens PR and starts CI monitoring.
7. `team-lead` creates the next dev worktree for `comp`.
8. `team-lead` reads the active plan, extracts the next sprint slice verbatim,
   and sends that sprint assignment to `comp`.
9. `team-lead` sends the QA assignment to `quality-mgr` using
   `qa-template.xml.j2` with the correct `review_type`.
10. `quality-mgr` launches reviewers per its own prompt and returns one
    consolidated report.
11. `team-lead` schedules fixes if needed.
12. merge only after QA pass and CI green.

## Anti-Patterns

- Do not hardcode reviewer selection in `team-lead` workflow.
- Do not choose reviewers in the template instead of `quality-mgr`.
- Do not rewrite sprint scope before assigning it to `comp`.
- Do not summarize the plan when the sprint can be extracted directly.
- Do not treat `team-lead` interpretation as authoritative over the plan text.
- Do not assume every newly delivered assignment should start immediately.
- Do not use `atm ack` as a synonym for "message received."
- Do not interrupt an in-progress sprint on another worktree for normal dev/QA
  loop work.
- Do not schedule a later-sprint task ahead of an earlier-sprint fix in the
  same phase unless the assignment explicitly overrides queue order.
- Do not wait for a task to finish before queueing the next known task.
- Do not expand task content into a nudge.
