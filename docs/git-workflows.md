# Git Workflows

## Branch Policy

- Keep the primary checkout on `main`.
- Use short-lived feature branches for changes.
- Open pull requests against `main`.
- Use git worktrees when parallel tasks need isolated branches.

## Pull Request Rule

Every change should follow:
1. Create a feature branch from `main`.
2. Run local validation.
3. Push the branch.
4. Open a PR to `main`.
5. Wait for CI and review before merge.

## Release Rule

- Release tags must come from `main`, not from a feature branch.
- Publishing must follow the order in `release/publish-artifacts.toml`.
- Release automation must validate publish order and version alignment before
  any tag or publish step runs.

## Worktree Rule

- Do not switch the main checkout away from `main` for sprint work.
- Create dedicated worktrees for long-running or parallel tasks.
- Remove worktrees only after the user approves cleanup.
