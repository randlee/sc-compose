# Git Workflows

## Branch Policy

- Use a `main` / `develop` git flow.
- Keep the primary checkout on `main` or `develop`, not feature branches.
- Create feature branches from `develop`.
- Open normal pull requests against `develop`.
- Merge `develop` into `main` for release readiness and release publication.
- Use git worktrees when parallel tasks need isolated branches.

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
