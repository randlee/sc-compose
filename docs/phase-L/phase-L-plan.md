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
