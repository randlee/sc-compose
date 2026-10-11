# Repository Quality Policy

This file contains repository-specific QA policy. Reusable agents and skills
read this file rather than embedding repository names, commands or
exceptions in their own prompts. The orchestration templates name this file
as `policy_path`.

## Repository Baseline

- Default comparison branch: `develop`
- Requirements index: `docs/requirements.md`
- Architecture index: `docs/architecture.md`; ADRs are
  `docs/adrs/NNNN-<slug>.md`
- Project plan: `docs/project-plan.md`
- Developer roster: `.atm.toml` and the ATM roster (`atm members --team
  sc-compose`) are authoritative for live identities.

## Reviewer Policy

- Initial implementation review (sprint QA-1): `req-qa`, `arch-qa`,
  `rust-qa-agent`, `rust-best-practices-agent`, and
  `rust-service-hardening-agent`
- Fix verification (sprint QA-2 and later): only the reviewer that filed a
  finding verifies its fix, scoped to that finding
- Plan review QA-1: `plan-scope-reviewer`, `req-qa`, `arch-qa`
- Phase-end review: the initial implementation set plus `flaky-test-qa`
- Every QA round requires an open PR; the rendered report is posted to it

## Validation Policy

- Lint: `just lint`
- Tests: `just test`

## Plan Naming

| Thing | Form | Example |
|---|---|---|
| Phase id | next unused letter, lower case in branches | `t` |
| Plan directory | `docs/phase-<PHASE>/` | `docs/phase-S/` |
| Phase branch | `integrate/phase-<phase>`, cut from `develop` | `integrate/phase-s` |
| Sprint branch | `sprint/<phase>-<n>-<slug>` | `sprint/s-2-slug` |
| Fix layer | `fix/<phase>-<n>-<slug>` | `fix/s-2-qa1` |

## Governed Interfaces

None declared.

## Repository Exceptions

None.
