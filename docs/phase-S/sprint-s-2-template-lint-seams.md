---
id: S.2
title: Template Lint Seams
status: complete
branch: sprint/s-2-template-lint-seams
worktree: ../sc-compose-worktrees/sprint/s-2-template-lint-seams
target: integrate/phase-s
---

# Sprint S.2 — Template Lint Seams

## Goal

Separate private template-lint source analysis, repository traversal, and
report assembly while preserving the existing scanner, lint codes, locations,
command grammar, and output. This closes S-T4.

## Hard Dependencies

- `integrate/phase-s` exists from `develop` before this sprint branch exists.
- No hard code dependency on S.1; merge-forward the latest integration branch
  before implementation and submission.

## Exact Targets

- `crates/sc-compose/src/commands/template_lint.rs`
- `crates/sc-compose/tests/template_contracts_cli.rs`
- existing template-lint unit-test locations only
- `docs/plans/phase-S.md`

## Deliverables

- Private seams for source analysis, repository traversal, and report assembly.
- Regression coverage showing the existing `sc_composer::template_scanner`
  remains the only scanner and preserves lint codes and locations.

## Required Work

- Do not add a parser, change CLI flags, alter `Command` variants, or move
  lint policy into `sc-composer`.
- Follow `CLAUDE.md` Rule 2: `sc-compose` remains an adapter and must not
  reverse the `sc-composer` dependency.

## Explicit Code Samples

```rust
// Private-only organization; existing scanner remains authoritative.
fn analyze_template_source(/* existing inputs */) -> /* existing lint results */;
fn assemble_lint_report(/* existing inputs */) -> /* existing lint report */;
```

The helpers may retain concrete private input types, but may not introduce a
second parser or a new public lint-policy API.

## This Sprint Does Not Close

- Checked-validation output/exit handling (S.3).
- JSON-capability dispatch (S.4), guardrail work (S.5–S.7), or runner work (S.8).

## Acceptance Criteria

- [ ] `template_lint` uses exactly the existing scanner and retains diagnostic
  codes and locations.
- [ ] Source analysis, traversal, and report assembly have independently
  exercisable regression coverage.
- [ ] No `sc-composer` public API, CLI grammar, or dependency direction changes.

## gh-stack Workflow

```bash
git switch integrate/phase-s
git pull --ff-only origin integrate/phase-s
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base integrate/phase-s sprint/s-2-template-lint-seams
git add crates/sc-compose/src/commands/template_lint.rs crates/sc-compose/tests docs/plans/phase-S.md docs/phase-S/sprint-s-2-template-lint-seams.md
git commit -m "refactor(lint): isolate private template-lint seams"
gh stack submit --auto
gh stack view --json
gh stack merge <sprint-s-2-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-compose --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
