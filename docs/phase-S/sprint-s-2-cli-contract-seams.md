---
id: S.2
title: CLI Contract Seams
status: complete
branch: sprint/s-2-cli-contract-seams
worktree: ../sc-compose-worktrees/sprint/s-2-cli-contract-seams
target: integrate/phase-s
---

# Sprint S.2 — CLI Contract Seams

## Goal

Separate private lint analysis, checked-validation report construction, and
JSON-capability dispatch while preserving command grammar, text/JSON output,
and exit codes. This closes S-T4, S-T5, and S-T6.

## Hard Dependencies

- `integrate/phase-s` exists.
- No hard code dependency on S.1; merge-forward the latest integration branch
  before implementation and submission.

## Exact Targets

- `crates/sc-compose/src/commands/template_lint.rs`
- `crates/sc-compose/src/commands/compose.rs`
- `crates/sc-compose/src/cli/capability.rs`
- existing `crates/sc-compose` CLI and command tests
- `docs/plans/phase-S.md`

## Deliverables

- Private `template_lint` source-analysis, repository-traversal, and
  report-assembly seams using the existing `template_scanner`.
- Checked-validation report construction separated from presentation and exit
  choice.
- Exhaustive JSON-capability tests for every `Examples`, `Templates`,
  `Reports`, and `Bead` subcommand.
- Fixtures freezing checked-validation JSON/text output and exit status.

## Required Work

- Do not add a parser, change CLI flags, alter `Command` variants, or move
  lint policy into `sc-composer`.
- Keep `compose_with_observer` and `check_rendered_output_with_meta` as the
  authoritative operations.
- Follow `CLAUDE.md` Rule 2: `sc-compose` remains an adapter and must not
  reverse the `sc-composer` dependency.

## Explicit Code Samples

```rust
// Private classification seam; exact concrete helper names may vary.
fn checked_validation_report(/* existing inputs */) -> RenderCheckReport;
fn command_wants_json(command: &Command) -> bool;
```

The second function remains exhaustive over the existing CLI tree; no public
trait or clap argument redesign is authorized merely to reduce CCN.

## This Sprint Does Not Close

- A rendering or validation semantic change.
- Extractor changes (S.1), boundary restructuring (S.3), or runner work (S.4).

## Acceptance Criteria

- [ ] Existing CLI integration tests show unchanged text/JSON output and exit
  codes for lint and `validate --check-render` paths.
- [ ] Every command variant has a JSON-capability regression case.
- [ ] `template_lint` uses exactly the existing scanner and retains diagnostic
  codes/locations.
- [ ] No `sc-composer` public API or dependency direction changes.

## gh-stack Workflow

```bash
git switch integrate/phase-s
git pull --ff-only origin integrate/phase-s
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base integrate/phase-s sprint/s-2-cli-contract-seams
git add crates/sc-compose/src/commands/template_lint.rs crates/sc-compose/src/commands/compose.rs crates/sc-compose/src/cli/capability.rs crates/sc-compose/tests docs/plans/phase-S.md docs/phase-S/sprint-s-2-cli-contract-seams.md
git commit -m "refactor(cli): isolate contract-preserving command seams"
gh stack submit --auto
gh stack view --json
gh stack merge <sprint-s-2-pr-number> --yes --merge

# Phase close only, after every Phase S sprint is merged into integrate/phase-s.
git switch develop
gh stack init --base develop integrate/phase-s
gh stack view --json
gh stack merge <phase-s-integration-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-compose --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
