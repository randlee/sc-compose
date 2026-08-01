---
id: F.3
title: CLI Integration Test Decomposition
status: planned
branch: sprint/f-3-integration-test-decomposition
worktree: /Users/randlee/Documents/github/sc-compose-worktrees/sprint/f-3-integration-test-decomposition
target: develop
---

# Sprint F.3 — CLI Integration Test Decomposition

## Goal

- Reduce the extreme duplication, churn, and defect cost of the sc-compose CLI integration suites by extracting a shared test harness and decomposing cli.rs/json_cli.rs by capability while retaining format-specific assertions. This is a bounded test-maintainability sprint with no production behavior change.
## Hard Dependencies

- None. The existing crates/sc-compose/tests/support/mod.rs, cli.rs, json_cli.rs, and current integration behavior are the baseline.
- The test harness must remain test-only under crates/sc-compose/tests/ and must not become a production dependency or alter the sc-composer/sc-compose crate boundary.
## Exact Targets

- `crates/sc-compose/tests/cli.rs`
- `crates/sc-compose/tests/json_cli.rs`
- `crates/sc-compose/tests/support/mod.rs`
- `crates/sc-compose/tests/repo_boundaries.rs`
- `crates/sc-compose/Cargo.toml`
## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- F3-D1 — Decompose the monolithic text and JSON suites into capability-oriented test modules or equivalent clearly owned sections covering render/validate/verify, reports, templates, and observability.
- F3-D2 — Centralize repeated command invocation, temporary-root lifecycle, fixture writers, path normalization, JSON envelope/diagnostic assertions, and repeated status/stdout checks in the existing test support module or focused support submodules.
- F3-D3 — Retain format-specific assertions: text tests continue to verify human output and stderr, JSON tests continue to verify envelope shape, payload, diagnostics, exit codes, and stdout cleanliness.
- F3-D4 — Add a small text/JSON equivalence matrix for commands whose semantics are shared, proving matching exit codes and diagnostics without collapsing distinct output contracts.
- F3-D5 — Remove only duplication made redundant by the shared harness; preserve all meaningful regression scenarios, including recursive inputs, report pipelines, observability health, confinement, and failure diagnostics.
- F3-D6 — Rendered plan evidence: `sc-compose render --file .claude/skills/codex-orchestration/sprint-plan.md.j2 --var-file var-files/phase-f-3.json --output docs/phase-F/sprint-f-3-integration-test-decomposition.md` exits 0.
## Required Work

- Inventory existing test functions and group them by capability before moving code; every moved test must retain its original behavioral assertions.
- Design helpers around stable user workflows rather than production implementation details so future command refactors do not require rewriting every test.
- Keep text-vs-JSON concerns explicit and avoid a helper that silently accepts either output format; shared helpers may cover mechanics, not format semantics.
- Run the full workspace suite during the refactor and compare test counts or named coverage to detect accidental deletion.
- This is an L-sized but bounded test-only refactor. It is considered single-sprint feasible only if no runtime behavior or fixture contract is redesigned; if the implementation exposes a need for runtime changes, stop and split rather than silently carrying that work forward.
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
## This Sprint Does Not Close

- This sprint does not change sc-compose runtime code, CLI behavior, diagnostics, JSON schemas, report formats, or observability contracts.
- This sprint does not move support helpers into sc-compose/src or sc-composer; no production crate may depend on test support.
- This sprint does not claim new product coverage merely because tests moved; every retained scenario must continue to execute and assert its existing behavior.
## Acceptance Criteria

- The two suites no longer duplicate the shared command/fixture/assertion mechanics identified by Repowise, and the worst cross-file clone is removed or materially reduced.
- All existing CLI and JSON CLI tests remain present in an equivalent capability module and pass with their meaningful assertions intact.
- The text/JSON equivalence matrix covers representative success and failure commands and proves matching exit/diagnostic semantics while preserving distinct output assertions.
- The support module remains test-only and no dependency or import violates repo boundaries.
- Full workspace validation passes, and the refactor does not rely on weakening or deleting a failing test.
## Required Validation

- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p sc-compose --test cli`
- `cargo test -p sc-compose --test json_cli`
- `git diff --check`