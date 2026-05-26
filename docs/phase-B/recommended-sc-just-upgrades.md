# Recommended `sc-just` Upgrades For `sc-compose`

## Purpose

This document records the recommended upgrades to the canonical `sc-just`
package at:

- `synaptic-canvas/packages/sc-just`

The goal is to keep `sc-just` as the shared `just` skill/package while making
it a better consumer of the reporting/runtime commands implemented by
`sc-compose`.

## Canonical Split

### `sc-compose` owns

- report catalog loading and validation
- source collection and `render-many`
- shared template-family resolution
- latest/archive output handling
- report verification and index/summary commands
- publish-manifest generation
- typed diagram-spec support

### `sc-just` owns

- recommended `Justfile` recipe surface
- recipe scaffolding and bootstrap flow
- repo-facing command layout
- standard TODO markers showing what the consumer repo must fill in

### consumer repos own

- repo-specific `just lint`, `just test`, and `just smoke` bodies
- repo-specific templates and source inputs
- repo-specific wrapper/publish commands

## Required Design Rule

`sc-just` should use `sc-compose` for shared reporting behavior rather than
reimplementing reporting logic in shell recipes.

That means:

- `sc-just` calls `sc-compose` commands for report catalog/runtime behavior
- `sc-just` scaffolds `Justfile` entries that point at `sc-compose`
- consumer repos fill in producer-specific command bodies where needed

## Recommended Recipe Surface

The recommended `sc-just` surface should include:

```make
lint:
	@echo "repo-owned lint producer"

test:
	@echo "repo-owned test producer"

smoke:
	@echo "repo-owned smoke producer"

reports:
	sc-compose reports index --catalog reports/catalog/reports.toml

reports-verify:
	sc-compose reports verify --catalog reports/catalog/reports.toml
```

Optional repo-local helpers may exist, but they are not part of the shared
contract:

- `just reports-open`
- `just reports-clean`
- `just reports-publish`

## Recommended Upgrades

### 1. Report Catalog Bootstrap

`sc-just` should scaffold:

- `reports/catalog/reports.toml`
- `reports/latest/`
- `reports/archive/`
- `reports/templates/`

The scaffold should assume `sc-compose` owns catalog parsing and validation.

### 2. Smoke Scaffold Fixtures

`sc-just` should generate a standard smoke scaffold shape:

- one reference smoke template fixture
- one `sample-vars.json` fixture
- one wrapper entrypoint that the generated `just smoke` target invokes

This keeps the smoke contract stable across repos while still leaving the
actual repo-specific smoke command body consumer-owned.

### 3. Ownership Markers In Scaffold Output

Generated files should mark ownership explicitly:

- `sc-compose`-owned contract calls
- `sc-just`-owned scaffold structure
- consumer-owned command bodies and repo-local templates

This should be visible in generated comments or TODO markers.

### 4. `reports` And `reports-verify` Stubs

`sc-just` should scaffold the shared reporting commands as direct
`sc-compose` invocations:

- `sc-compose reports index --catalog reports/catalog/reports.toml`
- `sc-compose reports verify --catalog reports/catalog/reports.toml`

The skill should not shell-duplicate catalog logic or verification semantics.

### 5. Backward-Compatibility Harness Hooks

`sc-just` should support a standard place for backward-compat checks that run
through `sc-compose`, especially for:

- bundled example regressions
- migrated report-family regressions
- stable output-path expectations

### 6. Edge-Case Fixture Generation

`sc-just` should be able to scaffold fixtures that stress the reporting
pipeline:

- nested includes
- missing optional fields
- extra unknown fields
- large variable payloads
- unusual whitespace

These belong in the scaffold because they are shared test-shape concerns, not
repo-specific business logic.

### 7. Reference Repo Pattern

`sc-compose` should act as the reference repo showing how `sc-just` uses
`sc-compose` commands, but `sc-compose` should not become a second source of
truth for the skill itself.

The canonical package remains:

- `synaptic-canvas/packages/sc-just`

## Phase B Implications

This doc affects the interpretation of:

- `B2`
  - `sc-compose` provides the contract and scaffold seam that `sc-just`
    should call into
- `B5`
  - the real `reports` / `reports-verify` behavior is implemented in
    `sc-compose`
- `B8`
  - the end-to-end proof should exercise the recommended `sc-just` pattern,
    not a separate ad hoc `Justfile` model

## Non-Goal

This document does not move the canonical `just` skill/package into this repo.
It only records how the canonical `sc-just` package should use `sc-compose`.
