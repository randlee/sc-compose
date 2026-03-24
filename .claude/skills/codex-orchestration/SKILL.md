---
name: codex-orchestration
description: Orchestrate phased work where arch-ctm is the sole developer and quality-mgr enforces the QA gate for sc-compose.
---

# Codex Orchestration

This skill defines a lightweight phase workflow for standalone repos like
`sc-compose`.

## Model

- `arch-comp` coordinates (repo lead)
- `ccomp` is the sole developer
- `quality-mgr` runs QA after each sprint delivery

The repo may use either:
- direct sprint PRs to `main`, or
- an explicit `integrate/phase-{P}` branch when a phase needs a staging branch

Use the `pr_target` in the assignment as the source of truth.

## Preconditions

Before starting:
1. `docs/requirements.md`, `docs/architecture.md`, and `docs/project-plan.md`
   describe the sprint.
2. The target branch for the sprint is chosen (`main` or `integrate/phase-{P}`).
3. A worktree exists for the sprint branch.
4. `quality-mgr`, `rust-qa-agent`, `req-qa`, and `arch-qa` are available.

## Sprint Flow

1. `arch-comp` sends a sprint assignment to `ccomp` using `dev-template.xml.j2`.
2. `ccomp` ACKs, implements, commits, pushes, and reports the branch + SHA.
3. `arch-comp` opens/updates the PR.
4. `arch-comp` assigns QA to `quality-mgr` using `qa-template.xml.j2`.
5. `quality-mgr` runs:
   - `rust-qa-agent`
   - `req-qa`
   - `arch-qa`
6. If QA passes and CI is green, merge proceeds.
7. If QA fails, `arch-comp` routes the fixes back to `ccomp`.

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

## Required Message Sequence

Every ATM task message must follow:
1. ACK
2. Work
3. Completion summary
4. Completion ACK by receiver
