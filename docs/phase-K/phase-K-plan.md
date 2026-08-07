---
id: phase-K
title: Repowise Hot-Spot Maintainability Cleanup
status: planned
branch: integrate/phase-k
worktree: ../sc-compose-worktrees/integrate/phase-k
target: develop
baseline: 5fc2f38
---

# Phase K — Repowise Hot-Spot Maintainability Cleanup

## Objective

Reduce the maintainability risk identified by GitHub issue [#311](https://github.com/randlee/sc-compose/issues/311). The issue's 2026-08-07 Repowise scan analyzed 537 files, scored overall health 8.46/10 and hotspot health 5.75/10, and identified eight concrete files with the lowest hotspot scores. Phase K performs behavior-preserving decomposition only; it does not change rendering, extraction, diagnostics, CLI, Python bindings, error semantics, or report schemas.

## Plan authority and review contract

This phase plan is the scope authority for phase-wide boundaries, sprint
numbering, dependencies, and the explicit follow-on exclusions below. Each
sprint document is independently authoritative for its own exact targets,
deliverables, acceptance criteria, and required validation; a downstream
dispatch or QA prompt must not narrow or replace those sections. All eight
sprints are implementation sprints and therefore must land executable,
behavior-preserving code plus characterization coverage. None is a planning-
only sprint and none has a deletion deliverable. K.7 may be stopped before
merge if its characterization proves that no safe seam exists, but that is a
failed/non-closed sprint outcome, not a test-only completion or a Phase K
exit contribution.

The reviewed implementation baseline is `develop` at `5fc2f38` (the branch
also contains this plan package at the plan-hardening commit). Before any
sprint changes code, it must characterize the named baseline behavior, record
the focused test result, and rerun the same test after the move. A refactor
that cannot preserve the stated seam is explicitly allowed to close with the
code move abandoned, but it must not claim decomposition completion.

## Evidence and scope ledger

| Sprint | Exact target | Issue score | Reported signal | Closure boundary |
|---|---|---:|---|---|
| K.1 | `crates/sc-composer/src/extract/xml.rs` | 2.35 | CCN 13, 672 NLOC | parser/model, evidence collection, and orchestration have private seams; XML extraction behavior is unchanged |
| K.2 | `crates/sc-compose/src/commands/compose.rs` | 3.09 | 593 NLOC | request/preflight, render execution, and output emission have private seams; CLI behavior is unchanged |
| K.3 | `crates/sc-compose/src/var_file.rs` | 3.91 | CCN 17, 722 NLOC | JSON/YAML decoding, lexical safety scans, and object validation are isolated; var-file results and errors are unchanged |
| K.4 | `crates/sc-composer/src/diagnostics.rs` | 3.90 | 38% duplication | code schema, filesystem classification, diagnostic model, and envelope are isolated; serialized codes/schema remain unchanged |
| K.5 | `crates/sc-composer/src/error.rs` | 3.99 | 55% duplication | error-family definitions share private display/source helpers; constructors, accessors, codes, and text remain unchanged |
| K.6 | `crates/sc-composer/src/include.rs` | 4.05 | 54% duplication, CCN 10 | expansion state, path resolution, and directive scanning are isolated; include graph, confinement, and diagnostics remain unchanged |
| K.7 | `crates/sc-composer/src/discovery.rs` | 4.06 | CCN 9, 484 NLOC | delimiter walking, scope parsing, and identifier masking are isolated; discovered-token and loop-scope sets remain unchanged |
| K.8 | `crates/sc-compose/src/reporting/output.rs` | 4.21 | 28% duplication | report path/layout, materialization, and archive handling are isolated; artifact paths and metadata remain unchanged |

The scan's `catalog.rs` and `resolver.rs` entries remain follow-on candidates. They are not included in this phase because K.8 owns the reporting output boundary and K.6 owns include path handling; adding both adjacent files now would enlarge fan-out without a separately characterized seam. `discovery.rs` is included despite recent stabilization because K.7 is explicitly characterization-first and may be stopped without moving code if the seam is not safe.

## Sequence and dependencies

K.1, K.2, K.3, K.5, K.6, K.7, and K.8 are independent implementation sprints from `integrate/phase-k`; each has one file/module owner and may be reviewed independently. K.4 should land before K.5 and K.6 when practical because both consume diagnostic/error contracts, but it is not a source-level prerequisite if characterization tests prove the existing exports remain stable. No sprint may begin implementation until its own plan-gate review passes.

Recommended merge order is K.4 → K.5 → K.6, with K.1/K.2/K.3/K.7/K.8 parallelized around that chain. The integration branch is the only merge-forward target; sprint branches must not repeatedly merge sibling branches into one another.

## Hard boundaries

- Every sprint is structural and behavior-preserving. No public Rust/Python API, CLI flag, exit code, diagnostic code/severity/order/location, error message, extraction report, include policy, or report artifact schema may change.
- Existing public paths remain available through re-exports. New modules are private implementation seams unless an existing item was already public.
- Each sprint must add or strengthen characterization tests before moving code and retain those tests after the move.
- No speculative abstraction, generic framework, algorithm rewrite, performance claim, or cross-hotspot cleanup is in scope.
- A refactor may be stopped after characterization if the proposed seam does
  not reduce ownership without increasing risk; the sprint then reports the
  evidence, remains non-closed (or is explicitly re-planned), and leaves the
  original module intact. It may not claim completion on characterization
  tests alone.

## Phase-wide validation contract

Every implementation sprint's Required validation section is the
authoritative command list for that sprint and must include the following
common gates verbatim: `cargo fmt --all --check`, `git diff --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo test --workspace`. It must also name a focused command or test target
for the sprint's seam, and require that command to pass both before and after
the move. Sprints touching `sc-composer` public types or the Python-facing
crate additionally run the applicable Python smoke suite. Each sprint records
before/after production-NLOC and module ownership evidence. Repowise
rescanning is a post-integration diagnostic, not a standalone closure gate.

No sprint may close on a shape-only split, test-only addition, or an
unmeasured line-count reduction. The focused characterization result, public
surface/diff review, and full workspace gates are required evidence.

## Exit gate

Phase K closes only when all eight sprint documents are QA-approved, their characterization tests remain green, the full integration suite passes, no hard-boundary file changed outside declared scope, and a fresh Repowise scan is recorded for follow-up—not used to reject closure solely because scan timing or scoring varies.

## Sprint documents

1. [K.1 — XML extraction decomposition](sprint-k-1-xml-extraction-decomposition.md)
2. [K.2 — Compose command orchestration](sprint-k-2-compose-command-orchestration.md)
3. [K.3 — Var-file decoding and validation](sprint-k-3-var-file-decoding.md)
4. [K.4 — Diagnostic schema and envelope](sprint-k-4-diagnostics-schema.md)
5. [K.5 — Error-family modules](sprint-k-5-error-family-modules.md)
6. [K.6 — Include expansion seams](sprint-k-6-include-expansion.md)
7. [K.7 — Template discovery seams](sprint-k-7-discovery-seams.md)
8. [K.8 — Report output materialization](sprint-k-8-report-output.md)

## References

- GitHub issue #311: Repowise Hot Spot Analysis — sc/repowise-update2
- `docs/phase-J/phase-J-plan.md` for the prior decomposition precedent
- `.claude/skills/plan-hardening/sprint-planning-guidelines.md`
- `docs/git-workflows.md`
- `docs/requirements.md` and `docs/architecture.md` for the unchanged
  library/CLI boundary and observable contracts
