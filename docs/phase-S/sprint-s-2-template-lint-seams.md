---
id: S.2
title: Template Lint Seams
status: complete
branch: sprint/s-2-template-lint-seams
worktree: ../sc-compose-worktrees/sprint/s-2-template-lint-seams
target: sprint/s-1-extractor-internal-seams
---

# Sprint S.2 — Template Lint Seams

## Goal

Separate private template-lint source analysis, repository traversal, and
report assembly while preserving the existing scanner, lint codes, locations,
command grammar, and output. This closes S-T4.

## Hard Dependencies

- S.1 is this branch's required `gh stack` parent. There is no functional code
  dependency on S.1; the parent exists to make this PR an incremental layer.

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
- **Production-ready closure:** every listed lint seam and its committed
  regression coverage must land in this sprint; partial source, traversal, or
  report coverage does not close S-T4.

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

- [x] `template_lint` uses exactly the existing scanner and retains diagnostic
  codes and locations.
- [x] Source analysis, traversal, and report assembly have independently
  exercisable regression coverage.
- [x] No `sc-composer` public API, CLI grammar, or dependency direction changes.

## gh-stack Workflow

```bash
# The phase plan added this branch directly on top of S.1.
git config rerere.enabled true
git config remote.pushDefault origin
git add crates/sc-compose/src/commands/template_lint.rs crates/sc-compose/tests docs/plans/phase-S.md docs/phase-S/sprint-s-2-template-lint-seams.md
git commit -m "refactor(lint): isolate private template-lint seams"
gh stack submit --auto
gh pr ready <sprint-s-2-pr-number>
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
