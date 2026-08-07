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
- `bindings/python` is a Python-facing adapter package that may depend on
  `sc-composer` only.
- `bindings/python` must not depend on `sc-compose`, `sc-observability`, or
  ATM-specific crates.
- `sc-composer` must not depend on `bindings/python`.
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

- all FR-1 through FR-15 behavior is implemented and covered by automated tests
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
  - Phase D adds multi-pass `render --all`, pass-scoped `--pass N` / `--var`
    groups, `verify`, and `template-init`
  - Phase D also lands delimiter hardening for custom variable delimiters:
    `Renderer::with_delimiters` is now fallible on invalid delimiters and ships
    in `1.3.0` under the narrow ADR-0010 stability exception
- FR-7b:
  - Phase D assigns `verify` drift to exit code `1`
  - Phase D keeps render and validation failures on `2`
  - Phase D keeps usage and configuration failures, including `template-init`
    literal-miss cases, on `3`
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
- FR-12 through FR-15 (H4):
  - Sprint H4 extends wrapper integration across FR-12–FR-15 and finalizes source-of-truth documentation; introduces no new functional requirements
- FR-16 and Phase H:
  - Phase G establishes the known-template XML-first extraction contract and
    its scalar-only boundaries
  - Phase H.1 owns the contract amendments for issue #193's JSON, YAML, and
    TOML extensions and plans the migration of format-neutral value matching
    from the XML path into a shared raw-text core
  - Phase H.2 through H.8 own implementation, cross-surface parity, corpus,
    adversarial hardening, and phase-ending remediation for those accepted
    amendments
  - XML mixed-content, XML dirty-prefix tolerance, and customer-facing
    raw-text mode remain outside Phase H; they are now planned as Phase I
    work in `docs/phase-I/phase-I-plan.md`, not retroactively claimed as H
    behavior
- FR-17 through FR-21 and Phase I:
  - I.1 accepts the raw-text, XML recovery, loop-context, and YAML merge-key
    contracts in ADR-0013 and the synchronized requirements/architecture docs
  - I.2 through I.6 own runtime implementation, cross-surface parity, and
    evidence for those accepted requirements; I.1 adds no executable code

## Production Readiness Gate

Release is complete only when all numbered Phase-H sprints have passed and all of the
following are true:

- no release blocker remains open
- `requirements.md`, `architecture.md`, and `project-plan.md` match the shipped
  behavior
- all FR-1 through FR-15 behavior is implemented and covered by automated tests
- `cargo test --workspace` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo fmt --all --check` passes
- full end-to-end smoke coverage passes
- `quality-mgr` completes a full QA pass
- `team-lead` completes a final design review
- release is approved for merge to `main`

## Follow-On Work

### Phase C Sprint Plans

Status:

- planned follow-on Python bindings work after the shipped `1.1.x` line

Sprint entries:

- [Sprint C.1 — Maturin Python Bindings Foundation](phase-C/sprint-c-1-maturin-bindings.md)
- [Sprint C.2 — Python API Surface](phase-C/sprint-c-2-python-api-surface.md)
- [Sprint C.3 — Python Release Train And Packaging Hardening](phase-C/sprint-c-3-python-release-train.md)
- [Phase C README](phase-C/README.md)
- [Maturin Bindings Investigation](phase-C/maturin-bindings-investigation.md)

These sprint plans define the first implementation path for a Python-facing
adapter package that depends on `sc-composer` while keeping `sc-composer` a
pure Rust library and keeping reporting plus observability APIs out of the
initial Python scope.

### Phase D Sprint Plans

Status:

- complete on `integrate/phase-d`; all eight D-track sprints have passed QA
  and merged, and PR #140 is open to promote the phase to `develop`

Sprint entries:

- [Sprint D.1 — Multi-Pass Library Foundation](phase-D/sprint-d-1-library-foundation.md)
- [Sprint D.1-py — Python Bindings — Multi-Pass Library Foundation](phase-D/sprint-d-1-py-bindings.md)
- [Sprint D.2 — Multi-Pass Composition Pipeline](phase-D/sprint-d-2-composition-pipeline.md)
- [Sprint D.2-py — Python Bindings — Multi-Pass Composition Pipeline](phase-D/sprint-d-2-py-bindings.md)
- [Sprint D.3 — Multi-Pass CLI Surface](phase-D/sprint-d-3-cli-surface.md)
- [Sprint D.3-py — Python Bindings — Multi-Pass CLI Surface Parity Check](phase-D/sprint-d-3-py-bindings.md)
- [Sprint D.4 — template-init + verify](phase-D/sprint-d-4-template-init-verify.md)
- [Sprint D.4-py — Python Bindings — template-init + verify](phase-D/sprint-d-4-py-bindings.md)
- [Phase D README](phase-D/README.md)

These sprint plans define the first implementation path for multi-pass
stacked-header template rendering in `sc-composer` and `sc-compose`,
closing 10 of the 11 gaps identified in the prototype gap analysis
(prototype/multipass/docs/gaps.md). GAP-4 (Renderer::with_delimiters)
was already closed by Phase C.2.

### Phase B Cleanup Sprint Plans

Status:

- planned follow-on cleanup after the integrated Phase B production-readiness review

Sprint entries:

- [Sprint B11 — Contract-Alignment](sprints/b11-contract-alignment.md)
- [Sprint B12 — JSON Surface Hardening](sprints/b12-json-surface-hardening.md)
- [Sprint B13 — Observability Panic Removal](sprints/b13-observability-panic-removal.md)
- [Sprint B14 — CLI Extraction](sprints/b14-cli-extraction.md)
- [Sprint B15 — Reporting Runtime Cleanup](sprints/b15-reporting-runtime-cleanup.md)
- [Issues Inventory](issues-inventory.md)

These sprint plans target `integrate/phase-B` follow-on cleanup work. They do
not change the completed Phase B execution record; they capture the next
implementation slices needed to close the remaining production-readiness gaps.
The accepted cleanup findings and sprint ownership are tracked in
[docs/issues-inventory.md](issues-inventory.md).

### Phase E Sprint Plans

Status:

- draft follow-on work for recursive structured inputs and adversarial rendering
  validation after the completed Phase D multi-pass delivery

Sprint entries:

- [Phase E plan](phase-E/phase-E-plan.md)
- [Sprint E.1 — Recursive Structured Input Support](phase-E/sprint-e-1-recursive-structured-input.md)
- [Sprint E.2 — Adversarial Fuzzing Workflow](phase-E/sprint-e-2-adversarial-fuzzing.md)
- [Sprint E.3 — First Adversarial Campaign And Regression Closure](phase-E/sprint-e-3-first-adversarial-campaign.md)
- [Sprint 157 — Multi-Agent Fuzz-Session Report Template](sprint-fuzz-run-report-template.md)
- [Sprint — Top-Level Report Shell Template](sprint-fuzz-run-report-shell-template.md)
- [Sprint — Per-Report Artifact Subdirectory Layout](sprint-fuzz-report-artifact-layout.md)

E.1 changes the runtime input contract, E.2 defines the multi-agent QA
workflow, E.3 proves that workflow against the expanded contract, and Sprint
157 defines the single-page multi-panel report package emitted by a session.

### Phase F Sprint Plans

Status:

- complete on `integrate/phase-f` at
  `0360fb790fdc52541d6ff4e3faebd0618b2ff370`; PRs #174-#178 are merged and
  format, clippy, manifest-validation, workspace tests, and Python-wheel CI
  checks are green on macOS, Ubuntu, and Windows.

Sprint entries:

| ID | Sprint | Status | Branch | Worktree |
| --- | --- | --- | --- | --- |
| F.1 | [CLI Input Parsing and JSON Capability Seams](phase-F/sprint-f-1-cli-input-parsing.md) | complete | `sprint/f-1-cli-input-parsing` | `/Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-1-cli-input-parsing` |
| F.2 | [Main Dispatch Runner and Process Boundary](phase-F/sprint-f-2-main-dispatch-runner.md) | complete | `sprint/f-2-main-dispatch-runner` | `/Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-2-main-dispatch-runner` |
| F.3 | [CLI Integration Test Decomposition](phase-F/sprint-f-3-integration-test-decomposition.md) | complete | `sprint/f-3-integration-test-decomposition` | `/Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-3-integration-test-decomposition` |
| F.4 | [Var-File Decode and Validation Split](phase-F/sprint-f-4-var-file-decode-split.md) | complete | `sprint/f-4-var-file-decode-split` | `/Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-4-var-file-decode-split` |
| F.5 | [Observer Typed Event Builder](phase-F/sprint-f-5-observer-event-builder.md) | complete | `sprint/f-5-observer-event-builder` | `/Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-5-observer-event-builder` |

F.1 through F.5 convert the five findings from the read-only Repowise review
into one-file-or-boundary-owned implementation sprints. They preserve the
sc-composer pure-library boundary, the Python adapter boundary, and the
standalone observability dependency direction.

Required Phase F merge order:

- `F.1 -> F.2 -> F.4 -> F.5 -> F.3`
- F.2, F.4, and F.5 must rebase onto the preceding sprint's merged `develop`
  state before implementation. F.3 is deliberately last because it
  decomposes the shared `tests/cli.rs` and `tests/json_cli.rs` suites after all
  earlier sprints' test additions have landed.

Unnumbered Phase F follow-on work:

- Add a text/JSON equivalence matrix after F.3; it is intentionally not part
  of F.3's closed decomposition scope or acceptance gate.
- The canonical `.claude/skills/codex-orchestration/sprint-plan.md.j2` tooling
  defect was fixed by the follow-on FIX-238 sprint. The parser now preserves
  adjacent rendered-document frontmatter containing Jinja syntax as template
  body, and the canonical template validates and renders end to end.

### Phase G Sprint Plans

Status:

- complete: a known-template, XML-first `sc-compose extract` feature informed
  by prior reverse-extraction research, with Python as a first-class customer
  surface. G.1 through G.7 landed; the G.6 adversarial-evidence campaign
  reports all required gates PASS with no unresolved candidates.

Sprint entries:

- [Phase G plan](phase-G/phase-G-plan.md)
- [Sprint G.1 — Extraction Contract and Analysis Model](phase-G/sprint-g-1-extraction-contract.md)
- [Sprint G.2 — Deterministic XML Extraction Engine](phase-G/sprint-g-2-xml-extraction-engine.md)
- [Sprint G.3 — Python Extraction Bindings](phase-G/sprint-g-3-python-extraction-bindings.md)
- [Sprint G.4 — CLI Extract Surface](phase-G/sprint-g-4-cli-extract-surface.md)
- [Sprint G.5 — Corpus and Regression Closure](phase-G/sprint-g-5-corpus-hardening.md)
- [Sprint G.6 — Adversarial Evidence and Hardening](phase-G/sprint-g-6-adversarial-evidence.md)
- [Sprint G.7 — Reject Dotted Extraction Expressions](phase-G/sprint-g-7-dotted-expression-rejection.md)

Phase G is intentionally narrower than a general inverse-Jinja feature. It
starts with known-template XML extraction, gives the first customer a Python
binding over the same library semantics, reports unsupported or ambiguous
constructs instead of fabricating values, and leaves unknown-template
identification, JSON/Markdown adapters, loop reconstruction, and typed-value
recovery as separately planned follow-on work.

### Phase H Sprint Plans

Status:

- complete closure of the in-scope real-customer reverse-extraction gaps
  recorded in issue #193; H.1's contract and H.2 through H.8 implementation,
  parity, corpus, hardening, and closure gates are complete
- H.7 owns the promoted JSON/YAML depth-diagnostic hardening and the related
  release-registry documentation updates
- H.8 owns the phase-ending remediation and production-readiness gate for the
  remaining YAML, TOML, diagnostic, documentation, and evidence findings

Sprint entries:

- [Phase H plan](phase-H/phase-H-plan.md)
- [Sprint H.1 — Reverse Extraction Format Contract](phase-H/sprint-h-1-reverse-extraction-extension-contract.md)
- [Sprint H.2 — JSON Extraction Core](phase-H/sprint-h-2-json-extraction-core.md)
- [Sprint H.3 — JSON Cross-Surface Parity](phase-H/sprint-h-3-json-cross-surface-parity.md)
- [Sprint H.4 — YAML Extraction](phase-H/sprint-h-4-yaml-extraction.md)
- [Sprint H.5 — TOML Extraction](phase-H/sprint-h-5-toml-extraction.md)
- [Sprint H.6 — Cross-Format Corpus and Adversarial Closure](phase-H/sprint-h-6-cross-format-closure.md)
- [Sprint H.7 — Alias and Input-Limit Hardening](phase-H/sprint-h-7-alias-input-limit-hardening.md)
- [Sprint H.8 — Phase-Ending Review Remediation](phase-H/sprint-h-8-phase-ending-remediation.md)

Disambiguation: the completed `Phase HTML-Report (H1-H4)` section above uses
undotted H1/H2/H3/H4 labels for the HTML-Report feature family. The `Phase H`
section here uses dotted H.1–H.8 identifiers for reverse-extraction extension
sprints; these are separate phases and sprint sequences.

### Phase I Sprint Plans

Status:

- completed Phase I work for the remaining XML extraction gaps from issue #193,
  the strict-validation loop-context gap in issue #167, and the YAML var-file
  merge-key safety gap in issue #166;
- I.1 Contract, Raw-Text Semantics, and Traceability: complete/accepted on
  `sprint/i-1-contract-and-traceability` at `6604de7` (docs-only contract gate);
- I.2 Customer-Facing Raw-Text Mode: complete/accepted on
  `sprint/i-2-customer-raw-text-mode` at `ac6d62f`, with Rust, CLI, Python,
  and Markdown/text evidence;
- I.3 XML Block and Mixed-Content Extraction: complete/accepted on
  `sprint/i-3-xml-block-mixed-content` at `a3c0ce1` after the QA remediation pass;
- I.4 XML Dirty-Prefix Normalization: complete on
  `sprint/i-4-xml-dirty-prefix` at `f3dca07`, with Rust, CLI, Python, and boundary evidence;
- I.5 Jinja Loop-Context Built-ins: complete/accepted on
  `sprint/i-5-loop-context-builtins` at `8cde64d`, with QA-2 PASS 10/10 and
  strict validation evidence;
- I.6 YAML Merge-Key Var-File Safety: complete on
  `sprint/i-6-yaml-merge-key-safety` at `e62cfea/1232c8d`, with fail-closed Rust/CLI
  coverage, JSON and alias controls, and source-located diagnostics;
- Phase I also makes the shared H raw-text matcher customer-facing for known
  Markdown/text templates because raw text is the product use case that the
  format adapters alone do not provide;
- I.1 is the contract and traceability gate. I.2 through I.6 are independent
  implementation tracks after that gate except that I.3 depends on I.2's
  public matcher seam. Independent sprint QA may proceed in parallel; no
  sprint is required to wait for an unrelated sprint's QA result.

Sprint entries:

- [Phase I plan](phase-I/phase-I-plan.md)
- [Sprint I.1 — Contract, Raw-Text Semantics, and Traceability](phase-I/sprint-i-1-contract-and-traceability.md)
- [Sprint I.2 — Customer-Facing Raw-Text Mode](phase-I/sprint-i-2-customer-raw-text-mode.md)
- [Sprint I.3 — XML Block and Mixed-Content Extraction](phase-I/sprint-i-3-xml-block-mixed-content.md)
- [Sprint I.4 — XML Dirty-Prefix Normalization](phase-I/sprint-i-4-xml-dirty-prefix.md)
- [Sprint I.5 — Jinja Loop-Context Built-ins](phase-I/sprint-i-5-loop-context-builtins.md)
- [Sprint I.6 — YAML Merge-Key Var-File Safety](phase-I/sprint-i-6-yaml-merge-key-safety.md)
- [Phase I Backlog Cleanup](phase-I/backlog-cleanup.md)

### Phase J Sprint Plans

Status:

- all four behavior-preserving maintainability decomposition sprints are
  merged into `integrate/phase-j` (J.1 PR #228, J.2 PR #229, J.3 PR #230,
  and J.4 PR #231)
- J.1 is independent; J.2 precedes J.3; J.4 follows J.2 and J.3
- all four are full implementation sprints with the complete Phase J
  validation checklist, not planning/design sprints

Sprint entries:

- [Phase J plan](phase-J/phase-J-plan.md)
- [Sprint J.1 — CLI Argument and Pass-Input Seams](phase-J/sprint-j-1-cli-argument-seams.md)
- [Sprint J.2 — Validation State and Context Assembly](phase-J/sprint-j-2-validation-state-assembly.md)
- [Sprint J.3 — Validation Policy and Required-Path Diagnostics](phase-J/sprint-j-3-validation-policy-diagnostics.md)
- [Sprint J.4 — Frontmatter Parser and Normalizer Split](phase-J/sprint-j-4-frontmatter-parser-split.md)

### Follow-on Fix Sprint: FIX-238

Status:

- complete on `fix/frontmatter-parser-adjacent-delimiter` at `226ebbc`

Sprint entry:

- [Sprint FIX-238 — Frontmatter Parser Adjacent Delimiter](sprints/fix-frontmatter-adjacent-delimiter.md)

FIX-238 closes the canonical sprint-plan template regression in which an
adjacent rendered-document `---` block containing Jinja syntax was incorrectly
parsed as a second YAML config block. Plain-YAML stacked headers remain
supported, while the canonical template now validates and renders with the
conditional `worktree` field both set and unset.

### Follow-on Fix Sprint: FIX-246

Status:

- complete on `fix/246-strict-ignores-custom-delimiters` at `0c7d90c`

Sprint entry:

- [Sprint FIX-246 — Strict Validation Custom Delimiters](sprints/fix-246-strict-ignores-custom-delimiters.md)

FIX-246 threads active custom variable delimiters through strict undeclared-
token validation. Literal default-delimiter text remains inert under custom
delimiters, while undeclared variables referenced through the active custom
delimiters now fail closed with `ERR_VAL_UNDECLARED_TOKEN`.

### Follow-on Fix Sprint: FIX-243

Status:

- complete on `fix/243-frontmatter-silent-data-loss` at `01a1e5c`

Sprint entry:

- [Sprint FIX-243 — Frontmatter Parser Silent Data Loss](sprints/fix-243-frontmatter-silent-data-loss.md)

FIX-243 preserves adjacent plain-YAML blocks with unrecognized top-level keys
as template body while retaining recognized-key multi-pass stacking and the
FIX-238 Jinja-syntax break behavior.

### Follow-on Fix Sprint: FIX-244

Status:

- complete on `fix/244-whitespace-control-phantom-dash` at `ac7c139`

Sprint entry:

- [Sprint FIX-244 — Jinja Whitespace-Control Phantom Dash](sprints/fix-244-whitespace-control-phantom-dash.md)

FIX-244 strips delimiter-adjacent Jinja whitespace-control markers before
token discovery without changing kebab-case variable support.

### Follow-on Fix Sprint: FIX-245

Status:

- complete on `fix/245-opening-delimiter-trailing-whitespace` at `61aadec`

Sprint entry:

- [Sprint FIX-245 — Opening Frontmatter Delimiter Trailing Whitespace](sprints/fix-245-opening-delimiter-trailing-whitespace.md)

FIX-245 accepts spaces and tabs after an opening `---` delimiter before LF,
CRLF, or EOF, while preserving strict closing-delimiter matching and the
resulting `ERR_CONFIG_PARSE` for trailing-whitespace closing lines.

### Follow-on Fix Sprint: FIX-247

Status:

- complete on `fix/247-expand-file-stack-overflow` at `49cf5ad`

Sprint entry:

- [Sprint FIX-247 — Expand File Stack-Overflow Safety Ceiling](sprints/fix-247-expand-file-stack-overflow.md)

FIX-247 caps the effective recursive include depth at 128 inside
`expand_includes`, preserving the public `IncludeDepth` API while ensuring
unreasonably deep include chains return `ERR_INCLUDE_DEPTH` instead of
overflowing the native stack.

### Follow-on Fix Sprint: FIX-251

Status:

- complete on `fix/251-io-error-collapse-not-found` at `5f4b05e`

Sprint entry:

- [Sprint FIX-251 — Distinguish Filesystem I/O Diagnostics](sprints/fix-251-io-error-collapse-not-found.md)

FIX-251 distinguishes permission-denied, directory-target, and filesystem
symlink-loop failures from genuine not-found results at include and explicit
template resolution boundaries, while preserving existing invalid-data,
confinement, and not-found behavior. The follow-up also centralizes the
filesystem classification and makes directory-target handling independent of
Windows `io::ErrorKind` mappings.

### Follow-on Fix Sprint: FIX-248

Status:

- complete on `fix/248-err-config-parse-leaks-raw-yaml` at `c65ba50`

Sprint entry:

- [Sprint FIX-248 — ERR_CONFIG_PARSE Raw YAML Leak](sprints/fix-248-err-config-parse-leaks-raw-yaml.md)

FIX-248 removes the raw `serde_yaml` source attachment from frontmatter
syntax errors on the CLI text path while preserving the stable diagnostic
message, recovery hint, JSON envelope, and all other configuration-error
source handling.

### Follow-on Fix Sprint: FIX-249

Status:

- complete on `fix/249-path-confinement-existence-oracle` at `6aa2912`

Sprint entry:

- [Sprint FIX-249 — Path-Confinement Existence Oracle](sprints/fix-249-path-confinement-existence-oracle.md)

FIX-249 makes out-of-root resolver diagnostics independent of whether the
candidate exists, while preserving the normal not-found diagnostic for
lexically in-root missing paths. The implementation also handles macOS
`/var`/`/private/var` temporary-directory aliases without weakening
confinement.

### Follow-on Fix Sprint: FIX-268

Status:

- complete on `fix/268-xml-format-autoescape-not-applied` at `19953ad`

Sprint entry:

- [Sprint FIX-268 — XML/HTML Filename-Aware Auto-Escape](sprints/fix-268-xml-format-autoescape-not-applied.md)

FIX-268 preserves the existing filename-extension auto-escape convention on
the default single-pass, multi-pass, and custom-delimiter render paths. XML
and HTML templates now escape interpolated markup while non-markup templates
and the public in-memory `render_all()` API retain their unescaped behavior.

### Follow-on Fix Sprint: FIX-269

Status:

- complete on `fix/269-json-stdout-content-loss` at `352c91f`

Sprint entry:

- [Sprint FIX-269 — JSON Render Stdout Body](sprints/fix-269-json-stdout-content-loss.md)

FIX-269 makes non-dry-run `render --json` stdout content observable through
the optional `body` payload field while preserving file-output and dry-run
payload shapes.

### Follow-on Fix Sprint: FIX-272

Status:

- complete on `fix/272-format-aware-escaping` at `0ccff88`

Sprint entry:

- [Sprint FIX-272 — Format-Aware Escaping](sprints/fix-272-format-aware-escaping.md)

FIX-272 adds filename-aware JSON escaping plus opt-in `cdata_escape` and
`turtle_escape` filters, while preserving existing HTML/XML behavior. The
plan-hardening CDATA fields now opt into safe CDATA splitting.

### Follow-on Fix Sprint: FIX-270

Status:

- complete on `fix/270-dict-get-method-unsupported` at `6e61f7c` (squash-merged
  via PR #281 to `develop`)

Sprint entry:

- [Sprint FIX-270 — Jinja Dict Get Method](sprints/fix-270-dict-get-method-unsupported.md)

FIX-270 adds a narrow project-owned Minijinja unknown-method callback for map
`.get(key[, default])` calls. Missing keys return `Undefined` or the supplied
default, while unrelated methods, value kinds, and arities retain the original
unknown-method behavior.

### Follow-on Fix Sprint: FIX-242-271

Status:

- complete on `fix/242-undeclared-token-false-positives` at `eed3369`
  (squash-merged via PR #282 to `develop` at `8992ad0`)

Sprint entry:

- [Sprint FIX-242-271 — Undeclared Token False Positives](sprints/fix-242-271-undeclared-token-false-positives.md)

FIX-242-271 removes false undeclared-token diagnostics for numeric
subscripts/slices, binary operator fragments, Jinja filter names, and simple
`{% set %}` locals while preserving real filter-argument references and
loop-context diagnostics outside active loops.

### Follow-on Fix Sprint: FIX-278

Status:

- complete on `fix/278-control-chars-survive-escape-filter` at `388c6d8`

Sprint entry:

- [Sprint FIX-278 — XML Control Character Escaping](sprints/fix-278-control-chars-survive-escape-filter.md)

FIX-278 makes the shared HTML/XML/XHTML escaping formatter XML-character-safe
for forbidden C0 controls, adds the opt-in `xml_char_safe` filter, and covers
the XHTML filename dispatch alongside explicit and implicit escaping paths.

### Follow-on Fix Sprint: FIX-274

Status:

- complete on `fix/274-spoofed-frontmatter-delimiter` at `2145245`

Sprint entry:

- [Sprint FIX-274 — Spoofed Frontmatter Delimiter](sprints/fix-274-spoofed-frontmatter-delimiter.md)

 FIX-274 adds an opt-in `frontmatter_safe` filter for interpolated values in
 frontmatter-shaped Markdown output. Standalone `---` and `...` lines are
 neutralized without changing ordinary text or mid-line delimiter sequences;
 the codex-orchestration sprint-plan template applies the filter to its title
 fields.

 ### Follow-on Fix Sprint: FIX-275

Status:

- complete on `fix/275-markdown-table-pipe-escape` at `b5d225c`

Sprint entry:

- [Sprint FIX-275 — Markdown Table Pipe Escape](sprints/fix-275-markdown-table-pipe-escape.md)

FIX-275 adds the explicit `md_table_safe` filter for Markdown table cells. It
 escapes literal pipes as `\|`, collapses embedded line breaks to spaces, and
 leaves all other characters unchanged. The filter is opt-in so ordinary
 Markdown text and existing auto-escape behavior remain unchanged.

### Follow-on Fix Sprint: FIX-273

Status:

- complete on `fix/273-array-typed-vars-accept-scalars` at `cca8486`

Sprint entry:

- [Sprint FIX-273 — Reject Scalar Input For Array-Only Required Variables](sprints/fix-273-array-typed-vars-accept-scalars.md)

FIX-273 rejects present scalar and object values when a top-level required
variable is consumed by a conservative bare-identifier for-loop, while
preserving existing dotted-path validation and non-loop behavior.

### Follow-on Fix Sprint: FIX-276

Status:

- complete on `fix/276-yaml-colon-space-unescaped` at `75e51d9`

Sprint entry:

- [Sprint FIX-276 — YAML Colon-Space Escaping](sprints/fix-276-yaml-colon-space-unescaped.md)

FIX-276 adds the explicit `yaml_safe` filter for caller-controlled YAML
mapping values. It emits a double-quoted scalar with scoped escaping for
backslashes, quotes, and line-control characters, and applies the existing
`frontmatter_safe` delimiter protection before YAML quoting in the sprint-plan
template.

### Follow-on Fix Sprint: FIX-277

Status:

- complete on `fix/277-bytes-written-off-by-one` at `07e4ca0`

Sprint entry:

- [Sprint FIX-277 — `bytes_written` Off-By-One](sprints/fix-277-bytes-written-off-by-one.md)

FIX-277 corrects JSON stdout render metadata to include the trailing newline
emitted by the equivalent plain-mode stdout path. File output and dry-run
metadata remain unchanged.

### Standalone Repowise Cleanup: Render Request Module Split

Status:

- complete

Sprint entry:

- [Render Request Module Split Cleanup](sprint-render-request-split.md)

Branch:

- `refactor/render-request-real-module` -> `develop`

This completed cleanup replaces the render-request monolith with focused
blocks, mode, request, vars, and test modules. The full workspace suite and
standard Rust validation checks pass; the refactor is ready for independent
regression QA.

### Standalone Repowise Cleanup: Publish Manifest Module Split

Status:

- complete

Sprint entry:

- [Publish Manifest Module Split Cleanup](sprint-publish-manifest-split.md)

Branch:

- `refactor/publish-manifest-real-module` -> `develop`

This completed cleanup replaces the publish-manifest monolith with focused
archive, error, files, model, report, write, and test modules. The full
workspace suite and standard Rust validation checks pass; the refactor is ready
for independent regression QA.

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
- Multi-panel HTML/XHTML report composition, wrapper-level open/app selection,
  and any reusable post-render hook design remain follow-on work documented in
  [docs/html-sprint-report-plan.md](html-sprint-report-plan.md). The shipped
  Phase HTML-Report scope is limited to the structured-input model, the
  bundled single-panel `sprint-report-html` example, and wrapper-owned HTML
  rendering integration.

### Sprint S9: User Data Directory Unification (`~/.sc-compose`)

Status:

- planned

Branch:

- `TBD`

Goals:

- unify the default examples and user-template directories under one stable
  user-owned root: `~/.sc-compose/`
- preserve `SC_COMPOSE_DATA_DIR` and `SC_COMPOSE_TEMPLATE_DIR` override
  behavior exactly as it exists today
- make installer-created directories and first-run diagnostics match the new
  unified user-data contract

Deliverables:

- `crates/sc-compose/src/template_store.rs`
  - `pub(crate) fn data_dir() -> Result<PathBuf>`
    - preserves `SC_COMPOSE_DATA_DIR`
    - changes the default fallback from install-relative
      `../share/sc-compose` to `~/.sc-compose`
  - `pub(crate) fn user_templates_dir() -> Result<PathBuf>`
    - preserves `SC_COMPOSE_TEMPLATE_DIR`
    - changes the default fallback from the platform-specific user-data root to
      `~/.sc-compose/templates`
  - `fn platform_user_data_dir() -> Option<PathBuf>`
    - is simplified or removed if it no longer owns the examples/templates
      default-resolution path
- `release/homebrew/sc-compose.rb.j2`
  - adds a `post_install` block that:
    - creates `~/.sc-compose/examples`
    - copies bundled examples from `#{share}/sc-compose/examples/` into
      `~/.sc-compose/examples/`
    - creates `~/.sc-compose/templates`
- equivalent installer/runtime planning notes for other distribution paths so
  Homebrew is not treated as the only installer owning the unified user-data
  contract
- unit and integration tests covering:
  - new default examples resolution
  - new default templates resolution
  - preserved env-override precedence
  - clear first-run error messaging when the expected user-data dirs are absent
- docs updated so the default user-facing examples/templates locations point to
  `~/.sc-compose/`

Explicit non-closure:

- no change to the meaning of `SC_COMPOSE_DATA_DIR`
- no change to the meaning of `SC_COMPOSE_TEMPLATE_DIR`
- no runtime writes into package-managed install prefixes
- no implementation of publish/reporting behavior in this sprint

Acceptance Criteria:

- `sc-compose examples` resolves bundled examples from
  `SC_COMPOSE_DATA_DIR/examples` when `SC_COMPOSE_DATA_DIR` is set
- without `SC_COMPOSE_DATA_DIR`, `sc-compose examples` resolves from
  `~/.sc-compose/examples`
- `sc-compose templates` resolves from `SC_COMPOSE_TEMPLATE_DIR` when
  `SC_COMPOSE_TEMPLATE_DIR` is set
- without `SC_COMPOSE_TEMPLATE_DIR`, `sc-compose templates` resolves from
  `~/.sc-compose/templates`
- if the unified user-data dirs are missing on first run, `sc-compose` emits a
  clear error that points users to `~/.sc-compose/`
- the Homebrew install path creates `~/.sc-compose/examples`,
  populates it from packaged bundled examples, and creates an empty
  `~/.sc-compose/templates` directory
- no runtime fallback remains on install-relative
  `../share/sc-compose/examples/` for default bundled-example discovery
- tests cover the new defaults and the preserved env overrides on macOS, Linux,
  and Windows path conventions

Required Validation:

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`

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

- completed as specified in [docs/publishing.md](publishing.md)

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

- completed

Phase goal:

- broaden `sc-compose` into a strong structured HTML report generator without
  moving wrapper-owned browser/display behavior into the core tool.

Release blocker inventory:

| ID | Blocker | Status | Sprint | Closure condition |
| --- | --- | --- | --- | --- |
| HRB-01 | The current input model cannot express structured records such as PR objects and nested field access. | Closed — PR #45, `2280bd1`. All 11 H1 acceptance tests pass including `frontmatter_defaults_accept_object_value` (`crates/sc-composer/src/lib.rs:107`), `render_accepts_object_values_in_json_var_file` (`crates/sc-compose/tests/cli.rs:818`), and `template_json_object_input_defaults_obey_precedence` (`crates/sc-compose/tests/cli.rs:581`). | H1 | Object/map input values render end-to-end with stable field-path diagnostics. |
| HRB-02 | The current input model cannot express repeated report sections as arrays of structured records. | Closed — H2 implements arrays-of-objects ingress and loop-body discovery; E.1 removes the historical nested-array restriction with recursive validation and regression coverage. | H2/E.1 | Recursive arrays and arrays of objects render, validate, and support loop-body discovery end-to-end. |
| HRB-03 | There is no bundled HTML report example proving `sc-compose` can generate a useful clickable report artifact. | Closed — H3 adds `examples/sprint-report-html.html.j2`, realistic sample vars, and named-render coverage for `sprint-report-html.html.j2 -> sprint-report-html.html`. | H3 | `sprint-report-html` renders a self-contained HTML report from realistic structured input. |

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
- scope-tracker chosen over a MiniJinja AST dependency for loop-body
  discovery; the decision is documented in `architecture.md` section 21.5
- frontmatter-init discovery for nested references inside loop bodies
- the historical H2 nested-array restriction is superseded by
  [ADR-E1](architecture.md#61-adr-e1-recursive-structured-input-contract-2026-07-29);
  the current phase index names the implementation Sprint E.1 to avoid
  colliding with the completed multi-pass Phase D identifiers
- unit and integration tests for arrays of objects

Acceptance Criteria:

- arrays of objects render end-to-end through Jinja loops
- frontmatter-init discovers loop-body variable references from array members
- recursive arrays, nested arrays of objects, and jagged arrays render through
  the E.1 recursive-value contract without emitting the reserved legacy code
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
  - an explicit safety note describing filename-aware automatic escaping and
    limiting `| safe` to trusted, pre-rendered HTML fragments
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

- `sc-compose examples sprint-report-html --var-file examples/sprint-report-html.sample-vars.json` works
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

- connect the shipped single-panel HTML example to the wrapper workflow without
  moving open/display behavior into `sc-compose`.

H4 introduces no new functional requirements. All H4 work extends FR-12,
FR-13, FR-14, and FR-15 with wrapper integration and final source-of-truth
documentation. This is intentional.

Deliverables:

- `/sprint-report` skill update that selects the shipped HTML artifact and
  writes or optionally opens it from wrapper logic
- architecture/docs update describing the wrapper-owned orchestration pattern
- explicit scoping language that multi-panel XHTML composition and any
  reusable post-render hook remain follow-on work, not H4 deliverables

Acceptance Criteria:

- `/sprint-report --html` produces the HTML report through wrapper-owned render
  orchestration
- the wrapper path opens or writes the output without requiring hook execution
  in `sc-compose`
- H4 keeps the bundled HTML artifact as the shipped single-panel example and
  does not redefine it into a multi-panel report

Exit Gate:

- wrapper integration works without changing `sc-compose` into a workflow
  orchestrator
- quality review confirms the final single-panel HTML report flow is usable and
  maintainable
- all HTML-Report phase blockers are closed
- `quality-mgr` sprint_review passes with no blocker findings

## Companion Planning Docs

- `docs/traceability-matrix.md`
- `docs/error-code-registry.md`
- `docs/test-strategy.md`
- `docs/html-sprint-report-plan.md`

## Follow-on Design Track (H5+)

The current plan is the authoritative release plan for `1.0`. Additional
post-`1.0` design exploration must not silently rewrite the shipped contract.

The current follow-on design track is:

- `docs/html-sprint-report-plan.md`
  - multi-panel HTML/XHTML sprint-report exploration beyond the shipped
    single-panel artifact,
  - wrapper-level output viewing behavior such as `--open` or application
    selection,
  - possible post-render-hook design exploration that remains outside the core
    `sc-compose` renderer boundary unless explicitly accepted in a later phase.
- `docs/phase-A/phase-A-plan.md`
  - reusable report-pack planning for multi-output bundles, shared XHTML panel
    chrome, latest/archive output policy, publish-manifest handoff, and
    `sc-observability` `1.1.0` adoption planning for the CLI logging layer
- `docs/phase-A/sprint-A6.md`
  - the sixth executable Phase A sprint, which defines latest/archive output
    policy and the shared `just reports` aggregation and verification behavior
- `docs/phase-A/sprint-A7.md`
  - the seventh executable Phase A sprint, which defines the machine-readable
    publish-manifest handoff from generated artifacts to CI or wrapper-owned
    publication steps
- `docs/phase-A/sprint-A1.md`
  - the first executable Phase A sprint, which defines the generic report
    artifact contract and report catalog before any later report-family or
    panel-specific planning work closes out
- `docs/phase-A/sprint-A3.md`
  - the third executable Phase A sprint, which defines the generic
    source-driven rendering contract for collection discovery, metadata
    extraction, render-many, and generated manifests
- `docs/phase-A/sprint-A4.md`
  - the fourth executable Phase A sprint, which defines the typed semantic
    report-spec contract so Mermaid becomes one renderer or migration input
    instead of the long-term semantic source model
- `docs/phase-A/sprint-A8.md`
  - the cross-use-case proof sprint, which demonstrates that the shared
    reporting model must serve both `atm-core` style multi-panel
    state-machine/SQL-query reports and `sc-lint` style lint/test/smoke
    evidence reports without changing the shared discovery or verification
    contract
- `docs/phase-A/sprint-A2.md`
  - the second executable Phase A sprint, which defines the standard producer
    command contract and reserves `just reports` for shared aggregation,
    verification, and opening/viewing
- `docs/phase-A/sprint-A5.md`
  - the fifth executable Phase A sprint, which defines shared template
    families, repo-local override points, and shared panel chrome with stable
    copy-action behavior
- `docs/phase-A/sprint-A9.md`
  - the observability follow-on sprint, which upgrades `sc-compose` to
    `sc-observability` `1.1.0`, keeps direct logger integration, and adopts
    logger-managed retained-log maintenance defaults

## Rule

Any follow-on sprint added after this plan must preserve the standalone
boundary defined by:

- `docs/requirements.md`
- `docs/architecture.md`
- `docs/git-workflows.md`
- `docs/publishing.md`

## Follow-on Implementation Track (Phase B)

## Fuzz-Queue Fix Sprint Index

- `docs/sprints/cleanup-298-path-containment-centralize.md`
- `docs/sprints/cleanup-299-json-integer-guard-dedup.md`
- `docs/sprints/cleanup-301-yaml-merge-key-scan.md`
- `docs/sprints/cleanup-297-bare-loop-discovery.md`
- `docs/sprints/cleanup-300-include-depth-wrapper.md`
- `docs/sprints/fix-250-varfile-object-wording-inconsistent.md`
- `docs/sprints/fix-252-varfile-missing-dir-misclassified.md`
- `docs/sprints/fix-253-doubled-delimiter-error-message.md`
- `docs/sprints/fix-254-varfile-negative-boundary-i128.md`
- `docs/sprints/fix-283-unbound-variable-policy-noop.md`

The current follow-on implementation track is:

- `docs/phase-B/phase-B-plan.md`
  - the implementation phase that turns the Phase A reporting contracts into a
    runnable shared system
- `docs/phase-B/sprint-B1.md`
  - report artifact runtime and catalog
- `docs/phase-B/sprint-B10.md`
  - built-in render context variables injected into every render context
  - added during Phase B hardening to make render-context precedence and
    template metadata injection explicit before implementation dispatch
- `docs/phase-B/sprint-B2.md`
  - producer recipes, report-init scaffold, and `just` command surface
- `docs/phase-B/sprint-B3.md`
  - source collection, metadata extraction, and render-many runtime
- `docs/phase-B/sprint-B4.md`
  - template families and shared panel chrome
- `docs/phase-B/sprint-B5.md`
  - latest/archive output policy and reports aggregator
- `docs/phase-B/sprint-B6.md`
  - publish manifest and CI handoff
- `docs/phase-B/sprint-B7.md`
  - semantic diagram-spec runtime
- `docs/phase-B/sprint-B8.md`
  - cross-use-case proof by implemented examples
- `docs/phase-B/sprint-B9.md`
  - complete: `sc-observability` `1.2` uplift landed and Phase B closeout docs/tests track the shipped surface
- `docs/sprints/b11-contract-alignment.md`
  - follow-on cleanup sprint for normative doc/API drift between Phase B
    source-of-truth docs and the shipped `sc-composer` API
- `docs/sprints/b12-json-surface-hardening.md`
  - follow-on cleanup sprint for remaining JSON and JSONL forward-slash path
    normalization plus Windows-sensitive coverage
- `docs/sprints/b13-observability-panic-removal.md`
  - follow-on cleanup sprint for removing panic paths from production
    observability code after the `sc-observability 1.2` uplift
- `docs/sprints/b14-cli-extraction.md`
  - follow-on cleanup sprint for oversized CLI module extraction and command
    ownership cleanup
- `docs/sprints/b15-reporting-runtime-cleanup.md`
  - follow-on cleanup sprint for dead reporting seams, duplicated helper
    removal, and report-runtime scope tightening
