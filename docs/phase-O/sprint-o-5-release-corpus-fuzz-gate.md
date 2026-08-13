---
id: O.5
title: Cross-Repository Release Corpus and Fuzz Gate
phase: O
status: complete
branch: sprint/o-5-release-corpus-fuzz-gate
worktree: ../sc-compose-worktrees/sprint/o-5-release-corpus-fuzz-gate
target: integrate/phase-o
merge: PR #430 at a2a5b2d; release-waiver evidence in 14f6a3b
---

# Sprint O.5 — cross-repository release corpus and fuzz gate

## Goal

Find the same JSON interpolation contract failure in the repositories that
consume these templates, prove the 1.4.1 candidate rejects malformed output,
and make parser-backed fuzz evidence a release gate. This sprint consumes the
O.2/O.3 contracts and O.4's six-template migration fixtures; it does not
change the renderer or silently edit external repositories.

This is a verification/release-gate sprint, not a Rust implementation sprint:
it produces campaign instructions, inventories, and evidence, but no
production executable artifact. Any renderer or scanner fix found in the
campaign is handed off as a separately scoped fix worktree.

## Dependencies and parallelism

Requires O.4 merged and QA-approved, plus the accepted Phase O design and the
merged O.2/O.3 contracts. Corpus-root preparation may begin while O.4 is in
QA, but all scan and release claims must use the merged O.4 parent commit.
O.5 is the final Phase O sprint and cannot execute in parallel with another O
sprint that changes the render contract.

## Exact targets

- `.claude/skills/adversarial-fuzzing/SKILL.md`
- `docs/phase-O/release-corpus-roots.txt` (source-of-truth root/commit list)
- `docs/phase-O/evidence/o5-release-corpus.md`
- `site/reports/` (dated fuzz and release evidence)
- `docs/requirements.md`
- `docs/migration-notes.md`
- `CHANGELOG.md`

The scan may read external repositories, including `../atm-core`, but this
sprint does not edit them. Any external fix is reported with repository,
commit, path, and owner for separate worktree/PR handling.

## Corpus source, owner, and process

`docs/phase-O/release-corpus-roots.txt` is the authoritative inventory source
for this sprint. It is owned by `team-lead@sc-compose`, reviewed by
quality-mgr, and contains one repository root and pinned commit per line, plus
an explanation for exclusions. It must enumerate actual roots; the campaign
must report the actual count rather than repeat an unverified “20–30” estimate.
The initial required roots are:

```text
<this sc-compose worktree> <merged O.4 commit>
../atm-core <pinned commit supplied by team-lead>
```

Team-lead may add additional high-traffic repositories before the scan, but
each added root must be pinned and named in the file. For every root, O.5
must:

1. verify the root and commit with `git -C ROOT rev-parse COMMIT`;
2. enumerate `.json.j2`, `.json.jinja`, and equivalent JSON template paths
   with `rg --files ROOT`;
3. search for literal-quoted placeholders and bare placeholders separately;
4. record repository, commit, path, mode/annotation, finding code, and
   migration owner in `docs/phase-O/evidence/o5-release-corpus.md`;
5. preserve a minimal reproducer and exact context for every promoted finding.

No repository is declared clean solely because its files are not available;
unavailable roots are explicit campaign errors and block the release gate.

## Required work

1. Update the adversarial-fuzzing skill so every successful JSON render is
   parsed as a complete JSON document before it is marked PASS. The result
   must record binary/version, commit, repository root, exact template,
   exact context, effective mode, parser result, and diagnostic code.
2. Add campaign probes for secure-auto, legacy-compatibility, mode mismatch,
   template-init round trip, output-parser failures, nested/conditional
   templates, hostile strings, and the six-template O.4 corpus.
3. Include the original 1.4.0 regression as a permanent red-to-green fixture:
   the quoted `"{{ value }}"` shape must fail in auto mode before emission,
   while legacy mode must produce one safe JSON string with a deprecation
   diagnostic.
4. Run the pinned cross-repository corpus inventory and attach the complete
   path-level result to the dated report under `site/reports/`.
5. Re-run the full campaign against the merged O.4 behavior and the 1.4.1
   release candidate; do not use a report generated from an earlier parent.
6. Record rollout readiness, unresolved external findings, compatibility
   deprecation timing, and the decision criteria for future removal of explicit
   `legacy` mode.

## Required tests and evidence

- every successful JSON case has parser-backed PASS evidence;
- malformed output has non-zero/fail-closed evidence and no emitted body;
- both source forms are tested with quotes, backslashes, control characters,
  Unicode, arrays, objects, nulls, loops, includes, and conditionals;
- the original 1.4 regression is reproduced by the campaign and caught;
- every corpus root is pinned, scanned, counted, and reported;
- every promoted issue includes minimal template, exact input, expected oracle,
  observed result, reproduction count, and requirement/ADR trace;
- `site/reports/` contains the dated multi-panel report and machine-readable
  result used to make the release decision.

## Deliverables

- updated adversarial-fuzzing skill and parser-backed oracle;
- pinned root inventory and path-level migration report;
- dated release-candidate fuzz report;
- external-repository findings with owners and handoff evidence;
- 1.4.1 release-readiness recommendation and deprecation decision criteria.

## Acceptance criteria

- [x] The corpus source-of-truth file contains the actual pinned repository
      roots and the evidence reports the actual scanned count.
- [x] All available roots scan clean or have an owned, actionable finding;
      unavailable roots block rather than pass.
- [x] The campaign fails on the original 1.4.0 double-quote regression and
      passes both supported modes only under their correct contracts.
- [x] Every successful JSON render is parser-checked before PASS.
- [x] No release candidate emits malformed JSON with success status.
- [x] O.4's six-template fixtures and O.2/O.3 diagnostic contracts are used
      without a duplicate parser or scanner.
- [x] Reports are materialized under the required dated `site/reports/` path.
- [x] Changelog, migration, ATM-core handoff, and external issue ownership
      are complete.
- [x] Workspace gates, the authoritative provisioned lint profile, and
      release-candidate evidence pass; bare local `just lint` remains blocked
      only when its CI-provided sibling binaries are absent.

## Sc-lint cleanup and QA handoff

Run the applicable sc-lint/template-contract profile against the final O.5
commit and the release evidence. Fix minor findings locally. Remaining
findings require `fix/` worktrees created from O.5's final commit, grouped by
independent rule class and owner; do not create one worktree per warning or
mix evidence changes with semantic renderer fixes. Send team-lead the parent
commit, fix path, finding class, evidence, tests, and fix commit. Team-lead
creates PRs and routes them to quality-mgr. O.5 cannot close until QA approves,
required fixes merge, and the corpus/fuzz campaign is rerun on the merged
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
