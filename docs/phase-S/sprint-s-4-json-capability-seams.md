---
id: S.4
title: JSON Capability Seams
status: complete
branch: sprint/s-4-json-capability-seams
worktree: ../sc-compose-worktrees/sprint/s-4-json-capability-seams
target: sprint/s-3-checked-validation-seams
---

# Sprint S.4 — JSON Capability Seams

## Goal

Decompose JSON-capability dispatch into private helpers and exhaustively freeze
the existing command matrix, without changing clap arguments, command variants,
or JSON-mode behavior. This closes S-T6.

## Hard Dependencies

- S.3 is this branch's required `gh stack` parent. There is no functional code
  dependency on S.1–S.3; the parent keeps this PR incremental.

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

- [x] Every existing command variant has a JSON-capability regression case.
- [x] The full existing command matrix returns the same JSON-capability result.
- [x] No public CLI grammar, `Command` variant, or dependency changes occur.

## gh-stack Workflow

```bash
# The phase plan added this branch directly on top of S.3.
git config rerere.enabled true
git config remote.pushDefault origin
git add crates/sc-compose/src/cli/capability.rs crates/sc-compose/src/cli/mod.rs crates/sc-compose/tests docs/plans/phase-S.md docs/phase-S/sprint-s-4-json-capability-seams.md
git commit -m "refactor(cli): isolate JSON-capability seams"
gh stack submit --auto
gh pr ready <sprint-s-4-pr-number>
gh stack view --json
# Do not merge an individual sprint layer; phase close merges the full stack.
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-compose --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
