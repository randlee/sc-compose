---
id: O.4
title: Six-Template Migration, Release Corpus, and Fuzz Oracle Gate
phase: N
status: planned
branch: sprint/o-4-migration-release-fuzz-gate
worktree: ../sc-compose-worktrees/sprint/o-4-migration-release-fuzz-gate
target: integrate/phase-o
---

# Sprint O.4 — six-template migration, release corpus, and fuzz oracle gate

## Goal

Migrate the six known affected templates, inventory likely downstream
repositories, and prove that the release candidate catches both the legacy
compatibility shape and the secure auto shape before 1.4.1 is published.

## Dependencies and parallelism

Requires O.2 and O.3 merged and QA-approved. O.4 is the final phase sprint; it
must not claim release readiness from an unmerged or unvalidated parent. The
inventory/design work may begin during O.2/O.3, but migration and release
evidence must run against their merged behavior.

## Exact targets

- `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`
- `.claude/assets/sc-rust/quality-mgr/templates/rust-qa-assignment.json.j2`
- `.claude/assets/sc-rust/quality-mgr/templates/rust-service-hardening-assignment.json.j2`
- `.claude/skills/codex-orchestration/arch-qa-assignment.json.j2`
- `.claude/skills/codex-orchestration/flaky-test-qa-assignment.json.j2`
- `.claude/skills/codex-orchestration/req-qa-assignment.json.j2`
- `.claude/skills/adversarial-fuzzing/SKILL.md`
- `site/reports/` release evidence location
- `docs/requirements.md`, changelog, and migration documentation

External repository inventory and ATM-core adapter changes are handoff
artifacts, not source edits in sc-compose.

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
7. Update the adversarial-fuzzing campaign contract so every successful JSON
   render is parsed and every campaign records release binary/commit, mode,
   exact template, exact context, and parser result.
8. Re-run the campaign against both source forms, including the template-init
   round trip and the six-template corpus.
9. Inventory the 20–30 likely consumer repositories, report each template
   contract finding, and provide a migration command/example rather than
   silently assuming all consumers are fixed.
10. Record 1.4.1 rollout, deprecation, and future absent-mode-default decision.

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
- release corpus scan reports every old quoted placeholder;
- fuzz workers run secure-auto, legacy-compatibility, mode-mismatch,
  template-init, output-parser, and release-corpus probes;
- successful JSON cases include parser-backed PASS evidence;
- no promoted finding is accepted without a minimal template, input,
  expected oracle, observed result, reproduction count, and requirement/ADR
  trace.

## Deliverables

- six-template migration;
- representative semantic fixtures;
- updated adversarial-fuzzing skill/oracle;
- release-candidate fuzz and lint reports;
- downstream inventory and ATM-core handoff;
- 1.4.1 release readiness recommendation.

## Acceptance criteria

- [ ] Six known templates are auto-mode and semantically parseable, or have an
      explicit documented legacy exception with a migration owner.
- [ ] Release-candidate output never silently emits malformed JSON.
- [ ] Fuzz campaign would fail on the original 1.4 regression and pass both
      supported modes only when their correct contracts are met.
- [ ] The 20–30 repository inventory is actionable and includes evidence.
- [ ] ATM-core has exact checked-render integration instructions.
- [ ] Changelog and migration documentation are complete.
- [ ] All workspace, lint, and release-candidate gates pass.

## Sc-lint cleanup and QA handoff

Run the full sc-lint/template-contract profile against the final migration and
fuzz evidence commit. Fix minor findings locally. For remaining findings,
create fix worktrees from this sprint's final commit, grouped by independent
rule class and owning crate; do not create one worktree per string warning or
mix unrelated refactors with migration semantics. Send team-lead each parent
commit, fix path, finding class, evidence, tests, and fix commit. Team-lead
creates PRs and routes them to quality-mgr. O.4 cannot close until QA approves,
all required fixes merge, and the release corpus is re-run on the merged
parent.

## Validation

```text
cargo test --workspace
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
sc-compose lint --target template-contracts --root . --json
just lint
adversarial-fuzzing release-candidate campaign with parser-backed JSON oracle
git diff --check
```
