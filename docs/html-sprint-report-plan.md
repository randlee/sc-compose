# SC-Reporting Follow-On Plan

## Status

Planning line only. H1-H4 are shipped. This document covers the reusable
reporting line that follows the shipped single-panel HTML example and does not
change the delivered `1.0` contract until a later review accepts it.

## Goal

Lay down one reusable reporting pattern for `sc-compose` consumers so repos can
add lint, test, smoke, diagram, and custom publishable reports without
reinventing report layout, output policy, or handoff conventions.

The follow-on line must support:

- report generation through the repo's domain `just` recipes such as
  `just lint`, `just test`, `just smoke`, and repo-specific custom recipes,
- a shared evidence contract for generated artifacts and metadata,
- one stable latest output plus timestamped archive copies where the producer
  recipe enables them,
- one shared `just reports` surface for aggregation, verification, and
  opening/viewing,
- reusable templates and panel chrome where they add value,
- reusable diagram/state-machine and SQL-query reporting patterns across many
  repos,
- future renderer changes without keeping Mermaid as the long-term semantic
  source of truth.

## Shipped Baseline

Phase HTML-Report already delivered:

- H1 object/map inputs,
- H2 arrays of objects,
- H3 the bundled single-panel `sprint-report-html` example,
- H4 wrapper-owned HTML rendering integration without hook execution in
  `sc-compose`.

## Follow-On Rules

- Producer recipes own report generation. `just lint`, `just test`,
  `just smoke`, and repo-specific producer recipes generate their own evidence.
- `just reports` is an aggregator and verifier, not the primary producer.
- Authored docs and generated evidence stay separate:
  - `docs/` for authored policy and design notes
  - report specs/templates/catalogs under a report-specific tree
  - generated latest/archive outputs under generated-evidence paths
- The report contract must allow repo-specific custom reports without changing
  the shared aggregation pattern.
- GitHub issue `#56` is in-scope for the follow-on line as the generic
  source-collection and render-many capability, but Mermaid-as-SSOT is treated
  as transitional rather than the long-term semantic end state.
- Network publish behavior and browser-open behavior remain outside
  `sc-composer` and `sc-compose`.

## Phase A Sprint Sequence

The authoritative sprint order for this line is the Phase A plan in
[docs/phase-A/phase-A-plan.md](phase-A/phase-A-plan.md):

1. `A1` report artifact contract and catalog
2. `A2` producer-recipe and `just` command contract
3. `A3` source-collection, metadata-extraction, and render-many contract
4. `A4` semantic diagram-spec contract
5. `A5` template-family and shared panel-chrome contract
6. `A6` latest/archive output policy and `just reports` aggregator contract
7. `A7` publish-manifest and CI handoff contract
8. `A8` cross-use-case proof through multiple report families
9. `A9` `sc-observability` `1.1.0` adoption for report-producing CLI flows

## Output Direction

The follow-on line should converge on a shared evidence shape with:

- a report catalog/manifest
- source specs and templates separated from generated outputs
- one latest artifact location per report
- optional timestamped archive outputs
- one machine-readable sidecar per generated report
- one machine-readable handoff for downstream publication tooling

## Example Consumer Shapes

The shared reporting line must be broad enough to cover at least:

- `atm-core` style repeated state-machine and SQL-query diagrams
- `sc-lint` style lint/test/smoke and other evidence reports
- repo-specific custom evidence producers added without changing the shared
  report contract

## Explicit Non-Goals

- browser-opening logic inside `sc-compose`
- hook execution inside `sc-composer`
- network upload or hosting behavior inside `sc-compose`
- locking the long-term diagram source model to Mermaid text
