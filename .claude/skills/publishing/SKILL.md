---
name: publishing
version: 0.1.0
description: >
  Standing rule for any sc-compose release/publish work: always route through
  the publisher agent, and always run a preflight on a release/* worktree
  before merging that branch to main.
---

# Publishing Skill

Trigger: any request to cut, prepare, preflight, or publish an sc-compose
release, or to merge a `release/*` branch into `main`.

## Rule 1 — Always use the `publisher` agent

Never run publish steps (tag creation, workflow dispatch, crates.io/PyPI/
Homebrew/winget/Scoop publication, credential-gated actions) directly via ad
hoc `gh`/`cargo publish`/shell commands, and never substitute a different
agent type for release orchestration. Route every release action through the
`publisher` agent (`.claude/agents/publisher.md`), which in turn delegates
each manifest channel to a `publisher-channel-worker` background agent per
`release/publish-artifacts.toml`.

This applies to preflight-only (read-only, no side effects) runs as well as
real publish runs — same agent, same contract, only the explicit
authorization scope differs.

## Rule 2 — Preflight on a `release/*` worktree before merging to main

Per `docs/git-workflows.md`'s Release Rule, release tags and publication only
ever come from `main`. Before a `release/*` branch is merged into `main`:

1. Create or use a dedicated `release/*` worktree (never preflight against
   the primary checkout or a feature branch).
2. Launch a `publisher` agent scoped to that worktree and candidate
   tag/commit, with explicit team-lead authorization for a preflight-only
   scenario (no tag, no workflow dispatch, no publish, no credential
   read/print/request).
3. Confirm every manifest channel's preflight result — including any
   `blocked` (evidence absent) vs `failed` (evidence present and negative)
   classification and each channel's `release_authorization` check — before
   approving the merge to `main`.
4. Only after the `release/*` -> `main` merge and a passing preflight does
   the publisher agent get authorized to proceed with an actual publish run.

Do not treat a green CI run or a `develop`-side QA pass as a substitute for
this preflight; it is a distinct gate that must run on the `release/*`
worktree itself, immediately before the `main` merge.

## Reference

- `.claude/agents/publisher.md` — publisher agent contract, Non-Negotiable
  Rules, Channel Gate delegation, Output Format.
- `.claude/agents/publisher-channel-worker.md` — per-channel worker contract.
- `release/publish-artifacts.toml` — manifest of channels and publish order.
- `docs/git-workflows.md` — Release Rule (branch/tag policy).
- `docs/eval/publishing/publish-kit-agent-eval-plan.md` — durable eval plan
  for verifying publisher/worker behavior without a real release.
