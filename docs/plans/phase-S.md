---
id: phase-S
title: Hotspot Reliability and Maintainability
status: planned
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

Planning is complete; Phase S implementation is planned. It is a bounded,
behavior-preserving maintenance phase derived from issue #572 and the repowise dataset on
`origin/sc/repowise-update2`. It improves private seams, testability, and
platform reliability without changing public rendering, CLI, Beads, Python, or
release contracts.

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

## Sprint stack and dependencies

Phase S is one strictly linear `gh stack`, ordered bottom to top. This is
deliberate: each PR contains only its sprint's incremental diff and runs CI
once for that increment, instead of repeatedly merging every independently
rooted sprint into the integration branch and re-running CI on every growing
combination.

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
                                    └── S.9 go-native plan (PR → S.8)
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
| [S.9](../phase-S/sprint-s-9-go-native-module-remediation-plan.md) | Planning-only: draft a remediation plan for the `stage-go-native-module` gap (issue #583). No production/CI file changes. | S.8; no functional dependency. | Top and final sprint layer. |

The stack parent is a delivery dependency, not permission to mix concerns:
each sprint keeps its own exact targets and validation. A reviewer can review
each layer independently because its PR base is its immediate predecessor.

## Phase integration setup and close

This phase plan is the sole owner of stack setup and phase close. Sprint docs
contain only their own layer's commit/submit commands. This follows the
[Phase Integration Rule](../git-workflows.md#phase-integration-rule)'s explicit
linear-stack mode: create one integration root, add each child at the top of
the same stack, and merge the reviewed stack once after the phase-ending
review.

```bash
# One-time setup before S.1. This is the only `gh stack init` in Phase S.
git switch develop
git pull --ff-only origin develop
git config rerere.enabled true
git config remote.pushDefault origin
gh stack init --base develop integrate/phase-s sprint/s-1-extractor-internal-seams

# After committing the current top sprint and before beginning the next sprint,
# add exactly one child from that top branch, then work and submit it as the
# next incremental layer. Do not pre-create independent sprint roots.
gh stack top
gh stack add sprint/s-2-template-lint-seams
# Repeat, in order, for S.3 through S.9 with the branch names in the table.

# If develop advances during the phase, update all layers together:
gh stack sync

# `gh stack submit --auto` creates the integration PR as a draft plus the
# sprint PRs with their immediate-parent bases. Mark only reviewed sprint PRs
# ready; leave the integration PR draft until phase close.

# Phase close: all S.1–S.9 PRs must be open, reviewed, and CI-green.
# REQUIRED GATE: phase-ending review per docs/git-workflows.md Phase Integration
# Rule step 5 must PASS before making the draft integration PR ready.
gh stack view --json
gh pr ready <phase-s-integration-pr-number>
gh stack merge <sprint-s-9-pr-number> --yes --merge
```

No sprint doc may run `gh stack init` or `gh stack merge`. The final command
merges the integration PR and every reviewed sprint layer bottom-to-top as one
atomic stack operation; it is the only Phase S path to `develop`.

## Non-goals and ADR gate

Phase S does not change architecture, add dependencies, move vendored
`sc-publish` assets, alter stable external contracts, or introduce a new
public abstraction. A proposed change that would require any reverse
dependency, adapter crossing, Beads runtime integration, or release-package
ownership change is out of scope and must begin with an ADR and a separately
approved phase.

## Phase acceptance criteria

- [ ] Every S-T1 through S-T10 target preserves its existing public behavior
  through focused regression tests and the full workspace suite.
- [ ] Every extractor diagnostic code, recovery hint, byte span, and path
  remains stable on the committed JSON/YAML/XML corpus.
- [ ] CLI text/JSON result shapes and exit statuses remain stable for lint and
  checked validation.
- [ ] Boundary checks are at least as strict as the pre-phase baseline.
- [ ] S-T9 freezes the existing diagnostics facade: schema version `"1"` and
  the existing `DiagnosticEnvelope`, `Diagnostic`, `DiagnosticCode`, and
  `DiagnosticSeverity` public exports.
- [ ] S-T10 covers the existing normalized-relative-path contract without
  moving CLI path policy or changing serialized path output.
- [ ] Runner tests prove bounded output and containment behavior on supported
  Unix and Windows implementations without a shell.
- [ ] Every S.1–S.9 PR passes its own CI and review as a linear stack layer;
  the required phase-ending review passes before the draft integration PR and
  its descendants merge atomically into `develop`.
- [ ] No Phase S change violates any `CLAUDE.md` Boundary Rule; any exception
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
