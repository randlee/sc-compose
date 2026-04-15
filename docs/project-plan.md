# SC-Compose Project Plan

## Status

This repo is in release execution.

The goal is production-ready release of:

- `sc-composer`
- `sc-compose`

This document is the authoritative release plan. It replaces the earlier
implementation-history view with one sprint sequence that contains all work
required to ship.

## Release Rules

- `requirements.md`, `architecture.md`, and this plan are the release source of
  truth.
- No sprint may leave a known release blocker unassigned.
- A later sprint may start only after the prior sprint exit gate passes.
- Deferred work is allowed only when it is explicitly out of scope for the
  initial release and does not reduce production readiness.
- `sc-composer` remains a pure library.
- `sc-compose` may depend on `sc-composer` and standalone observability crates
  only.
- No ATM-specific runtime assumptions may enter code or manifests.

## Release Blocker Inventory

Current known release blockers:

| ID | Blocker | Status | Sprint | Closure condition |
| --- | --- | --- | --- | --- |
| RB-01 | Final release command surface and JSON contracts are not yet locked as an implementation baseline. | Closed | Sprint 1 | `requirements.md`, `architecture.md`, and `project-plan.md` define one consistent command and schema contract. |
| RB-02 | The local observer contract and event conventions are not yet fully implemented in `sc-composer`. | Closed | Sprint 2 | Observer API, event fields, and no-op behavior are fixed in docs and then implemented in code. |
| RB-03 | `sc-compose` does not yet wire the concrete `sc-observability::Logger` integration path. | Closed | Sprint 2 | CLI startup constructs the logger, adapts it into the observer path, and exposes `observability-health`. |
| RB-03a | `CliObserver` still uses a bespoke sink path instead of `sc-observability::Logger`. | Closed | Sprint 2 | Replace the bespoke observer sink with `sc-observability::Logger` construction and adapter wiring. |
| RB-03b | `--json` console sink suppression is not yet wired through the observer/logger path. | Closed | Sprint 2 | Console sink suppression is enforced through the `sc-observability::Logger` construction path whenever `--json` is active. |
| RB-03c | Graceful logger shutdown is not yet called before `process::exit()`. | Closed | Sprint 2 | The CLI calls `logger.shutdown()` before process exit so pending events flush cleanly. |
| RB-04 | Production logging safeguards are not yet proven. | Closed | Sprint 3 | Tests prove `--json` cleanliness, shutdown/flush behavior, sink degradation behavior, and event coverage. |
| RB-05 | Any non-observability release blocker found during audit must be closed before release. | Closed | Sprint 3 | Every audit finding is either closed or explicitly moved to a later sprint in this plan before Sprint 1 exit. |
| RB-06 | Final release validation, QA approval, and cutover readiness are not yet complete. | Open | Sprint 4 | End-to-end smoke tests, QA review, design review, and release approval all pass. |

Inventory rules:

- Sprint 1 owns this inventory.
- Any blocker discovered during Sprint 1 must be added to this table with an
  assigned sprint before Sprint 1 can exit.
- No blocker may be removed from this table until its closure condition is met.

## Release Plan

### Sprint 1: Release Blocker Audit and Contract Closure

Branch:

- `feature/release-contract-closure` -> `develop`

FRs addressed:

- FR-7
- FR-8a
- FR-9
- FR-10
- FR-11

Deliverables:

- update the `Release Blocker Inventory` section in this document so it lists
  every known release blocker and its assigned sprint
- final normative updates in:
  - `docs/requirements.md`
  - `docs/architecture.md`
  - `docs/project-plan.md`
- final command surface for the initial release, including:
  - `render`
  - `resolve`
  - `validate`
  - `frontmatter-init`
  - `init`
  - `observability-health`
- final logging-only observability contract, including:
  - `sc-composer` local observer hook model
  - CLI-owned command lifecycle events
  - pipeline event emission points
  - stable event `message` conventions
  - no-op fallback behavior
  - `--json` console suppression
  - `observability-health` command behavior
  - graceful shutdown behavior
- explicit initial-release scope statement that keeps:
  - structured logging and health reporting in scope
  - `sc-observe` and `sc-observability-otlp` out of scope

Acceptance criteria:

- the `Release Blocker Inventory` section lists every known release blocker and
  assigns each one to a sprint in this plan
- `requirements.md`, `architecture.md`, and `project-plan.md` are mutually
  consistent
- the logging contract is precise enough to implement without inventing new
  behavior during coding
- the initial-release command surface is final
- `observability-health` is fully specified as a release command rather than a
  placeholder
- no unresolved contradiction remains around the local observer model, logger
  wiring, command lifecycle events, event `message` conventions, or no-op
  behavior

Exit gate:

- `qm-comp` cross-document consistency review passes
- req-qa and arch-qa find no blocking document mismatch
- the `Release Blocker Inventory` section is accepted as complete

### Sprint 2: Logging Integration Implementation

Branch:

- `feature/release-logging-integration` -> `develop`

FRs addressed:

- FR-9
- FR-10
- FR-11

Deliverables:

- `crates/sc-composer/src/observer.rs` implementing the local observer
  contract, including:
  - `ObservationEvent`
  - `ObservationSink`
  - `CompositionObserver`
  - built-in no-op observer path
- `compose_with_observer(...)` as the end-to-end library injection entry point
- `compose()` and `Renderer` default behavior that remains functional when no
  observer is injected
- `sc-observability` dependency and logger construction in `sc-compose`
- CLI-owned adapter from the local `sc-composer` observer model to
  `sc-observability::Logger`
- command lifecycle logging for:
  - command start
  - command completion
  - command failure
- pipeline-stage logging for:
  - resolve
  - include-expand
  - validate
  - render
- `observability-health` command implementation
- console sink suppression in `--json` mode
- logger shutdown wiring on process exit

Acceptance criteria:

- `sc-composer` does not depend on `sc-observability-types`
- `sc-composer` does not depend on `sc-observability`
- `sc-compose` constructs `Logger` and adapts it into the library observer
  path
- command lifecycle events and composition-stage events are emitted through the
  documented mapping
- `observability-health` returns the documented `LoggingHealthReport`
- `--json` mode remains machine-readable
- shutdown flushes sinks on exit and does not break command behavior

Exit gate:

- `cargo test --workspace` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- `qm-comp` implementation review finds no blocking contract mismatch

### Sprint 3: Production Hardening and Gap Closure

Branch:

- `feature/release-production-hardening` -> `develop`

FRs addressed:

- FR-1 through FR-11 where production behavior requires hardening

Deliverables:

- focused tests for:
  - observer injection and no-op defaults
  - command lifecycle logging
  - resolve/include-expand/validate/render event coverage
  - event `message` guidance and stable target/action naming
  - `observability-health` text output
  - `observability-health --json`
  - `observability-health` process-local behavior without daemon dependency
  - `--json` console suppression and stdout cleanliness
  - graceful shutdown and flush behavior
  - sink failure degradation behavior
- failure-path coverage for logging integration
- closure of every non-observability release blocker identified in Sprint 1
- updates to release notes, migration notes, and cutover notes where changed
  behavior affects downstream consumers

Acceptance criteria:

- no release blocker from Sprint 1 remains open
- logging support is production-ready for:
  - CLI use
  - consuming applications that extend logging through the documented observer
    hook model
- all documented logging behaviors are covered by automated tests
- no command emits console log noise that corrupts machine-readable stdout
- health reporting and shutdown behavior are proven by tests rather than by
  documentation alone

Exit gate:

- `cargo test --workspace` passes with the full logging and hardening suites
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- no Priority 1 or Priority 2 QA finding remains open

### Sprint 4: Release Readiness and Cutover

Branch:

- `feature/release-gate` -> `develop`

FRs addressed:

- FR-1 through FR-11 release validation

Deliverables:

- final release-readiness checklist for both crates
- final migration and cutover notes for downstream consumers
- final verification of standalone boundary rules
- final end-to-end smoke tests
- final QA and design review pass
- branch prepared for merge to `develop`, then release merge to `main`

Acceptance criteria:

- all FR-1 through FR-11 behavior is implemented and covered by automated tests
- all release blockers are closed
- all required docs match shipped behavior
- downstream cutover notes are accurate
- release workflow prerequisites are satisfied

Exit gate:

- `cargo test --workspace` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- full end-to-end smoke test passes using includes, vars, frontmatter, and
  observability-health
- `qm-comp` full QA pass
- `arch-ctm` final design review pass
- branch approved for merge to `develop`
- release approved for merge to `main`

## FR Coverage Matrix

- FR-1 through FR-6:
  - already specified in the normative docs
  - revalidated in Sprint 3 and Sprint 4 where release blockers or integration
    changes touch them
- FR-7:
  - Sprint 1 finalizes the command surface
  - Sprint 2 implements `observability-health`
  - Sprint 3 hardens command behavior
  - Sprint 4 validates release behavior
- FR-8 and FR-8a:
  - Sprint 1 finalizes command and health schemas
  - Sprint 2 implements the logger-facing command output
  - Sprint 3 hardens JSON and failure-path behavior
  - Sprint 4 validates release behavior
- FR-9:
  - Sprint 1 finalizes the logging-only integration contract
  - Sprint 2 implements the logging path
  - Sprint 3 hardens and validates it
  - Sprint 4 validates release behavior
- FR-10:
  - Sprint 1 finalizes the local observer contract
  - Sprint 2 implements it
  - Sprint 3 hardens and validates it
  - Sprint 4 validates release behavior
- FR-11:
  - Sprint 1 finalizes CLI logger behavior
  - Sprint 2 implements it
  - Sprint 3 hardens and validates it
  - Sprint 4 validates release behavior

## Production Readiness Gate

Release is complete only when all four sprints have passed and all of the
following are true:

- no release blocker remains open
- `requirements.md`, `architecture.md`, and `project-plan.md` match the shipped
  behavior
- all FR-1 through FR-11 behavior is implemented and covered by automated tests
- `cargo test --workspace` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- full end-to-end smoke coverage passes
- `qm-comp` completes a full QA pass
- `arch-ctm` completes a final design review
- release is approved for merge to `main`

## Companion Planning Docs

- `docs/traceability-matrix.md`
- `docs/error-code-registry.md`
- `docs/test-strategy.md`

## Rule

Any follow-on sprint added after this plan must preserve the standalone
boundary defined by:

- `docs/requirements.md`
- `docs/architecture.md`
- `docs/git-workflows.md`
- `docs/publishing.md`
