---
id: S.4
title: JSON Capability Seams
status: planned
branch: sprint/s-4-json-capability-seams
worktree: ../sc-compose-worktrees/sprint/s-4-json-capability-seams
target: integrate/phase-s
---

# Sprint S.4 — JSON Capability Seams

## Goal

Decompose JSON-capability dispatch into private helpers and exhaustively freeze
the existing command matrix, without changing clap arguments, command variants,
or JSON-mode behavior. This closes S-T6.

## Hard Dependencies

- `integrate/phase-s` exists from `develop` before this sprint branch exists.
- No hard code dependency on S.1–S.3; merge-forward the latest integration
  branch before implementation and submission.

## Exact Targets

- `crates/sc-compose/src/cli/capability.rs`
- `crates/sc-compose/src/cli/mod.rs`
- existing capability unit-test locations only
- `docs/plans/phase-S.md`

## Deliverables

- Private capability helpers replacing nested subcommand checks.
- Exhaustive table-driven regression cases for every existing `Examples`,
  `Templates`, `Reports`, and `Bead` subcommand.

## Required Work

- Do not change CLI flags, `Command` variants, clap configuration, or the
  existing JSON-mode decision for any command.
- Follow `CLAUDE.md` Rule 2: remain in the CLI adapter with no new dependency.
- **Production-ready closure:** every listed capability helper and every
  committed command-matrix regression case must land in this sprint; a partial
  subcommand matrix does not close S-T6.

## Explicit Code Samples

```rust
fn command_wants_json(command: &Command) -> bool;
```

The function remains exhaustive over the existing CLI tree; no public trait or
clap-argument redesign is authorized merely to reduce CCN.

## This Sprint Does Not Close

- Template-lint refactoring (S.2) or checked-validation work (S.3).
- Guardrail work (S.5–S.7) or runner work (S.8).

## Acceptance Criteria

- [ ] Every existing command variant has a JSON-capability regression case.
- [ ] The full existing command matrix returns the same JSON-capability result.
- [ ] No public CLI grammar, `Command` variant, or dependency changes occur.

## gh-stack Workflow

```bash
git switch integrate/phase-s
git pull --ff-only origin integrate/phase-s
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base integrate/phase-s sprint/s-4-json-capability-seams
git add crates/sc-compose/src/cli/capability.rs crates/sc-compose/src/cli/mod.rs crates/sc-compose/tests docs/plans/phase-S.md docs/phase-S/sprint-s-4-json-capability-seams.md
git commit -m "refactor(cli): isolate JSON-capability seams"
gh stack submit --auto
gh stack view --json
gh stack merge <sprint-s-4-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-compose --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
