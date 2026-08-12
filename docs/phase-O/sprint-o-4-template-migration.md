---
id: O.4
title: Six-Template Migration and Compatibility Fixtures
phase: O
status: planned
branch: sprint/o-4-template-migration
worktree: ../sc-compose-worktrees/sprint/o-4-template-migration
target: integrate/phase-o
---

# Sprint O.4 — six-template migration and compatibility fixtures

## Goal

Migrate the six known affected templates and establish semantic compatibility
fixtures for both the legacy source shape and the secure auto shape. Cross-
repository inventory, release-candidate fuzzing, and the release gate are
owned by O.5, not this migration sprint.

This is an implementation sprint for the six in-repository templates and
fixtures only. It does not produce the cross-repository inventory, fuzz
release report, or release-readiness decision; those are O.5 deliverables.

## Dependencies and parallelism

Requires O.2 and O.3 merged and QA-approved. O.4 may execute only after both
parent contracts are available. O.5 is the only sprint that may claim
cross-repository release readiness, and it starts after O.4 merges.

## Exact targets

- `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`
- `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2`
- `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2`
- `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2`
- `.claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2`
- `.claude/skills/codex-orchestration/req-qa-assignment.json.j2`
- `crates/sc-compose/tests/cli/templates.rs`
- `crates/sc-compose/tests/json_cli/templates.rs`
- `crates/sc-compose/tests/cli/render.rs`
- `crates/sc-compose/tests/json_cli/render.rs`
- `docs/requirements.md`
- `docs/migration-notes.md`
- `CHANGELOG.md`

External repository inventory, fuzz-skill changes, and ATM-core adapter
changes are O.5 or handoff artifacts, not source edits in this sprint.

## Required work

1. For each of the six templates, classify every interpolation as scalar,
   structured value, loop element, conditional fragment, or raw JSON.
2. Add `json_escape_mode: auto` and remove literal quotes only where the
   renderer owns a complete JSON string value.
3. Preserve and test intentionally structured/raw fields; do not mechanically
   strip every quote.
4. Render each template with representative values containing quotes,
   backslashes, Unicode, newline, empty strings, arrays, objects, nulls, and
   control characters where the field type permits them.
5. Parse complete outputs semantically and compare expected JSON values, not
   only snapshots.
6. Maintain explicit legacy fixtures proving unmodified old templates remain
   safely renderable during the compatibility window.
7. Record the six-template migration matrix, fixture contexts, semantic
   expected values, and explicit legacy exceptions in repository documentation.

## Required migration matrix

| Source shape | Expected O.4 action |
| --- | --- |
| scalar inside JSON quotes | remove quotes, auto mode |
| scalar array element inside quotes | remove quotes, auto mode |
| structured value | keep bare and verify type |
| raw JSON fragment | preserve only with explicit reviewed contract |
| conditional complete JSON value | test all statically reachable branches |
| ambiguous macro/include | legacy compatibility or refactor with fixture before auto |

## Required tests and evidence

- all six migrated templates pass `validate --lint`;
- all six render with representative contexts and parse as JSON;
- hostile string values cannot add keys or alter structure;
- legacy compatibility fixture emits deprecation warning but valid JSON;
- `template-init` generated JSON round-trips;
- no cross-repository release claim is made from O.4; O.5 owns that evidence;
- no promoted finding is accepted without a minimal template, input,
  expected oracle, observed result, reproduction count, and requirement/ADR
  trace.

## Deliverables

- six-template migration;
- representative semantic fixtures;
- migration and compatibility documentation;
- semantic fixture evidence for O.5 to consume.

## Acceptance criteria

- [ ] Six known templates are auto-mode and semantically parseable, or have an
      explicit documented legacy exception with a migration owner.
- [ ] Every affected interpolation has an explicit source-shape classification
      and migration/legacy decision.
- [ ] Six-template semantic fixtures cover representative and hostile values.
- [ ] O.5 has a precise handoff containing the six-template corpus, expected
      JSON values, and compatibility exceptions.
- [ ] Changelog and migration documentation are complete.
- [ ] ADR-0019 is accepted before implementation handoff.
- [ ] All workspace and targeted lint gates pass.

## Sc-lint cleanup and QA handoff

Run the full sc-lint/template-contract profile against the final migration
commit. Fix minor findings locally. For remaining findings, create fix
worktrees from this sprint's final commit, grouped by independent rule class
and owning crate; do not create one worktree per string warning or mix
unrelated refactors with migration semantics. Send team-lead each parent
commit, fix path, finding class, evidence, tests, and fix commit. Team-lead
creates PRs and routes them to quality-mgr. O.4 cannot close until QA approves
the migration and required fix PRs merge.

## Validation

```text
cargo test --workspace
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
sc-compose lint --target template-contracts --root . --json
just lint target=template-contracts
git diff --check
```
