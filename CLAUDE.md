# Claude Instructions for sc-compose

## Critical Workflow Rule

Do not switch the main checkout away from `main` for sprint work.

- Keep the primary repo checkout on `main`
- Use git worktrees for feature work when parallel branches are needed
- Prefer short-lived feature branches targeting `main`

## Project Overview

`sc-compose` is a standalone template-composition workspace.

It contains:
- `sc-composer`: reusable rendering library
- `sc-compose`: CLI wrapper around the library

This repo is intentionally independent from ATM. Do not introduce
`agent-team-mail-*` dependencies or ATM path/runtime assumptions.

## Key Documents

- [`docs/requirements.md`](./docs/requirements.md)
- [`docs/architecture.md`](./docs/architecture.md)
- [`docs/project-plan.md`](./docs/project-plan.md)
- [`docs/git-workflows.md`](./docs/git-workflows.md)
- [`docs/cross-platform-guidelines.md`](./docs/cross-platform-guidelines.md)
- [`docs/team-protocol.md`](./docs/team-protocol.md)
- [`.claude/skills/rust-development/guidelines.txt`](./.claude/skills/rust-development/guidelines.txt)

## Boundary Rules

1. `sc-composer` must remain a pure library.
2. `sc-compose` may depend on `sc-composer` and standalone observability crates only.
3. Do not read `ATM_HOME`.
4. Any ATM integration belongs in ATM adapters, not in this repo.

## Team Communication

If this repo is being run with ATM team workflow enabled, follow
[`docs/team-protocol.md`](./docs/team-protocol.md) for all ATM messages.
