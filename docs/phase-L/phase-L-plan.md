---
phase: L
title: sc-lint Integration
status: planned
target: integrate/phase-l
---

# Phase L — sc-lint Integration

## Baseline and evidence

The integration target is the installed sc-lint 0.4.0 CLI. Its top-level
command surface is:

- lint sc-boundary, sc-portability, sc-runtime, line-counts, identity-literals,
  fast, full, and ci
- view findings and the reserved view graph
- check native and check xwin
- clippy native and clippy xwin
- top-level ci and version

The source contract in sc-lint/crates/sc-lint/src/cli.rs, command.rs,
workflow.rs, dispatch.rs, and python_adapter.rs is the authority for these
names and their JSON envelopes. sc-lint --json is the machine interface;
sc-compose must not scrape human output or reimplement analyzer rules.

The current sc-compose Justfile only renders the existing sc-lint-named HTML
evidence family. It does not invoke the sc-lint executable. A direct probe of
every 0.4.0 target against sc-compose failed before analysis because
sc-compose has no boundaries/ sentinel. This is an integration prerequisite,
not evidence that an analyzer is incompatible.

## Phase goal

Make sc-compose a standard consumer of sc-lint 0.4.0 with a shared,
machine-readable invocation/reporting path, then integrate each supported
command or target in its own independently reviewable sprint.

## Ordering and parallelism

L.1 and L.2 are the only infrastructure sprints and must land in order. L.1
establishes repository/config/tool prerequisites. L.2 establishes the generic
sc-compose command, report materialization, and standardized just recipes.

After L.2 is merged to integrate/phase-l, L.3 through L.16 are independent.
Each owns one sc-lint command target or profile, has a disjoint target
descriptor and focused test fixture, and may be staffed or merged in parallel.
No later sprint may edit the shared runner or duplicate a Python adapter.
L.17 runs only after L.1-L.16 are complete; it is a closeout/documentation
sprint that turns the script inventory and packaging recommendation into a
tracked issue on the sc-lint repository.

## Standard consumer command set

Every consuming repository must expose the same recipes:

- just lint — default sc-lint lint full
- just lint fast|full|ci
- just lint sc-boundary|sc-portability|sc-runtime|line-counts|identity-literals
- just view findings
- just check native|xwin
- just clippy native|xwin
- just ci — top-level sc-lint ci, including workspace tests

The recipes invoke the shared sc-compose integration, which invokes
sc-lint --json. No repository-specific Python runner is permitted. JSON
stdout, exit status, diagnostics, and target identity remain available as
report artifacts; sc-compose owns only orchestration and HTML/XHTML
materialization.

## Sprint inventory

| Sprint | Scope | Parallel after L.2 |
| --- | --- | --- |
| L.1 | Repository/tool/config bootstrap and boundary inventory | No |
| L.2 | Shared sc-compose lint runner, reports, and just contract | No |
| L.3 | lint sc-boundary | Yes |
| L.4 | lint sc-portability | Yes |
| L.5 | lint sc-runtime | Yes |
| L.6 | lint line-counts | Yes |
| L.7 | lint identity-literals | Yes |
| L.8 | view findings | Yes |
| L.9 | check native | Yes |
| L.10 | check xwin | Yes |
| L.11 | clippy native | Yes |
| L.12 | clippy xwin | Yes |
| L.13 | lint fast | Yes |
| L.14 | lint full | Yes |
| L.15 | lint ci | Yes |
| L.16 | top-level ci | Yes |
| L.17 | sc-lint Python script inventory and packaging issue | No; after L.1-L.16 |

version is closed by L.1's version gate and is not duplicated as a feature
sprint. view graph remains explicitly deferred because sc-lint 0.4.0 marks it
as a reserved capability surface.

## Cross-sprint invariants

- No Cargo dependency on sc-lint is added to either sc-composer or sc-compose.
- The pure library boundary remains unchanged; external-tool orchestration is
  CLI, Justfile, and reporting work only.
- Every target uses the same sc-lint --json --root . invocation policy and
  preserves the top-level command envelope.
- Every feature sprint adds a positive and a negative or unsupported-path
  characterization case and verifies the target's report panel and raw JSON.
- Every implementation sprint runs cargo test --workspace, formatting, clippy
  with -D warnings, and git diff --check.

## Python script reuse and packaging constraint

The sc-lint sibling repository is the source of truth for Python-backed
utilities. Where a representative script exists, the target sprint must point
to that script and reuse it through the sc-lint command contract; it must not
copy the script into sc-compose or create a per-target wrapper. The current
0.4.0 inventory is:

| sc-compose target | Representative sc-lint source | Reuse decision |
| --- | --- | --- |
| lint sc-portability | `../sc-lint/.just/lint_sc_portability.py` | Reuse through `sc-lint`; retain its adapter tests |
| lint line-counts | `../sc-lint/.just/lint_line_counts.py` and `../sc-lint/.just/python_adapter.py` | Reuse through `sc-lint`; do not duplicate |
| lint identity-literals | `../sc-lint/.just/lint_identity_literals.py` and `../sc-lint/.just/python_adapter.py` | Reuse through `sc-lint`; do not duplicate |
| view findings | `../sc-lint/.just/view_findings.py`, `view_common.py`, and `python_adapter.py` | Reuse through `sc-lint`; do not duplicate |
| lint fast/full/ci | `../sc-lint/.just/run_lint.py` and `workflow.rs` | Reuse profile semantics; do not copy the runner |
| lint sc-boundary | `../sc-lint/.just/lint_sc_boundary.py` (legacy helper) plus Rust dispatch/backend sources | Do not invoke the legacy helper directly; reuse the supported Rust-backed CLI |
| lint sc-runtime, check, clippy | Rust dispatch/workflow/backend sources | No representative Python script exists in 0.4.0 |
| ci | Rust workflow composition | No separate Python script exists in 0.4.0 |

This inventory exposes a release concern: sc-lint 0.4.0's Python adapter
resolves these scripts relative to the analyzed repository's `.just/` path.
L.1 must record whether the supported distribution makes those utilities
available to a consumer without copying them. A related maturin/Python
bindings request is tracked in sc-lint issue #83. L.17 must create the final
sc-lint inventory issue, link #83, include concrete failing-path evidence, and
recommend a pip-installable package/embedded-resource or module entrypoint
that owns all commonly used scripts and preserves the JSON/schema contract.
Until that issue is resolved, sc-compose may invoke only the supported
`sc-lint --json` contract and may not vendor the scripts.

## Mandatory sc-lint cleanup and QA routing

Every L.1-L.16 sprint must run its applicable sc-lint targets against the
final sprint commit before handoff. Classify findings before changing them:

1. Fix minor findings immediately in the current sprint worktree and include
   their tests/validation in the sprint handoff.
2. For every remaining finding, create a dedicated `fix/` branch and worktree
   (for example branch `fix/l-7-identity-sc-compose` at
   `../sc-compose-worktrees/fix/l-7-identity-sc-compose`) from that sprint
   worktree's final commit. Keep each fix worktree to one independent class of
   change and one coherent ownership boundary.
3. Group mechanical constant-string/identity findings by owning crate rather
   than by individual finding; normally use one worktree per crate and split
   into at most three only when ownership or conflict risk requires it. Never
   create one worktree per string finding.
4. Keep length-driven refactors separate, normally one worktree per violating
   file/refactor. Do not mix them with constant-string, boundary, portability,
   runtime, or clippy changes.
5. Group other findings only when they share the same rule class, owner, and
   mechanical change. Distinct semantic refactors require distinct worktrees.
6. Send each fix worktree path, branch, parent sprint commit, finding class,
   evidence, tests, and fix commit to team-lead. The developer does not create
   the PR; team-lead creates the PR and sends it to quality-mgr for independent
   QA approval.
7. The parent sprint cannot be marked complete until every required fix PR is
   QA-approved, merged, and revalidated. Profile sprints route findings back
   to the originating target sprint and must not create duplicate fixes.

Each sprint document contains its expected finding classes and repeats this
routing contract as a sprint-local handoff gate.
