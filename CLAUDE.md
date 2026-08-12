# Claude Instructions for sc-compose

## Critical Workflow Rule

Do not switch the main checkout away from `main` for sprint work.

- Keep the primary repo checkout on `main`
- Use git worktrees for feature work when parallel branches are needed
- Prefer short-lived feature branches targeting `develop`

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
3. `bindings/python` is a Python-facing adapter package that may depend on `sc-composer` only.
4. `bindings/python` must not depend on `sc-compose`, `sc-observability`, or ATM-specific crates.
5. `sc-composer` must not depend on `bindings/python`.
6. Do not read `ATM_HOME`.
7. Any ATM integration belongs in ATM adapters, not in this repo.
8. `sc-sha` is a pure computation crate. It may depend only on approved
   hashing/encoding dependencies, and may be depended on only by
   `sc-composer` and `bindings/sc-sha-python`. See
   [ADR-0018](./docs/adrs/0018-sc-sha-hash-ownership.md).
9. `bindings/sc-sha-python` is a Python-facing adapter for `sc-sha` only. It
   may depend on published `sc-sha` plus PyO3/maturin packaging dependencies
   only. See [ADR-0018](./docs/adrs/0018-sc-sha-hash-ownership.md).

## Team Communication

If this repo is being run with ATM team workflow enabled, follow
[`docs/team-protocol.md`](./docs/team-protocol.md) for all ATM messages.
