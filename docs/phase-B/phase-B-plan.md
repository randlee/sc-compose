---
id: phase-B
title: SC-Reporting Phase B Plan
status: complete
branch: plan/phase-B
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/plan/phase-B
---

# SC-Reporting Phase B Plan

## Status

Phase B implementation is complete. The B1-B10 implementation line shipped on
the Phase-B integration branch; the remaining production-readiness gaps are
tracked separately in the follow-on cleanup slices below.

The note in `docs/phase-B/recommended-sc-just-upgrades.md` is a supporting
integration handoff for the external `sc-just` package. It is not a source of
truth for Phase B scope or sprint closure.

## Objective

Phase B implements the reusable reporting line defined by Phase A so repos can
add lint, test, smoke, diagram, and custom publishable reports without
reinventing generation flow, output layout, or publication handoff.

The phase exists because multiple consumer shapes already need the same
underlying reporting patterns, and Phase A defined the shared contract shape
without implementing the shared runtime for:

- report catalog loading and validation
- built-in render-context variable injection
- source-driven render-many generation
- shared template-family resolution and repo-local overrides
- latest plus archive output writing
- machine-readable publish-manifest handoff
- report-init scaffolding that makes consumer adoption easy

It also queues one explicit `sc-observability` `1.2` follow-up closure so the
CLI logging layer used by report-producing workflows can advance cleanly after
the `1.2` release exists.

The proof standard for this phase is not "the contract is documented." It is
"multiple clearly different report families can run through one shared
artifact, catalog, generation, verification, and publication-handoff runtime
without inventing a new reporting model per repo."

Phase A already shipped one observability implementation sprint:

- `A9`
  - `sc-observability` `1.1.0` adoption in `sc-compose`

Phase B focused on the reporting runtime and the tools that make it easy for
consumer repos to adopt.

## Design Direction

- keep `sc-composer` runtime-agnostic
- keep network publish and browser-open behavior outside the core engine
- keep report generation owned by producer recipes such as `just lint`,
  `just test`, `just smoke`, and repo-specific custom producers
- reserve `just reports` for aggregation, verification, and deterministic
  latest-entrypoint/index reporting
- treat wrapper-owned open helpers as optional add-ons rather than part of the
  shared command contract
- separate authored docs from generated report evidence
- prefer a report catalog plus machine-readable metadata over hard-coded file
  paths
- inject a small caller-overridable built-in render-context set so templates
  can depend on stable environment and template metadata without boilerplate
- treat GitHub issue `#56` source-collection/render-many work as foundational
- prefer typed semantic specs for diagrams where possible, with Mermaid
  retained only as a transitional output or migration input
- centralize template families and shared panel chrome instead of
  reimplementing them in each consumer repo
- make every sprint closure depend on runnable code and `cargo test --workspace`

## Sprint Sequence

1. `B1`
   - report artifact runtime and catalog
2. `B10`
   - built-in render context variables
   - follows `B1` because it changes the core render context
   - can run in parallel with `B3` through `B8` after the `B1` context shape
     is fixed
3. `B2`
   - producer recipes, report-init scaffold, and `just` command surface
4. `B3`
   - source collection, metadata extraction, and render-many runtime
5. `B4`
   - template families and shared panel chrome
6. `B5`
   - latest/archive output policy and reports aggregator
7. `B6`
   - publish manifest and CI handoff
8. `B7`
   - semantic diagram-spec runtime
9. `B8`
   - cross-use-case proof by implemented examples
10. `B9`
   - `sc-observability` `1.2` adoption after the upstream release exists

## Cross-Use-Case Proof Shape

Phase B treats the following families as the minimum proof that the shared
model is genuinely reusable rather than tuned to one repo:

### `atm-core` style diagram family

- producer commands such as `just state-diagrams` and `just sql-diagrams`
- repeated panels rendered from many semantic source inputs rather than one
  hand-written page
- panel-fragment or per-panel entrypoint support where consumers need drill-in
  pages
- mandatory text-copy output, optional JSON-copy output, and JSON sidecars for
  QA verification
- publish-manifest output suitable for later CI or wrapper publication

### `sc-lint` style evidence family

- producer commands such as `just lint`, `just test`, and `just smoke`
- one latest artifact set plus optional timestamped archive copies
- shared report pages that aggregate multiple evidence producers without
  requiring a special-case aggregator per repo
- JSON sidecars and publish-manifest output with the same catalog-driven
  discovery contract used by diagram families

### Shared Rule

- producer-command names and repo-local source inputs may vary by repo
- report discovery, verification, sidecar shape, latest/archive policy, and
  publish-manifest handoff stay shared across families
- `atm-core` and `sc-lint` are illustrative family labels only; the B4
  template-family key remains the catalog discriminator
- producer extension-point typing remains owned by the B1 report artifact
  runtime
- adding a repo-specific custom producer must require only new catalog entries
  and templates/specs, not a new aggregator or verification model

## Shipped Surface

Phase B left the repo with:

- one implemented report artifact contract with:
  - report catalog/manifest
  - source specs/templates separated from generated outputs
  - latest plus optional archive output behavior
  - machine-readable per-report metadata
- one implemented built-in render-context layer with:
  - caller-overridable template metadata variables
  - caller-overridable host/user/time variables
  - explicit precedence below caller inputs and above frontmatter defaults
- one implemented producer and scaffold contract for:
  - standard producers such as lint/test/smoke
  - repo-specific custom producers that do not break shared report handling
- one implemented source-collection and render-many runtime suitable for
  generic source-driven report families
- one implemented template-family model for at least:
  - lint/test/smoke evidence reports
  - public API / CLI / ICD style reports
  - diagram/state-machine and SQL-query reports
- one implemented shared panel shell contract with:
  - mandatory per-panel copy behavior
  - optional per-panel JSON copy behavior
- one implemented output policy for latest artifact overwrite and optional
  timestamped archive copy
- one implemented machine-readable publish-manifest output for CI or wrapper
  publication
- multiple example families that prove the shared model is generic enough for
  both `atm-core` style diagrams and `sc-lint` style evidence reports
- one explicit upstream-blocked follow-up sprint for `sc-observability` `1.2`

## Non-Goals

- browser-open automation inside `sc-compose`
- publish upload or network transport logic
- ATM-specific orchestration behavior inside either crate
- consumer-specific lint/smoke/test command bodies beyond scaffold TODO stubs

## Post-Close Cleanup Track

Phase B implementation is complete, but the accepted production-readiness
review left five follow-on cleanup slices that preserve the shipped scope while
closing remaining release-readiness gaps on `integrate/phase-B`.

- [Sprint B11 — Contract-Alignment](../sprints/b11-contract-alignment.md)
  - align `docs/requirements.md` and `docs/architecture.md` to the shipped
    `sc-composer` API surface
- [Sprint B12 — JSON Surface Hardening](../sprints/b12-json-surface-hardening.md)
  - normalize the remaining JSON and JSONL path surfaces and add
    Windows-sensitive coverage
- [Sprint B13 — Observability Panic Removal](../sprints/b13-observability-panic-removal.md)
  - remove production `expect()` / `unwrap()` paths from CLI observability code
  - boundary ADR: [ADR-0001: Observability Health Interface Stability During Panic Removal](../adrs/0001-observability-health-interface-stability.md)
- [Sprint B14 — CLI Extraction](../sprints/b14-cli-extraction.md)
  - split oversized CLI modules into focused command-owned files
- [Sprint B15 — Reporting Runtime Cleanup](../sprints/b15-reporting-runtime-cleanup.md)
  - remove dead reporting seams and over-scoped runtime helpers after the CLI
    extraction lands

These cleanup sprints do not reopen Phase B feature scope. They harden the
already-implemented Phase B line so it is easier to certify, review, and carry
forward.

The accepted cleanup findings and their owning sprint assignments are tracked
in [docs/issues-inventory.md](../issues-inventory.md).
