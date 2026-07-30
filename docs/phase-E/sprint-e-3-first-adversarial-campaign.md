---
id: E.3
title: First Adversarial Campaign And Regression Closure
status: complete
branch: sprint/e-3-first-adversarial-campaign
worktree: ../sc-compose-worktrees/sprint/e-3-first-adversarial-campaign
target: integrate/phase-e
---

# Sprint E.3 — First Adversarial Campaign And Regression Closure

## Goal

Run the first real, bounded adversarial campaign against the recursive-input
and rendering contracts, then turn every confirmed in-scope failure into a
deterministic regression test. This sprint is the proof that the E.2
coordinator and probes execute against the product; a contract-only or
dry-run result cannot satisfy the exit gate.

## Hard Dependencies

- [Phase E plan](./phase-E-plan.md)
- [Sprint E.1 — Recursive Structured Input Support](./sprint-e-1-recursive-structured-input.md)
- [Sprint E.2 — Adversarial Fuzzing Workflow](./sprint-e-2-adversarial-fuzzing.md)
- the registered skill, coordinator, and probe artifacts from commits
  `5fb6cb2` / `63d3c83`
- [Issue #157 implementation plan](../plan-157-nested-array-var-file.md)
- the four committed Issue #157 example fixtures

E.3 is blocked until E.2's structural contracts are reviewable. It unblocks
quality-mgr review of a campaign with actual worker evidence and any
follow-on bug-fix sprint required by a confirmed finding.

## Exact Targets

- `.claude/skills/adversarial-fuzzing/SKILL.md` (only for a contract defect
  discovered by the campaign)
- `.claude/agents/sc-adversarial-fuzz-coordinator.md` and
  `.claude/agents/sc-adversarial-fuzz-probe.md` (only for a contract defect)
- `crates/sc-composer/src/**` and its unit tests for library regressions
- `crates/sc-compose/tests/**` for CLI, ingress, diagnostic, or output
  regressions
- `examples/changelog-categories.md.j2`
- `examples/changelog-categories.sample-vars.json`
- `examples/jagged-array-values.md.j2`
- `examples/jagged-array-values.sample-vars.json`
- `docs/phase-E/evidence/e-3-adversarial-campaign.json`
- `docs/phase-E/sprint-e-3-first-adversarial-campaign.md`

The evidence file is the durable campaign record. It is committed so a
quality-mgr reviewer can audit the seed, worker execution, candidates, and
promotion decisions without relying on terminal scrollback.

## Campaign Protocol

Run one `full` campaign from the E.3 worktree with:

- a fixed, recorded seed;
- no more than four concurrent workers;
- one `shape-probe`, `template-probe`, `boundary-probe`, and
  `differential-probe` worker;
- a recorded case budget and per-worker timeout;
- real generated inputs/templates executed against the current worktree;
- deterministic correlation-ID ordering in the aggregate report.

The campaign is not real if workers only validate their JSON contract, emit
sample output, or inspect files without executing generated cases against
sc-compose. A worker timeout, launch failure, or partial result must remain
visible in the evidence file; it cannot be converted into a successful
no-finding result.

## Deliverables

- `E3-D1` — execute and record the bounded full campaign, including seed,
  target, baseline, limits, worker IDs, commands, exit status, and timing.
- `E3-D2` — capture every candidate and worker failure in the evidence
  envelope; do not discard intentional boundaries or inconclusive results.
- `E3-D3` — minimize each candidate and reproduce it at least three times,
  recording the minimized template/input and exact oracle.
- `E3-D4` — classify every candidate as `confirmed_bug`,
  `intentional_boundary`, or `inconclusive`; quality-mgr owns the final
  severity and next-owner fields.
- `E3-D5` — promote every confirmed in-scope bug to a deterministic test in
  the owning crate. Workers may not edit production code or silently commit a
  fix; a required runtime fix becomes a separately owned follow-on sprint.
- `E3-D6` — rerun the relevant workspace, fixture, boundary, and structural
  checks after test promotion and update the durable evidence record.

## Candidate Classification And Promotion

Use the E.2 finding contract. A stable panic, hang, wrong valid output,
broken metamorphic relation, unexplained JSON/YAML divergence, or violated
stable boundary is a `confirmed_bug`. A documented rejection of malformed or
unsupported input is an `intentional_boundary`. An unstable result or one
without a defensible oracle is `inconclusive`.

Promotion requires three successful reproductions after minimization and a
clear expected result. Library behavior belongs in `sc-composer` unit tests;
CLI behavior, diagnostics, ingress, and rendered artifacts belong in
`sc-compose` integration tests. Do not promote flaky tests or treat a
no-finding result as evidence when any worker failed or timed out.

## Acceptance Criteria

- A full campaign executes generated cases against the current worktree with
  all four required workers, a fixed seed, explicit case budget, and timeout.
- The committed evidence file accounts for every worker, case batch, timeout,
  candidate, and classification; no failure is hidden by aggregation.
- Each candidate is minimized and reproduced three times before promotion or
  an `inconclusive` classification.
- Every confirmed in-scope bug has a deterministic regression test, or the
  evidence names the follow-on owner and explains why it is outside this
  sprint's scope.
- Existing Issue #157 fixtures and repository boundary expectations remain
  green after promoted tests are added.
- `quality-mgr` can review the evidence and distinguish successful execution
  from a no-finding result.

## Required Validation

Run from the E.3 worktree:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo test -p sc-compose --test repo_boundaries
python3 <skill-creator-root>/scripts/quick_validate.py .claude/skills/adversarial-fuzzing
git diff --check
```

The skill-creator path is operator-specific and non-gating when unavailable;
registry/path checks, campaign evidence completeness, and the Rust gates are
gating. Run both Issue #157 fixture commands and record their output in the
evidence file.

## Closure Notes

- The real full campaign is recorded in
  `docs/phase-E/evidence/e-3-adversarial-campaign.json`.
- Four concurrent workers (`shape-probe`, `template-probe`,
  `boundary-probe`, and `differential-probe`) executed 12 generated cases each
  with seed `157` and a 120-second per-worker timeout.
- All 48 cases completed successfully. Three minimized boundary candidates
  reproduced four times each and were classified as intentional boundaries:
  malformed var-file, top-level sequence, and nested non-string YAML key.
- No confirmed product bugs were found, so no regression tests or runtime fixes
  were promoted. Workspace and fixture validation remained green.
