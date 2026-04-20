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
| RB-06 | Final release validation, QA approval, and cutover readiness were incomplete before Sprint 4 closeout. | Closed | Sprint 4 | End-to-end smoke tests, QA review, design review, and release approval all pass; closure evidence includes `crates/sc-compose/tests/cli.rs::release_smoke_covers_render_pipeline_and_observability_health`. |

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

- `quality-mgr` cross-document consistency review passes
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
- `quality-mgr` implementation review finds no blocking contract mismatch

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
- automated repo-boundary verification covering forbidden ATM env/import/manifest
  references
- final end-to-end smoke tests
- final QA and design review pass
- branch prepared for merge to `develop`, then release merge to `main`

Acceptance criteria:

- all FR-1 through FR-11 behavior is implemented and covered by automated tests
- all release blockers are closed
- all required docs match shipped behavior
- downstream cutover notes are accurate
- standalone boundary verification passes with no forbidden ATM runtime
  references or dependencies in source/manifests
- release workflow prerequisites are satisfied

Exit gate:

- `cargo test --workspace` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- full end-to-end smoke test passes using includes, vars, frontmatter, and
  observability-health
- `quality-mgr` full QA pass
- `team-lead` final design review pass
- branch approved for merge to `develop`
- release approved for merge to `main`

## FR Coverage Matrix

- FR-1 through FR-6:
  - already specified in the normative docs
  - revalidated in Sprint 3 and Sprint 4 where release blockers or integration
    changes touch them
- FR-1b:
  - Sprint S7 broadens render inputs from scalar-only to scalar values plus
    arrays of scalars
  - Sprint S7 validates empty-array acceptance and list iteration support
- FR-1d:
  - Sprint S7 defines flat bundled examples, per-template user directories,
    and the `TemplateStore`-based lookup model
- FR-2:
  - Sprint S7 extends precedence handling to include template
    `input_defaults`
- FR-7:
  - Sprint 1 finalizes the command surface
  - Sprint 2 implements `observability-health`
  - Sprint 3 hardens command behavior
  - Sprint 4 validates release behavior
  - Sprint S7 adds `examples list`, `examples <name>`, `templates list`,
    `templates add`, and `templates <name>`
- FR-8 and FR-8a:
  - Sprint 1 finalizes command and health schemas
  - Sprint 2 implements the logger-facing command output
  - Sprint 3 hardens JSON and failure-path behavior
  - Sprint 4 validates release behavior
  - Sprint S7 adds `examples list --json`, `templates list --json`, and
    `templates add --json`
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
- FR-12:
  - Sprint H1 adds structured object input support
- FR-13:
  - Sprint H2 adds arrays of objects and loop-body discovery support
- FR-14:
  - Sprint H3 adds HTML template output as a bundled report example track
- FR-15:
  - Sprint H3 ships the `sprint-report-html` bundled example

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
- `quality-mgr` completes a full QA pass
- `team-lead` completes a final design review
- release is approved for merge to `main`

## Follow-On Work

### Known Limitations

- Undeclared-token diagnostics currently attribute the warning or error to the
  resolved root template path. Per-include-file attribution is deferred because
  it does not block correct render or validation behavior in the initial
  release.
- `ObservationSink::emit()` remains an external extension point for host-owned
  sinks and adapters. Internal composition dispatch uses the typed
  `CompositionObserver` callbacks directly.
- Release determinism is covered by the stable rendering pipeline and golden
  output tests, but the repo does not yet carry a dedicated two-invocation
  byte-for-byte integration test.
- CLI-to-log-file emission is covered by command and observer integration
  tests, but there is not yet a standalone seam test that asserts every
  command event reaches the final sink file on disk.
- Structured object inputs, arrays of objects, and the HTML sprint-report track
  remain planned follow-on work documented in
  [docs/html-sprint-report-plan.md](html-sprint-report-plan.md) and the Phase
  HTML-Report section above.

### Sprint S8: Release Engineering And Distribution

Status:

- completed

Branch:

- `chore/version-bump-1.0.0` -> `develop`

Goals:

- finalize the first standalone `1.0.0` release path for `sc-composer` and
  `sc-compose`
- add release-control infrastructure that prevents accidental duplicate publish
- make Homebrew, `winget`, and packaged GitHub Release installs match the
  documented examples-discovery contract

Deliverables:

- completed as specified in [docs/publishing.md](docs/publishing.md)

Acceptance Criteria:

- workspace and crate manifests are updated to `1.0.0`
- release workflow archives ship `bin/sc-compose` and
  `share/sc-compose/examples/...`
- `scripts/release_gate.sh` exists and enforces release ancestry plus
  unpublished-version checks
- release preflight verifies unpublished crate versions before release
- release workflow publish steps are idempotent when crates are already live
- Homebrew automation updates `randlee/homebrew-tap` from the checked-in formula
  template
- `winget` automation and supporting docs are present for `randlee.sc-compose`
- publishing docs and operator guidance are aligned with the first standalone
  `1.0.0` release path

Exit Gate:

- `SC-RELEASE-ENG-QA-001` passed as the Sprint S8 exit gate

### Sprint S8: Release Engineering And Distribution

Status:

- completed

Branch:

- `chore/version-bump-1.0.0` -> `develop`

Goals:

- finalize the first standalone `1.0.0` release path for `sc-composer` and
  `sc-compose`
- add release-control infrastructure that prevents accidental duplicate publish
- make Homebrew, `winget`, and packaged GitHub Release installs match the
  documented examples-discovery contract

Deliverables:

- completed as specified in [docs/publishing.md](docs/publishing.md)

Acceptance Criteria:

- workspace and crate manifests are updated to `1.0.0`
- release workflow archives ship `bin/sc-compose` and
  `share/sc-compose/examples/...`
- `scripts/release_gate.sh` exists and enforces release ancestry plus
  unpublished-version checks
- release preflight verifies unpublished crate versions before release
- release workflow publish steps are idempotent when crates are already live
- Homebrew automation updates `randlee/homebrew-tap` from the checked-in formula
  template
- `winget` automation and supporting docs are present for `randlee.sc-compose`
- publishing docs and operator guidance are aligned with the first standalone
  `1.0.0` release path

Exit Gate:

- `SC-RELEASE-ENG-QA-001` passed as the Sprint S8 exit gate

### Sprint S7: Examples and Templates Commands

Status:

- completed

Branch:

- `feat/examples-command` -> `develop`

Goals:

- ship a small, reviewable starter set of bundled example files with the tool
- add a user-managed templates surface in the same sprint so created or custom
  templates are immediately usable
- support short named-render UX through command namespaces rather than a longer
  explicit render subcommand
- broaden the input model enough to support array/list-driven examples without
  expanding into hooks or manifest-owned execution logic

Deliverables:

- repo-root `examples/` directory with flat starter example files:
  - `hello.md.j2`
  - `frontmatter-demo.md.j2`
  - `service-config.yaml.j2`
  - `agent-task-branching.xml.j2`
  - `pytest-fixture.py.j2`
- user templates stored as one subdirectory per template under the user
  templates root
- optional `template.json` for user template directories carrying only:
  - `description`
  - `version`
  - `input_defaults`
- `sc-compose examples list`
  - discovers bundled example files through `SC_COMPOSE_DATA_DIR/examples`
    first
  - falls back to install-relative `../share/sc-compose/examples/`
  - lists bundled example files in text or JSON form
- `sc-compose examples <name>`
  - implicitly renders the flat example file matching the requested name
  - uses the same render flags and output behavior as `render`
- `sc-compose templates list`
  - lists user template packs from `SC_COMPOSE_TEMPLATE_DIR` or the platform
    user-data directory joined with `sc-compose/templates/`
- `sc-compose templates add <src> [name]`
  - adds a user template pack from either a single file or a directory source
  - uses `[name]` when provided
  - otherwise uses the source directory name for directory input or the
    normalized template filename for file input
  - fails if the destination pack name already exists
- `sc-compose templates <name>`
  - implicitly renders the single root-level `*.j2` file in the named user
    pack
- a lightweight `README.md` in the user templates root documenting:
  - where user templates live
  - the one-template-per-directory convention
  - the `templates add` and `templates <name>` workflow
- input-model expansion from scalar-only values to:
  - scalar values
  - simple arrays/lists of scalar values
- precedence updates so named-render pack defaults merge as:
  1. explicit input variables
  2. environment-derived variables
  3. user-template `template.json` `input_defaults`
  4. frontmatter defaults
- packaging/install documentation for:
  - Homebrew `#{prefix}/share/sc-compose/examples/`
  - Windows and other system installs using the same relative share layout
  - manual `SC_COMPOSE_DATA_DIR` override for CI and custom installs
  - the default user template root and `SC_COMPOSE_TEMPLATE_DIR` override
- tests for:
  - bundled example root resolution
  - user template root resolution
  - examples/templates listing
  - templates add
  - named render for single-template packs
  - array/list inputs through frontmatter defaults, user-template
    `template.json` `input_defaults`, and `--var-file`

Example design rules:

- examples should be immediately understandable without reading the source code
- each example should remain understandable from frontmatter, filename, and
  minimal inline guidance when needed, without polluting the primary rendered
  output
- the starter set should cover:
  - minimal rendering
  - frontmatter/defaults/validation behavior
  - practical multi-variable configuration generation
  - branching task/agent prompt generation
  - code-generation scaffolding for pytest
- the pytest example should exercise real array/list inputs rather than a
  scalar text-block workaround
- v1 named render resolves flat example files by stem and user templates by
  the single root-level `*.j2` file inside the named template directory

Explicit deferral:

- do not add `prepare-hook`, `post-render-hook`, or any other pack-executed
  hook model
- do not add manifest-owned entrypoint selection, hook declarations, or other
  code-driving fields to `template.json`
- do not add template deletion, update, sync, or remote registry features
- do not add implicit named render for packs with multiple root-level `*.j2`
  candidates

Acceptance criteria:

- all five starter example files exist and are review-ready
- `sc-compose examples` auto-finds bundled example files from install-relative
  share layout or `SC_COMPOSE_DATA_DIR/examples`
- `sc-compose templates` auto-finds the user template root from
  `SC_COMPOSE_TEMPLATE_DIR` or the platform user-data directory joined with
  `sc-compose/templates/`
- `examples list`, `examples <name>`, `templates list`, `templates add`, and
  `templates <name>` work on macOS, Linux, and Windows path conventions
- the user templates root includes a concise `README.md` describing the
  supported workflow and directory convention
- array/list inputs work through `--var-file`, frontmatter defaults, and
  user-template `template.json` `input_defaults`
- `template.json` remains a user-facing metadata/defaults file rather than a
  manifest that drives alternate execution logic
- `templates add` stores file sources as
  `<user-template-root>/<pack-name>/<original-file>` and directory sources as
  `<user-template-root>/<pack-name>/...`
- packager instructions are explicit enough for system package installs and
  user-template discovery

### Phase HTML-Report (H1-H4)

Status:

- planned

Phase goal:

- broaden `sc-compose` into a strong structured HTML report generator without
  moving wrapper-owned browser/display behavior into the core tool.

Release blocker inventory:

| ID | Blocker | Status | Sprint | Closure condition |
| --- | --- | --- | --- | --- |
| HRB-01 | The current input model cannot express structured records such as PR objects and nested field access. | Closed — PR #45, `2280bd1`. All 11 H1 acceptance tests pass including `frontmatter_defaults_accept_object_value` (`crates/sc-composer/src/lib.rs:107`), `render_accepts_object_values_in_json_var_file` (`crates/sc-compose/tests/cli.rs:818`), and `template_json_object_input_defaults_obey_precedence` (`crates/sc-compose/tests/cli.rs:581`). | H1 | Object/map input values render end-to-end with stable field-path diagnostics. |
| HRB-02 | The current input model cannot express repeated report sections as arrays of structured records. | Open | H2 | Arrays of objects render, validate, and support loop-body discovery end-to-end. |
| HRB-03 | There is no bundled HTML report example proving `sc-compose` can generate a useful clickable report artifact. | Open | H3 | `sprint-report-html` renders a self-contained HTML report from realistic structured input. |

#### Sprint H1: Structured Object Input Support

Description:

- expand the value model from scalars and arrays of scalars to include
  object/map values with string keys.

FRs addressed:

- FR-12

Deliverables:

- object/map values accepted through `--var-file` JSON and YAML input
- object values accepted in frontmatter defaults
- object values accepted in `template.json` `input_defaults`
- nested field access documented for Jinja templates
- stable diagnostics for malformed objects and missing nested fields:
  - `ERR_VAL_OBJECT_SHAPE`
  - `ERR_VAL_SHAPE_MISMATCH`
  - `ERR_VAL_MISSING_NESTED_FIELD`
- explicit top-level replacement semantics for structured defaults; no deep
  merge
- explicit top-level extra-variable policy for structured inputs
- invert or replace the three existing negative tests that reject objects:
  - `crates/sc-compose/tests/cli.rs:render_rejects_nested_object_values_in_var_file` (cli.rs:518)
  - `crates/sc-compose/tests/cli.rs:render_rejects_nested_sequence_values_in_var_file` (cli.rs:544)
  - `crates/sc-composer/src/lib.rs:frontmatter_rejects_nested_defaults` (lib.rs:110-122)

Acceptance Criteria:

- object values render end-to-end through `--var-file`
- object values work through frontmatter defaults and `template.json`
  `input_defaults`
- missing nested fields reference stable field paths such as `pr.number`
- structured defaults are replaced, not merged, at the top-level boundary
- unit tests (`sc-composer`) cover:
  - `validate_input_value_accepts_serde_json_object`
  - `input_value_from_yaml_mapping_becomes_object`
  - `frontmatter_defaults_accept_object_value`
  - `required_variable_path_pr_number_is_satisfied_by_object_input`
  - `missing_nested_field_reports_err_val_missing_nested_field`
  - `shape_mismatch_reports_err_val_shape_mismatch`
  - `structured_defaults_replace_without_deep_merge`
  - `extra_nested_fields_are_ignored_by_top_level_extra_input_policy`
- integration tests (`sc-compose`) cover:
  - `render_accepts_object_values_in_json_var_file`
  - `render_accepts_object_values_in_yaml_var_file`
  - `template_json_object_input_defaults_obey_precedence`

Exit Gate:

- object-input behavior is specified in `requirements.md` and `architecture.md`
- automated tests covering object input paths pass
- no open blocker remains against FR-12
- `quality-mgr` sprint_review passes with no blocker findings

#### Sprint H2: Arrays Of Objects Input Support

Description:

- extend the structured-input model so repeated report sections can be modeled
  as arrays of records.

FRs addressed:

- FR-13

Deliverables:

- arrays of objects accepted through `--var-file`
- arrays of objects accepted in frontmatter defaults
- arrays of objects accepted in `template.json` `input_defaults`
- loop-body field access in Jinja templates
- Spike: loop-body discovery approach (MiniJinja AST vs scope-tracker);
  document the decision in `architecture.md` section 21.5 before proceeding
  with the remaining H2 deliverables
- frontmatter-init discovery for nested references inside loop bodies
- nested arrays explicitly remain out of scope for H1/H2 and are rejected with
  `ERR_VAL_NESTED_ARRAY_UNSUPPORTED`
- unit and integration tests for arrays of objects

Acceptance Criteria:

- arrays of objects render end-to-end through Jinja loops
- frontmatter-init discovers loop-body variable references from array members
- nested arrays are rejected with `ERR_VAL_NESTED_ARRAY_UNSUPPORTED`
- at least 10 tests cover arrays-of-objects behavior and failure cases
- the `sprint-report-html` input shape is representable by the implemented value
  model

Exit Gate:

- all H2 deliverables complete
- the loop-body discovery spike is documented in `architecture.md` section 21.5
- automated tests covering arrays of objects pass
- no open blocker remains against FR-13
- `quality-mgr` sprint_review passes with no blocker findings

#### Sprint H3: `sprint-report-html` Bundled Example

Description:

- ship a self-contained single-panel HTML sprint report example that produces an
  immediately useful clickable artifact.

FRs addressed:

- FR-14
- FR-15

Deliverables:

- H3a (FR-14 implementation): reuse the existing `.j2` suffix-stripping output
  path behavior already implemented by `strip_j2_suffix`; H3 does not
  re-implement output-path logic
- H3a adds:
  - at least one integration test verifying
    `sprint-report-html.html.j2 -> sprint-report-html.html`
  - an explicit safety note that `.html.j2` templates do not use automatic
    escaping
- H3b (FR-15 content): bundled example at
  `examples/sprint-report-html.html.j2`
- H3b keeps all template content inline in a single flat file; no `_includes/`
  directory and no directory-based example pack
- realistic sample vars file with PR and CI data
- self-contained HTML output with inline CSS and no external dependencies
- action links for:
  - view PR
  - view CI run
  - merge URL

Acceptance Criteria:

- `sc-compose examples sprint-report-html --var-file sample-vars.json` works
  end-to-end
- rendered HTML is self-contained and browser-viewable
- rendered output includes working PR, CI, and plan/findings links from sample
  data
- the example clearly demonstrates why structured inputs are better than
  flattened prebuilt strings

Exit Gate:

- the bundled example renders successfully from realistic structured input
- design review confirms the example is a credible showcase artifact
- H3 remains a single flat example file with no bundled-example layout change
- no open blocker remains against FR-14 or FR-15 for the single-panel scope
- `quality-mgr` sprint_review passes with no blocker findings

#### Sprint H4: Multi-Panel Report And Wrapper Integration

Description:

- extend the single-panel example into a fuller report and connect it to the
  wrapper workflow without moving open/display behavior into `sc-compose`.

H4 introduces no new functional requirements. All H4 work extends FR-12,
FR-13, FR-14, and FR-15 with wrapper integration and multi-panel example work.
This is intentional.

Deliverables:

- multi-panel report layout with repeated per-sprint sections
- stage-sensitive panel sections or variants
- `/sprint-report` skill update that renders the HTML artifact and opens or
  writes it from wrapper logic
- architecture/docs update describing the wrapper-owned orchestration pattern

Acceptance Criteria:

- `/sprint-report --html` produces the HTML report through wrapper-owned render
  orchestration
- the wrapper path opens or writes the output without requiring hook execution
  in `sc-compose`
- the multi-panel layout remains self-contained and deterministic

Exit Gate:

- wrapper integration works without changing `sc-compose` into a workflow
  orchestrator
- quality review confirms the final report flow is usable and maintainable
- all HTML-Report phase blockers are closed
- `quality-mgr` sprint_review passes with no blocker findings

## Companion Planning Docs

- `docs/traceability-matrix.md`
- `docs/error-code-registry.md`
- `docs/test-strategy.md`
- `docs/html-sprint-report-plan.md`

## Follow-on Design Track

The current plan is the authoritative release plan for `1.0`. Additional
post-`1.0` design exploration must not silently rewrite the shipped contract.

The current follow-on design track is:

- `docs/html-sprint-report-plan.md`
  - structured input-value expansion for maps/objects and arrays of objects,
  - XHTML sprint-report example/template design,
  - wrapper-owned browser-open workflow rather than hook execution in
    `sc-compose`.

## Rule

Any follow-on sprint added after this plan must preserve the standalone
boundary defined by:

- `docs/requirements.md`
- `docs/architecture.md`
- `docs/git-workflows.md`
- `docs/publishing.md`
