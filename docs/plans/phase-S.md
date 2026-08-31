---
id: phase-S
title: Hotspot Reliability and Maintainability
status: complete
phase: S
branch: integrate/phase-s
owner_team: sc-compose
planning_branch: plan/phase-S-hotspot-remediation
planning_worktree: ../sc-compose-worktrees/plan/phase-S-hotspot-remediation
base: develop
related_issue: https://github.com/randlee/sc-compose/issues/572
---

# Phase S — Hotspot Reliability and Maintainability

## Plan status

Phase S is complete at `integrate/phase-s` commit
[`44a91bf`](https://github.com/randlee/sc-compose/pull/594/commits/44a91bfe3b25983eb62667e8ccce364460c43c8f).
It was a bounded, behavior-preserving maintenance phase derived from issue
#572 and the repowise dataset on `origin/sc/repowise-update2`. It improved
private seams, testability, and platform reliability without changing public
rendering, CLI, Beads, Python, or release contracts.

S.10 completed upstream in
[sc-publish PR #80](https://github.com/randlee/sc-publish/pull/80), merged at
`8d9d6790f2cad0a446758df5dcd4e2a5a9124ef9`; S.11 completed in
[sc-compose PR #594](https://github.com/randlee/sc-compose/pull/594), merged
at `44a91bfe3b25983eb62667e8ccce364460c43c8f`.

Issue #572 refers to `sprint-plan-guidelines.md`; no such file exists here.
This plan deliberately uses the canonical
`.claude/skills/codex-orchestration/sprint-plan.md.j2` template instead. Each
sprint below contains its required frontmatter schema and is the authoritative
implementation contract.

## Review method and limits

The repowise scores identify inspection candidates, not automatically approved
refactors. The review checked the issue's Top 10 against the exact code and
the boundary rules in `CLAUDE.md`. Generated or vendored package parity is not
ordinary duplication: removing it locally would make the release package less
portable. No Phase S sprint changes public types, stable diagnostic codes,
serialized receipt shapes, CLI argument grammar, release manifests, or
dependency direction.

S-T6, S-T8, S-T9, and S-T10 are deliberately sourced from the broader
repowise dataset rather than issue #572's named Top 10. Their source evidence
and rejected alternatives are recorded separately below; they do not replace
or reinterpret any Top-10 verdict.

## Critical review of issue #572 Top 10

| # | Hotspot | Verdict | Code-grounded rationale |
| --- | --- | --- | --- |
| 1 | `crates/sc-composer/src/extract/yaml.rs` | Modify | Sound target, but not a whole-file rewrite: `extract_yaml` starts the parse/match pipeline at [yaml.rs:250](../../crates/sc-composer/src/extract/yaml.rs#L250), while private parser/limit/error helpers accumulate at [yaml.rs:422](../../crates/sc-composer/src/extract/yaml.rs#L422) and [yaml.rs:634](../../crates/sc-composer/src/extract/yaml.rs#L634). Extract only private shared error/limit helpers and preserve every format-specific code. |
| 2 | `plugins/sc-publish/.github/scripts/release_artifacts.py` | Reject for Phase S | It is byte-identical to the installed root copy and is a vendorable package asset; its 45–853 command surface is release tooling, not a `sc-compose` core hotspot. A local de-duplication would violate the package's exact-copy portability contract. Ownership is `sc-publish`. |
| 3 | `crates/sc-compose/src/commands/template_lint.rs` | Modify | Sound target if split by private concern: source scanning is [template_lint.rs:55](../../crates/sc-compose/src/commands/template_lint.rs#L55), repository aggregation is [template_lint.rs:177](../../crates/sc-compose/src/commands/template_lint.rs#L177), and traversal is [template_lint.rs:305](../../crates/sc-compose/src/commands/template_lint.rs#L305). Do not introduce a second parser or alter lint codes. |
| 4 | `.github/scripts/release_artifacts.py` | Reject for Phase S | This is the same byte-identical vendored release asset as #2. The score measures maintained parity, not independently owned duplication. Phase S records ownership and leaves it unchanged. |
| 5 | `crates/sc-compose/src/commands/compose.rs` | Modify | Sound local extraction: `run_checked_validate` combines composition, report classification, presentation, and exit choice in [compose.rs:182](../../crates/sc-compose/src/commands/compose.rs#L182). Extract private report construction/classification; keep `sc-composer` calls and JSON envelope behavior unchanged. |
| 6 | `crates/sc-compose/tests/repo_boundaries.rs` | Modify | Sound test-only target: one test owns discovery plus all policy checks at [repo_boundaries.rs:185](../../crates/sc-compose/tests/repo_boundaries.rs#L185). Split into named rule-family helpers/tests without weakening any negative check. |
| 7 | `crates/sc-composer/src/extract/xml.rs` | Modify | Sound only as a narrow helper extraction. `map_raw_text_error` duplicates Request/Occurrence cases at [xml.rs:251](../../crates/sc-composer/src/extract/xml.rs#L251); the dataset's proposed four-file split is rejected because it needlessly moves cohesive private extraction behavior. |
| 8 | `.github/scripts/tests/test_release_artifacts.py` | Reject for Phase S | It is byte-identical to the plugin test copy and validates vendorable release assets (fixture helpers begin at [test_release_artifacts.py:19](../../.github/scripts/tests/test_release_artifacts.py#L19)). Consolidation belongs in `sc-publish` only if its installed-package test contract is redesigned there. |
| 9 | `plugins/sc-publish/.github/scripts/tests/test_release_artifacts.py` | Reject for Phase S | Same reasoning as #8: it is an intentional exact-copy test asset, not a separate behavior implementation. No local change is safe or useful. |
| 10 | `crates/sc-composer/src/extract/json.rs` | Modify | Sound target, constrained to private parsing/limit/error seams: `extract_json` starts at [json.rs:60](../../crates/sc-composer/src/extract/json.rs#L60), limit checks at [json.rs:229](../../crates/sc-composer/src/extract/json.rs#L229), and raw-text mapping at [json.rs:351](../../crates/sc-composer/src/extract/json.rs#L351). It must remain JSON-specific at the public diagnostic boundary. |

## Broader repowise dataset targets

| Target | Signal and code anchor | Bounded decision and rejected alternative |
| --- | --- | --- |
| S-T6 `crates/sc-compose/src/cli/capability.rs` | Score 5.25, high `complex_method`: `command_wants_json` has CCN 22 and begins at [capability.rs:3](../../crates/sc-compose/src/cli/capability.rs#L3). | Refactor only private dispatch helpers and add an exhaustive matrix. Reject a public trait or clap redesign: neither is needed to reduce the nested match. |
| S-T8 `crates/sc-composer-beads/src/runner.rs` | Score 4.42, critical `prior_defect`: three bug fixes in roughly six months; the containment/capture lifecycle begins at [runner.rs:78](../../crates/sc-composer-beads/src/runner.rs#L78). | Partition existing private lifecycle handling and prove supported-platform containment. Reject a new process library or Beads source/database integration: both cross Rule 11 boundaries. |
| S-T9 `crates/sc-composer/src/diagnostics.rs` | Score 4.5, critical `untested_hotspot`: 37 dependents; the schema constant and facade exports are at [diagnostics.rs:9](../../crates/sc-composer/src/diagnostics.rs#L9). | Freeze the existing facade with tests only. Reject a new diagnostic type, export, or schema revision: the target is contract confidence, not API redesign. |
| S-T10 `crates/sc-compose/src/path_utils.rs` | Score 6.0, critical `untested_hotspot`: 22 dependents; normalization helpers start at [path_utils.rs:41](../../crates/sc-compose/src/path_utils.rs#L41) and [path_utils.rs:49](../../crates/sc-compose/src/path_utils.rs#L49). | Add focused regression coverage only. Reject moving path policy to `sc-composer` or changing serialized paths: CLI ownership and output remain fixed. |

## Approved Phase S targets

| ID | Target and signal | Proposed approach | Boundary-safety verdict | Sprint |
| --- | --- | --- | --- | --- |
| S-T1 | `crates/sc-composer/src/extract/yaml.rs`; 789 NLOC, CCN 21, 43% duplication in issue #572 | Extract private validation and raw-text diagnostic construction seams; retain YAML path/value semantics and stable diagnostic codes. | Rule 1: safe only as an internal pure-library refactor; no CLI, filesystem policy, adapter, or ATM dependency. | S.1 |
| S-T2 | `crates/sc-composer/src/extract/json.rs`; 618 NLOC, CCN 15, 55% duplication | Factor private limit/error helpers while keeping JSON parser, paths, recovery hints, and public report types format-specific. | Rule 1: safe as private pure-library work; no dependency-direction change. | S.1 |
| S-T3 | `crates/sc-composer/src/extract/xml.rs`; nested raw-text error mapping at lines 251–312 | Extract the repeated common error construction from `map_raw_text_error`; preserve XML path decoration and adjacent-variable special case. | Rule 1: safe only if all XML behavior remains inside `sc-composer`; reject broad file fragmentation. | S.1 |
| S-T4 | `crates/sc-compose/src/commands/template_lint.rs`; 529 NLOC, scanner/aggregation/traversal mixed | Separate private source-analysis, repository traversal, and report-assembly units; reuse the existing `sc_composer::template_scanner`. | Rule 2: safe in CLI adapter; must not move lint policy into `sc-composer` or add a second parser. | S.2 |
| S-T5 | `crates/sc-compose/src/commands/compose.rs`; `run_checked_validate` lines 182–253 mixes four concerns | Extract private checked-report construction and output/exit handling, with fixtures that freeze JSON and text output. | Rule 2: safe in CLI; `sc-composer` remains the composition/check authority. | S.3 |
| S-T6 | `crates/sc-compose/src/cli/capability.rs`; `command_wants_json` CCN 22 | Decompose nested subcommand checks into private capability helpers and exhaustive table-driven tests. | Rule 2: safe in CLI only; no clap/public-argument change. | S.4 |
| S-T7 | `crates/sc-compose/tests/repo_boundaries.rs`; one CCN 18 test combines discovery and policy | Split discovery from named invariant-family assertions and add focused regression fixtures. | Rules 1–13: safe because it strengthens enforcement only; it may not remove or relax a prohibited dependency assertion. | S.5 |
| S-T8 | `crates/sc-composer-beads/src/runner.rs`; three recent defect fixes and process-containment state at lines 78–137 | Partition private capture-state handling from child containment/status collection; add Unix and Windows contractual tests without changing request/receipt contracts. | Rule 11: safe only with existing `process-wrap` and host-neutral runner boundaries; no CLI, Beads source/database, foreign adapter, or ATM dependency. | S.8 |
| S-T9 | `crates/sc-composer/src/diagnostics.rs`; high-dependent public diagnostic facade | Add contract tests that freeze the current facade's schema-version value and public re-exports: `DiagnosticEnvelope`, `Diagnostic`, `DiagnosticCode`, and `DiagnosticSeverity`. | Rule 1: test the existing public contract only; no new types, exports, or serialized schema. | S.6 |
| S-T10 | `crates/sc-compose/src/path_utils.rs` and `crates/sc-compose/src/reporting/publish_manifest/tests.rs`; high-dependent normalization and manifest-path coverage | Add focused regression coverage for `is_normalized_relative_path` and `normalize_relative_path`, including serialization-adjacent path cases. | Rule 2: preserve CLI-owned path policy; no policy relocation or format change. | S.7 |

## Historical sprint stack and dependencies

The completed Phase S work used a strictly linear `gh stack`, ordered bottom
to top. S.10 was an explicit upstream `sc-publish` gate outside that stack;
S.11 began only after S.10 merged. The diagram and table below preserve the
executed dependency record; they are not setup instructions for new work.

```text
develop
└── integrate/phase-s              (draft phase-close PR → develop)
    └── S.1 extractor seams        (PR → integrate/phase-s)
        └── S.2 template lint      (PR → S.1)
            └── S.3 validation     (PR → S.2)
                └── S.4 capability (PR → S.3)
                    └── S.5 boundaries (PR → S.4)
                        └── S.6 diagnostics (PR → S.5)
                            └── S.7 paths (PR → S.6)
                                └── S.8 Beads runner (PR → S.7)
                                    └── S.9 go-native remediation plan (PR → S.8)
                                        ├── S.10 sc-publish peer package (external PR → sc-publish/develop)
                                        └── S.11 sc-compose adoption (PR → S.9; waits for S.10 merge)
```

| Sprint | Goal | Stack parent / PR base | Merge order |
| --- | --- | --- | --- |
| [S.1](../phase-S/sprint-s-1-extractor-internal-seams.md) | Bound private JSON/YAML/XML extraction seams. | `integrate/phase-s`; no functional dependency. | Bottom sprint layer. |
| [S.2](../phase-S/sprint-s-2-template-lint-seams.md) | Split private template-lint source analysis, traversal, and report assembly. | S.1; no functional dependency. | Second layer. |
| [S.3](../phase-S/sprint-s-3-checked-validation-seams.md) | Isolate checked-validation report construction from presentation and exit choice. | S.2; no functional dependency. | Third layer. |
| [S.4](../phase-S/sprint-s-4-json-capability-seams.md) | Decompose JSON-capability dispatch and exhaustively freeze the command matrix. | S.3; no functional dependency. | Fourth layer. |
| [S.5](../phase-S/sprint-s-5-boundary-invariant-guardrails.md) | Split boundary discovery and invariant-family assertions. | S.4; no functional dependency. | Fifth layer. |
| [S.6](../phase-S/sprint-s-6-diagnostics-facade-contract.md) | Freeze the existing diagnostics facade and envelope defaults. | S.5; no functional dependency. | Sixth layer. |
| [S.7](../phase-S/sprint-s-7-path-normalization-contract.md) | Freeze CLI-owned path normalization and serialization-adjacent coverage. | S.6; no functional dependency. | Seventh layer. |
| [S.8](../phase-S/sprint-s-8-beads-runner-reliability.md) | Isolate the cross-platform output-capture lifecycle and prove containment. | S.7; no functional dependency. | Eighth layer. |
| [S.9](../phase-S/sprint-s-9-go-native-module-remediation-plan.md) | Planning-only: draft a remediation plan for the `stage-go-native-module` gap (issue #583). No production/CI file changes. | S.8; no functional dependency. | Ninth layer. |
| [S.10](../phase-S/sprint-s-10-sc-publish-go-native-module-package.md) | Create and validate the reusable `go-native-module` peer package in `sc-publish`. | S.9 plan approval and Accepted ADR-0022; implemented and reviewed in `sc-publish`, targeting `sc-publish/develop`. | External upstream gate; not a branch in this repository's `gh stack`. |
| [S.11](../phase-S/sprint-s-11-sc-compose-go-native-module-adoption.md) | Install the approved S.10 package and restore `sc-sha-go` bundle verification in this repository. | S.9 as its documentation parent and merged S.10 package as its functional dependency. | Final `sc-compose` layer after S.10. |

The `sc-compose` stack parent is a delivery dependency, not permission to mix
concerns: each sprint keeps its own exact targets and validation. A reviewer
can review each local layer independently because its PR base is its immediate
predecessor. S.10 is a cross-repository functional gate, not a local stack
parent: its approval and merge SHA are inputs to S.11.

## Completion record

All S.1–S.9 and S.11 changes are merged into `integrate/phase-s` at `44a91bf`.
S.10 is merged upstream at `8d9d6790f2cad0a446758df5dcd4e2a5a9124ef9`.
The phase-ending review confirmed the merged code, contracts, and CI evidence.
Any future phase must create its own integration and stack commands under the
[Phase Integration Rule](../git-workflows.md#phase-integration-rule).

## Non-goals and ADR gate

Phase S does not change architecture, add dependencies, move vendored
`sc-publish` assets, alter stable external contracts, or introduce a new
public abstraction. A proposed change that would require any reverse
dependency, adapter crossing, Beads runtime integration, or release-package
ownership change is out of scope and must begin with an ADR and a separately
approved phase.

## Phase acceptance criteria

- [x] Every S-T1 through S-T10 target preserves its existing public behavior
  through focused regression tests and the full workspace suite.
- [x] Every extractor diagnostic code, recovery hint, byte span, and path
  remains stable on the committed JSON/YAML/XML corpus.
- [x] CLI text/JSON result shapes and exit statuses remain stable for lint and
  checked validation.
- [x] Boundary checks are at least as strict as the pre-phase baseline.
- [x] S-T9 freezes the existing diagnostics facade: schema version `"1"` and
  the existing `DiagnosticEnvelope`, `Diagnostic`, `DiagnosticCode`, and
  `DiagnosticSeverity` public exports.
- [x] S-T10 covers the existing normalized-relative-path contract without
  moving CLI path policy or changing serialized path output.
- [x] Runner tests prove bounded output and containment behavior on supported
  Unix and Windows implementations without a shell.
- [x] Every sc-compose S.1–S.9 and S.11 PR passes its own CI and review as a
  linear stack layer; S.10 is merged and recorded from `sc-publish/develop`;
  the required phase-ending review passes before the draft integration PR and
  its descendants merge atomically into `develop`.
- [x] No Phase S change violates any `CLAUDE.md` Boundary Rule; any exception
  is stopped for ADR review rather than implemented.

## Phase-close validation

```text
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
python3 -m pytest -q bindings/sc-composer-beads-python/tests
just lint
git diff --check
```
