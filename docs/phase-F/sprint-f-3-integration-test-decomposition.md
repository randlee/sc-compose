---
id: F.3
title: CLI Integration Test Decomposition
status: complete
branch: sprint/f-3-integration-test-decomposition
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-3-integration-test-decomposition
target: integrate/phase-f
---

# Sprint F.3 — CLI Integration Test Decomposition

## Goal

- Reduce the extreme duplication, churn, and defect cost of the sc-compose CLI integration suites by extracting a shared test harness and decomposing cli.rs/json_cli.rs by capability while retaining every existing format-specific assertion. This sprint is strictly a test-harness/decomposition sprint: it relocates and deduplicates existing tests without adding new behavioral coverage or changing production code.
## Hard Dependencies

- F.1, F.2, F.4, and F.5 must all merge into `integrate/phase-f` in that order before F.3 starts or rebases. F.3 is last in the Phase F test-file sequence: F.1 -> F.2 -> F.4 -> F.5 -> F.3. It decomposes the already-updated suites only after the other four sprints' test additions are present; the integration branch is merged to `develop` once Phase F closes.
- The existing crates/sc-compose/tests/support/mod.rs, cli.rs, json_cli.rs, and current integration behavior are the baseline.
- The test harness must remain test-only under crates/sc-compose/tests/ and must not become a production dependency or alter the sc-composer/sc-compose crate boundary.
## Exact Targets

- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `crates/sc-compose/tests/support/mod.rs`
- `crates/sc-compose/tests/repo_boundaries.rs`
- `crates/sc-compose/Cargo.toml`
- `crates/sc-compose/tests/cli/render.rs` — post-decomposition text render/validate/verify module
- `crates/sc-compose/tests/cli/reports.rs` — post-decomposition report-pipeline module
- `crates/sc-compose/tests/cli/templates.rs` — post-decomposition examples/templates/init module
- `crates/sc-compose/tests/cli/observability.rs` — post-decomposition text observability module
- `crates/sc-compose/tests/json_cli/render.rs` — post-decomposition JSON render/validate/resolve module
- `crates/sc-compose/tests/json_cli/reports.rs` — post-decomposition JSON report-pipeline module
- `crates/sc-compose/tests/json_cli/templates.rs` — post-decomposition JSON examples/templates/init module
- `crates/sc-compose/tests/json_cli/observability.rs` — post-decomposition JSON observability module

### Cross-Cutting Placement Rule

- Route cross-cutting smoke tests through the observability modules because logger health is their common capability: `reports_smoke_keeps_observability_health_green_under_logger_12` from both `crates/sc-compose/tests/cli.rs` and `crates/sc-compose/tests/json_cli.rs` moves to `crates/sc-compose/tests/cli/observability.rs` and `crates/sc-compose/tests/json_cli/observability.rs`, respectively; `release_smoke_covers_render_pipeline_and_observability_health` from `crates/sc-compose/tests/cli.rs` moves to `crates/sc-compose/tests/cli/observability.rs`. These tests use shared `tests/support` fixtures and must not be duplicated into render or reports modules.
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- F3-D1 — Decompose the monolithic text and JSON suites into capability-oriented test modules or equivalent clearly owned sections covering render/validate/verify, reports, templates, and observability.
- F3-D2 — Centralize repeated command invocation, temporary-root lifecycle, fixture writers, path normalization, JSON envelope/diagnostic assertions, and repeated status/stdout checks in the existing test support module or focused support submodules.
- F3-D3 — Retain format-specific assertions: text tests continue to verify human output and stderr, JSON tests continue to verify envelope shape, payload, diagnostics, exit codes, and stdout cleanliness.
- F3-D4 — Preserve the existing text and JSON behavioral scenarios while moving them into the concrete module layout above; do not add a new equivalence-matrix test family in this sprint.
- F3-D5 — Remove only duplication made redundant by the shared harness; preserve all meaningful regression scenarios, including recursive inputs, report pipelines, observability health, confinement, and failure diagnostics.
- F3-D6 — Plan artifact provenance is recorded honestly: the document was authored/edited outside the templated pipeline because `sc-compose validate --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --json` reproducibly returns exit 3 while parsing the canonical template's nested Jinja frontmatter. The tooling defect is tracked as unnumbered Phase F follow-on work in `docs/project-plan.md`; this sprint does not claim templated-render evidence.
## Required Work

- Inventory existing test functions and group them by capability before moving code; every moved test must retain its original behavioral assertions.
- Design helpers around stable user workflows rather than production implementation details so future command refactors do not require rewriting every test.
- Keep text-vs-JSON concerns explicit and avoid a helper that silently accepts either output format; shared helpers may cover mechanics, not format semantics.
- Run the full workspace suite during the refactor and compare test counts or named coverage to detect accidental deletion.
- Closed scope inventory: the original baseline contained 127 `fn` declarations in `cli.rs` and 55 in `json_cli.rs`; the F.4/F.4b QA fixes added five CLI and one JSON authored regressions before this sprint, so the final pre-move inventory was 132 and 56 (188 declarations). Every one is eligible only for relocation, import rewiring, or replacement of duplicated test mechanics; no test function may change production behavior, fixture contents, or expected product semantics. Capture the inventory with `rg -n '^fn ' crates/sc-compose/tests/cli.rs crates/sc-compose/tests/json_cli.rs` before the move and reconcile it after the move against the two entrypoints plus `crates/sc-compose/tests/cli/**/*.rs` and `crates/sc-compose/tests/json_cli/**/*.rs`, accounting explicitly for shared helpers moved to `tests/support`.
- The sprint closes when that relocation/de-duplication is complete. New text/JSON equivalence-matrix authorship is explicitly outside this sprint and is listed as unnumbered follow-on work in `docs/project-plan.md`.
## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
// crates/sc-compose/tests/support/mod.rs
pub fn sc_compose(label: &str) -> Command;
pub fn temp_root(label: &str, namespace: &str) -> PathBuf;
pub fn write_file(path: &Path, contents: &str);
pub fn assert_json_envelope(value: &Value);
pub fn assert_diagnostic_code(value: &Value, code: &str);
```
The support API is illustrative: helpers must stay test-only, own no production state, and leave text assertions and JSON payload assertions in their respective test modules.

## Relocation Evidence

- Pre-move inventory after the final F.4/F.4b/F.5b merge-forward: 132 declarations in `cli.rs` and 56 in `json_cli.rs`, for 188 total. The original 127/55 baseline is preserved in the task history; the six additional declarations are the authored F.4/F.4b regressions.
- Post-move inventory: 121 text test declarations across `cli/{render,reports,templates,observability}.rs`, 52 JSON test declarations across `json_cli/{render,reports,templates,observability}.rs`, and the retained text-only `sprint_report_html_sample_vars` fixture helper. The 14 duplicated mechanics/helpers formerly declared in the two entrypoints are centralized in `tests/support/mod.rs` (including the two platform-specific symlink declarations), reconciling all 188 declarations with zero unexplained loss.
- The cross-cutting smoke tests are in the observability modules as required: `reports_smoke_keeps_observability_health_green_under_logger_12` appears in both observability modules, and `release_smoke_covers_render_pipeline_and_observability_health` appears in the text observability module.
## This Sprint Does Not Close

- This sprint does not change sc-compose runtime code, CLI behavior, diagnostics, JSON schemas, report formats, or observability contracts.
- This sprint does not move support helpers into sc-compose/src or sc-composer; no production crate may depend on test support.
- This sprint does not claim new product coverage merely because tests moved; every retained scenario must continue to execute and assert its existing behavior.
## Acceptance Criteria

- The two suites no longer duplicate the shared command/fixture/assertion mechanics identified by Repowise, and the worst cross-file clone is removed or materially reduced.
- All existing CLI and JSON CLI tests remain present in an equivalent capability module and pass with their meaningful assertions intact.
- Existing text and JSON success/failure scenarios remain present in their respective modules with their current exit, diagnostic, and output assertions; a new equivalence matrix is not an F.3 acceptance gate.
- The support module remains test-only and no dependency or import violates repo boundaries.
- Full workspace validation passes, and the refactor does not rely on weakening or deleting a failing test.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test cli`
- `cargo test -p sc-compose --test json_cli`
- `git diff --check`
