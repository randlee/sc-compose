---
id: S.3
title: Checked Validation Seams
status: planned
branch: sprint/s-3-checked-validation-seams
worktree: ../sc-compose-worktrees/sprint/s-3-checked-validation-seams
target: integrate/phase-s
---

# Sprint S.3 — Checked Validation Seams

## Goal

Separate private checked-validation report construction from presentation and
exit choice, while retaining the existing composition/check authority and
byte-for-byte compatible text/JSON results. This closes S-T5.

## Hard Dependencies

- `integrate/phase-s` exists from `develop` before this sprint branch exists.
- No hard code dependency on S.1 or S.2; merge-forward the latest integration
  branch before implementation and submission.

## Exact Targets

- `crates/sc-compose/src/commands/compose.rs`
- `crates/sc-compose/tests/cli/validate.rs`
- `crates/sc-compose/tests/json_cli/validate.rs`
- existing checked-validation unit-test locations only
- `docs/plans/phase-S.md`

## Deliverables

- Private checked-report construction separate from presentation and exit choice.
- Fixtures that freeze existing checked-validation JSON/text output and status
  for every existing report state.

## Required Work

- Keep `compose_with_observer` and `check_rendered_output_with_meta` as the
  authoritative operations.
- Do not change validation semantics, report schema, CLI flags, or exit codes.
- Follow `CLAUDE.md` Rule 2: `sc-compose` remains an adapter over
  `sc-composer`.
- **Production-ready closure:** every listed report/presentation/exit seam and
  its committed regression coverage must land in this sprint; partial report
  state coverage does not close S-T5.

## Explicit Code Samples

```rust
// Private classification seam; exact concrete helper name may vary.
fn checked_validation_report(/* existing inputs */) -> RenderCheckReport;
```

The implementation may reorganize private report construction only; existing
presentation and status behavior remain the contract under test.

## This Sprint Does Not Close

- Template-lint refactoring (S.2).
- JSON-capability dispatch (S.4), guardrail work (S.5–S.7), or runner work (S.8).

## Acceptance Criteria

- [ ] Existing CLI integration tests show unchanged text/JSON output and exit
  codes for `validate --check-render` paths.
- [ ] Report construction, presentation, and exit choice have independently
  exercisable tests for every existing report state.
- [ ] No `sc-composer` public API or dependency direction changes.

## gh-stack Workflow

```bash
git switch integrate/phase-s
git pull --ff-only origin integrate/phase-s
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base integrate/phase-s sprint/s-3-checked-validation-seams
git add crates/sc-compose/src/commands/compose.rs crates/sc-compose/tests/cli/validate.rs crates/sc-compose/tests/json_cli/validate.rs docs/plans/phase-S.md docs/phase-S/sprint-s-3-checked-validation-seams.md
git commit -m "refactor(validate): isolate checked-validation seams"
gh stack submit --auto
gh stack view --json
gh stack merge <sprint-s-3-pr-number> --yes --merge
```

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy -p sc-compose --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose`
- `cargo test --workspace`
- `just lint`
- `git diff --check`
