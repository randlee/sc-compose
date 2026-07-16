# Git Workflows

## Branch Policy

- Use a `main` / `develop` git flow.
- Keep the primary checkout on `main` or `develop`, not feature branches.
- Create feature branches from `develop`.
- Open normal pull requests against `develop`.
- Merge `develop` into `main` for release readiness and release publication.
- Use git worktrees when parallel tasks need isolated branches.

## Naming Convention

- Phase plan doc: `docs/phase-X/phase-X-plan.md` (e.g. `docs/phase-C/phase-C-plan.md`).
- Sprint doc: `docs/phase-X/sprint-X-#-<description>.md` (e.g.
  `docs/phase-C/sprint-C-2-python-release-train.md`) — phase letter matches the
  phase's own case, dash-separated from the sprint number, dash-separated from
  a short kebab-case description.
- Sprint numbers are contiguous within a phase, starting at 1, with no gaps.
  See `.claude/skills/plan-hardening/sprint-planning-guidelines.md`.
- Branch names are always lowercase, regardless of the phase letter's case in
  docs.
- Integration branch: `integrate/phase-x` (e.g. `integrate/phase-c`).
- Sprint branch: `sprint/x-#-<description>` (e.g.
  `sprint/c-2-python-release-train`), matching its sprint doc's number and
  description, lowercased.
- This naming convention applies going forward; already-shipped phase/sprint
  docs and branches (phases A and B) are not retroactively renamed.

## Pull Request Rule

Every change should follow:
1. Create a feature branch from `develop`.
2. Run local validation.
3. Push the branch.
4. Open a PR to `develop`.
5. Wait for CI and review before merge.

## Release Rule

- Release tags must come from `main`, not from `develop` or a feature branch.
- Publishing must follow the order in `release/publish-artifacts.toml`.
- Release automation must validate publish order and version alignment before
  any tag or publish step runs.

## Worktree Rule

- Do not switch the main checkout away from `main` for sprint work.
- Create dedicated worktrees for long-running or parallel tasks.
- Remove worktrees only after the user approves cleanup.
