# SC-Reporting Phase A Plan

## Status

Planning stub only. This phase is follow-on work after the shipped `1.0`
baseline and does not change the current release contract until a later review
accepts it.

## Objective

Phase A defines a reusable reporting line for `sc-compose` so repos can add
lint, test, smoke, diagram, and custom publishable reports without
reinventing generation flow, output layout, or publication handoff.

The phase exists because multiple consumer shapes already need the same
underlying reporting patterns:

- `atm-core` style state-machine and SQL-query diagram reports with repeated
  panels, JSON sidecars, and generated report pages
- `sc-lint` style lint/test/smoke and other evidence reports
- future repo-specific custom producers that must plug into the same shared
  contract without changing the aggregator behavior

It also queues one explicit `sc-observability` `1.1.0` adoption closure so the
CLI logging layer used by report-producing workflows does not drift behind the
shared runtime.

The proof standard for this phase is not "one nice HTML report." It is
"multiple clearly different report families can plug into one shared artifact,
catalog, verification, and publication-handoff contract without inventing a
new reporting model per repo."

## Design Direction

- keep `sc-composer` runtime-agnostic
- keep network publish and browser-open behavior outside the core engine
- keep report generation owned by producer recipes such as `just lint`,
  `just test`, `just smoke`, and repo-specific custom producers
- reserve `just reports` for aggregation, verification, and viewing/opening
- separate authored docs from generated report evidence
- prefer a report catalog plus machine-readable metadata over hard-coded file
  paths
- treat GitHub issue `#56` source-collection/render-many work as foundational
  for source-driven report families
- prefer typed semantic specs for diagrams where possible, with Mermaid
  retained only as a transitional output or migration input
- centralize template families and shared panel chrome instead of
  reimplementing them in each consumer repo

## Sprint Sequence

1. `A1`
   - report artifact contract and catalog
2. `A2`
   - producer-recipe and `just` command contract
3. `A3`
   - source-collection, metadata-extraction, and render-many contract
4. `A4`
   - semantic diagram-spec contract
5. `A5`
   - template-family and shared panel-chrome contract
6. `A6`
   - latest/archive output policy and `just reports` aggregator contract
7. `A7`
   - publish-manifest and CI handoff contract
8. `A8`
   - proof-by-example through multiple report families
9. `A9`
   - `sc-observability` `1.1.0` adoption, retained-log policy decision, and
     deprecated `emit` migration

## Cross-Use-Case Proof Shape

Phase A treats the following families as the minimum proof that the shared
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
- adding a repo-specific custom producer must require only new catalog entries
  and templates/specs, not a new aggregator or verification model

## Exit Direction

Phase A should leave the repo with:

- one implementation-ready report artifact contract with:
  - report catalog/manifest
  - source specs/templates separated from generated outputs
  - latest plus optional archive output rules
  - machine-readable per-report metadata
- one explicit producer contract for:
  - standard producers such as lint/test/smoke
  - repo-specific custom producers that do not break shared report handling
- one source-collection and render-many contract suitable for generic
  source-driven report families
- one explicit template-family model for at least:
  - lint/test/smoke evidence reports
  - public API / CLI / ICD style reports
  - diagram/state-machine and SQL-query reports
- one shared panel shell contract with:
  - mandatory per-panel copy button
  - optional per-panel copy-to-JSON button
- one defined `just reports` contract for aggregation, verification, and
  opening/viewing
- one defined output policy for latest artifact overwrite and optional
  timestamped archive copy
- one defined machine-readable publish-manifest contract for CI or wrapper
  publishing
- multiple example families that prove the shared model is generic enough for
  both `atm-core` style diagrams and `sc-lint` style evidence reports
- one explicit plan for `sc-observability` `1.1.0` adoption in the CLI logging
  layer, including:
  - logger typestate compatibility
  - retained-log policy enable/defer decision
  - deprecated `emit` call-site migration to `log` / `try_log`
  - explicit `sc-observe` adoption decision
