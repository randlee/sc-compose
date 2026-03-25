# SC-Compose Project Plan

## Status

This repo is in initial extraction/setup.

The immediate goal is to establish:
- correct standalone crate boundaries
- zero `agent-team-mail-*` dependencies
- a clean publishable workspace structure

## Near-Term Work

1. Set up repository git flow:
   - use `main` and `develop`
   - feature branches target `develop`
   - release tags and release publication come from `main`
   - keep repo workflow and review discipline aligned with ATM
2. Match GitHub automation and protection to ATM:
   - CI triggers match the ATM repo pattern for `pull_request` and `push`
   - branch protection and rulesets match ATM for `main` and `develop`
   - GitHub secrets and environments are configured and use the same variable
     names as ATM where the workflows overlap
3. Verify repository setup end to end:
   - release preflight validates publish order and version alignment
   - release workflow is ready to publish `sc-composer` then `sc-compose`
   - workspace version stays above the source ATM workspace version that last
     published these crate names
4. Complete crates.io ownership and release readiness:
   - verify crate ownership/maintainers for `sc-composer` and `sc-compose`
   - verify publish tokens and first-release permissions
   - document the handoff from ATM-published crates to this repo
5. Make `sc-composer` fully standalone.
6. Remove any `ATM_HOME` or ATM path assumptions from `sc-compose`.
7. Verify ATM cutover readiness:
   - published crate names match the existing names used in ATM
   - replacement instructions are documented
   - no `agent-team-mail-*` dependencies remain
8. Write the migration plan after the agents are live and operating on the new
   repos.

## Implementation Phase

After repository extraction is stable, the next implementation phase is the
FR-1 through FR-9 redesign defined in:

- `docs/requirements.md`
- `docs/architecture.md`

The crate-development phase is a six-sprint program:

- `S1` completed the normative documentation and review baseline.
- `S2` through `S6` implement and harden `sc-composer` and `sc-compose`.

A later sprint may start only after the prior sprint exit gate passes.

For sprint exit gates in this document, `qm-comp` is the QA/review agent
responsible for validating that implementation matches the normative docs and
the active quality gates.

### Sprint 1: Spec and Planning Baseline

Status:

- complete

Branch:

- `fix/docs-*` and `fix/atm-review-findings` branches into `develop`

FRs addressed:

- FR-1 through FR-9 at the specification level

Deliverables:

- normative `docs/requirements.md`
- normative `docs/architecture.md`
- initial project-plan baseline
- failure-mode matrix and per-command JSON schema definitions

Acceptance criteria:

- requirements and architecture are internally consistent
- ATM independence and host-injection boundaries are explicit
- `qm-comp` design/doc QA passes
- `arch-ctm` review findings are resolved or assigned forward

Exit gate:

- docs merged to `develop`
- planning may proceed against a stable spec baseline

### Sprint 2: Core Types, Errors, and Diagnostics

Branch:

- `feature/s2-core-types` -> `develop`

FRs addressed:

- FR-1
- FR-1a
- FR-1b
- FR-2
- FR-8

Deliverables:

- `crates/sc-composer/src/types.rs` or equivalent modules for:
  - `ScalarValue`
  - `MetadataValue`
  - `VariableName`
  - `IncludeDepth`
  - `ConfiningRoot`
  - `ComposeMode`
  - `ComposeRequest`
  - `ComposePolicy`
  - `ComposeResult`
  - `ValidationReport`
- `crates/sc-composer/src/error.rs` implementing:
  - `ResolveError`
  - `IncludeError`
  - `ValidationError`
  - `RenderError`
  - `ConfigError`
  - `ComposeError`
- `crates/sc-composer/src/diagnostics.rs` implementing:
  - concrete `Diagnostic`
  - stable diagnostic code registry aligned with the `ERR_*` matrix
  - top-level FR-8 JSON envelope model mirrored in architecture section 10
- `crates/sc-composer/src/frontmatter.rs` for typed frontmatter parsing and
  normalization
- crate docs and public-item docs for the new API surface

Carry-forward QA backlog assigned to Sprint 2:

- RenderError path test coverage
- doc comments on `RenderError::render` and `RenderError::backtrace`
- mirror the FR-8 top-level JSON envelope into `docs/architecture.md` section
  10 when the diagnostics implementation lands

Acceptance criteria:

- public APIs do not leak template-engine or third-party error types
- frontmatter normalization matches FR-1a exactly
- `ComposeMode` uses variant-specific fields rather than option soup
- `ComposeError` and the failure-mode matrix codes align one-to-one
- diagnostics can serialize the FR-8 envelope and concrete diagnostic records
- RenderError tests cover construction, source propagation, and backtrace
  accessors
- public doc comments are present on `RenderError::render` and
  `RenderError::backtrace`

Exit gate:

- `cargo test -p sc-composer` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `qm-comp` API/diagnostics review finds no blocking mismatch

### Sprint 3: Resolver and Include Engine

Branch:

- `feature/s3-resolver-include` -> `develop`

FRs addressed:

- FR-1c
- FR-3
- FR-4
- FR-5

Deliverables:

- `crates/sc-composer/src/resolver.rs` implementing:
  - runtime-aware path policy
  - profile-kind directory search
  - search trace capture
  - ambiguity detection and `ResolveResult`
- `crates/sc-composer/src/include.rs` implementing:
  - `@<path>` expansion
  - include cycle detection
  - include depth enforcement
  - confinement-root enforcement
  - include stack capture for diagnostics
- path normalization and confinement helpers respecting `ConfiningRoot`
- unit tests for resolver precedence and include failure modes

Acceptance criteria:

- omitted-runtime search and ambiguity behavior matches FR-5
- include resolution order matches FR-3
- path escape attempts fail with `ERR_INCLUDE_ESCAPE`
- include depth overflow fails with `ERR_INCLUDE_DEPTH`
- search traces are captured for `resolve --json`

Exit gate:

- resolver tests cover agent, command, and skill lookup across runtime/shared
  roots
- include tests cover missing file, cycle, depth overflow, and escape attempts
- `cargo test -p sc-composer resolver include` equivalent coverage passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes

### Sprint 4: Validation and Rendering Core

Branch:

- `feature/s4-validation-renderer` -> `develop`

FRs addressed:

- FR-2
- FR-2a
- FR-2b
- FR-1b
- FR-3a
- FR-6
- FR-7c
- FR-8
- FR-9 at the library-hook level

Deliverables:

- `crates/sc-composer/src/validation.rs` implementing context merge and token
  discovery (originally planned as separate `context.rs` and `tokens.rs` files;
  consolidated into `validation.rs` — see architecture.md §4 for rationale):
  - precedence merge (explicit input > environment-derived > frontmatter defaults)
  - variable origin tracking and unknown-variable policy
  - referenced-token discovery (declared, undeclared, missing, extra)
- `crates/sc-composer/src/validate.rs` implementing:
  - missing required variable checks
  - undeclared token handling
  - extra-variable policy handling
  - `validate() -> Result<ValidationReport, ComposeError>`
- `crates/sc-composer/src/render.rs` implementing:
  - `Renderer`
  - `render_template()`
  - trim/lstrip default behavior
  - strict and default undeclared-token behavior
- `crates/sc-composer/src/pipeline.rs` implementing typestate transitions:
  - `Document<Parsed>`
  - `Document<Expanded>`
  - `Document<Validated>`
  - `Document<Rendered>`
- `crates/sc-composer/src/observability.rs` implementing open observer/sink
  traits and a built-in no-op implementation
- `compose()` wiring over resolver, include, validation, render, and block
  assembly

Acceptance criteria:

- `Renderer` owns template loading, include resolution, variable expansion,
  validation, and rendering as documented
- `compose()` is the end-to-end convenience function over `Renderer`
- `render_template()` works as the lower-level entry point for resolved
  template content
- default mode preserves undeclared tokens; strict mode fails on them
- include-derived defaults and required variables merge per FR-3a
- stable diagnostics and `ERR_*` mappings are emitted for all failure classes
- observer injection works with a host-supplied implementation and with the
  no-op default

Exit gate:

- `cargo test -p sc-composer` passes with dedicated suites for context,
  validation, rendering, and observability hooks
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `qm-comp` review confirms API ownership and failure mapping match docs

### Sprint 5: CLI and Workspace Helpers

Branch:

- `feature/s5-cli-workspace` -> `develop`

FRs addressed:

- FR-6
- FR-7
- FR-7a
- FR-7b
- FR-7c
- FR-8a
- FR-8
- FR-9 at the CLI integration level

Deliverables:

- `crates/sc-compose/src/main.rs` or subcommand modules for:
  - `render`
  - `resolve`
  - `validate`
  - `frontmatter-init`
  - `init`
- CLI argument parsing for:
  - `--mode`
  - `--kind`
  - `--agent` and alias normalization
  - `--runtime` and alias normalization
  - `--var`
  - `--var-file`
  - `--env-prefix`
  - `--guidance` and `--guidance-file`
  - `--prompt` and `--prompt-file`
  - `--json`
  - `--dry-run`
  - `--output`
- `crates/sc-composer/src/workspace.rs` implementing:
  - `frontmatter_init()`
  - `init_workspace()`
- CLI JSON-output shapers matching the requirements and architecture schemas
- output-path derivation for file mode and profile mode
- CLI-side `sc-observability` binding over the open observer/sink traits

Acceptance criteria:

- `render`, `resolve`, `validate`, `frontmatter-init`, and `init` all delegate
  core semantics to `sc-composer`
- command JSON outputs match the documented schemas exactly
- dry-run outputs match the documented schemas exactly
- `resolve` is rejected in file mode
- stdin double-read conflicts fail with `ERR_RENDER_STDIN_DOUBLE_READ`
- output write failures map to `ERR_RENDER_WRITE`

Exit gate:

- CLI integration and golden tests pass for all commands
- JSON schema snapshots pass for normal and dry-run modes
- `cargo test -p sc-compose` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `qm-comp` QA finds no blocking mismatch in command behavior or schema output

### Sprint 6: Integration, Hardening, and Release Gate

Branch:

- `feature/s6-integration-gate` -> `develop`

FRs addressed:

- FR-1 through FR-9 end-to-end

Deliverables:

- end-to-end smoke-test assets covering:
  - frontmatter
  - includes
  - explicit vars
  - env vars
  - var-files
  - profile resolution
  - output-path derivation
- cross-platform path/confinement verification cases
- release-readiness checklist for both crates
- final migration and cutover notes for downstream consumers
- issue triage pass for any non-blocking carry-over discovered in S2-S5

Acceptance criteria:

- all FRs are implemented and mapped to passing tests
- `compose()`, `Renderer`, `render_template()`, `validate()`,
  `init_workspace()`, and `frontmatter_init()` behave as documented
- failure-mode matrix codes are exercised by tests
- standalone boundaries remain intact with no ATM-specific assumptions in code
  or manifests
- no open Priority 1 or Priority 2 QA findings remain

Exit gate:

- `cargo test --workspace` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- full end-to-end smoke test passes: render a template with includes, vars, and
  frontmatter
- `qm-comp` full QA pass
- `arch-ctm` final design review
- branch approved for merge to `develop`

## Crate Build Sequence

Implementation order is constrained by the architecture typestate pipeline and
crate dependency direction.

1. `sc-composer` foundational types and diagnostics:
   - `types.rs` or the equivalent foundational type modules
   - `error`
   - `diagnostics`
   - `frontmatter`
2. `sc-composer` path and graph mechanics:
   - `resolver`
   - `include`
3. `sc-composer` semantic pipeline:
   - `context`
   - `tokens`
   - `render`
   - `validate`
   - `pipeline`
4. `sc-composer` integration hooks:
   - `observability`
   - `workspace`
5. `sc-compose` CLI wiring:
   - argument parsing
   - command routing
   - JSON shaping
   - output writing
   - concrete observability binding

Modules that can be parallelized once the shared types exist:

- `resolver` and `frontmatter` may proceed in parallel after Sprint 2
  foundational type modules land
- `include` and `tokens` may proceed in parallel once path and document
  representations stabilize
- `observability` and `workspace` can proceed in parallel with late Sprint 4
  or early Sprint 5 CLI work
- CLI JSON shaping and output-path handling can proceed in parallel once
  `ComposeResult`, `ValidationReport`, and command schema contracts stabilize

Parallel work must not violate ownership:

- `sc-composer` remains the only crate that defines composition semantics
- `sc-compose` implements UX and transport only

## FR Coverage Matrix

- FR-1, FR-1a: S2
- FR-1b: S2 and S4
- FR-1c: S3
- FR-2, FR-2a, FR-2b: S2 and S4
- FR-3: S3
- FR-3a: S4
- FR-4: S3 and S6
- FR-5: S3
- FR-6: S4 and S5
- FR-7, FR-7a, FR-7b: S5
- FR-7c: S4 and S5
- FR-8: S2, S4, S5, and S6
- FR-8a: S5 and S6
- FR-9: S4 and S5
- NFRs:
  - cross-platform behavior: S3 and S6
  - interactive performance expectations: S5 and S6
  - public API stability: S2 and S6
  - crate separability and boundary enforcement: S2 through S6

## Phase Exit Gate

The crate-development phase is complete only when Sprint 6 passes all of the
following:

- all prior sprint exit gates for S2 through S5 have already passed
- all FR-1 through FR-9 behavior is implemented and covered by automated tests
- `cargo test --workspace` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- the failure-mode matrix `ERR_*` codes are reflected in the emitted
  diagnostics and covered by tests
- a full end-to-end smoke test passes using includes, vars, and frontmatter
- `qm-comp` completes a full QA pass
- `arch-ctm` completes a final design review

If implementation is re-sliced, the replacement plan must preserve the
dependency order, FR coverage, and exit gates defined above.

## Companion Planning Docs

The following documents reduce execution ambiguity for implementation agents and
reviewers:

- `docs/traceability-matrix.md`
- `docs/error-code-registry.md`
- `docs/test-strategy.md`

## Rule

Any sprint plan added here must preserve the standalone boundary defined by:
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/git-workflows.md`
- `docs/publishing.md`
