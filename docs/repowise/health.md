# sc-compose — Repowise Code Health

**Version:** release-candidate-v1.6.1-10-gb763d2ff | **Commit:** b763d2ff | **Generated:** 2026-08-31
**Analyzed:** 207 files | **Biomarker findings:** 1488 | **Data:** repowise health + dead-code + refactoring-targets

![health 8.32/10](https://img.shields.io/badge/health-83%2F100-brightgreen)

## Quick Summary

| Metric | Value |
|---|---|
| Overall Health | **8.32/10** |
| Hotspot Health | 5.21/10 |
| Worst In-Scope File | `.github/scripts/release_artifacts.py` (1.90/10) |
| Files Scored (this scope) | 207 |
| Files in Full Index | 1017 (repo-wide scan; scored table above = configured modules only) |
| Maintainability | 9.09 avg / 7.45 hotspot |
| Performance | 9.77 avg / 9.24 hotspot |

**Read:** in-scope average 8.32/10 with hotspot 5.21/10 — 1 of 207 scored file sit below 3.0/10 and carry the hotspot drag.

## Trend Between Runs

**This run vs the previous recorded run: +0.06/10 overall health — improved.** Full log below.

Latest run is last. Chronological; read with `health.md` for the current deep-dive.

| Generated | Version / Commit | Overall | Hotspot | Files | Worst file (score) | PR |
|---|---|---|---|---|---|---|
| 2026-08-02 | v1.2.0-219-gbb79a5f / bb79a5f (develop baseline) | **7.90/10** | 4.70/10 | 286 | `crates/sc-compose/src/cli.rs` (1.55) | n/a (pre-PR runs) |
| 2026-08-29 | release-candidate-v1.6.0-1-g0d70f0c3 / 0d70f0c3 | **8.23/10** | 5.20/10 | 988 | `crates/sc-composer/src/extract/yaml.rs` (2.54) | [#571](https://github.com/randlee/sc-compose/pull/571) |
| 2026-08-29 | v1.6.0-15-g54fff289 / 54fff289 | **8.26/10** | 5.15/10 | 999 | `crates/sc-composer/src/extract/yaml.rs` (2.54) | [#571](https://github.com/randlee/sc-compose/pull/571) |
| 2026-08-31 | release-candidate-v1.6.1-10-gb763d2ff / b763d2ff | **8.32/10** | 5.21/10 | 1017 | `.github/scripts/release_artifacts.py` (1.90) | [#571](https://github.com/randlee/sc-compose/pull/571) |

*History seeded + appended by `.sc/repowise/generate-report.py`; hand-edit only to correct a wrong row.*
## Worst 20 Files by Health Score

| Score | File | NLOC | CCN | Nest | Dup% |
|---|---|---|---|---|---|
| 2.58 | `crates/sc-compose/src/commands/template_lint.rs` | 576 | 9 | 4 | 11.7 |
| 4.21 | `crates/sc-compose/tests/repo_boundaries.rs` | 393 | 10 | 4 | — |
| 4.44 | `crates/sc-composer/src/extract/xml_model.rs` | 227 | 13 | 4 | — |
| 4.44 | `crates/sc-composer/src/resolver.rs` | 714 | 7 | 4 | 30.3 |
| 4.50 | `crates/sc-composer/src/diagnostics.rs` | 9 | 1 | 0 | — |
| 4.65 | `crates/sc-compose/src/commands/template_init.rs` | 685 | 10 | 2 | 14.6 |
| 4.65 | `crates/sc-composer-beads/src/execute.rs` | 936 | 16 | 2 | 20.7 |
| 4.70 | `crates/sc-composer/src/extract/yaml.rs` | 706 | 21 | 6 | 48.8 |
| 4.71 | `crates/sc-composer/src/discovery.rs` | 271 | 9 | 4 | 3.9 |
| 4.75 | `crates/sc-composer-beads/src/runner.rs` | 406 | 9 | 3 | 3.7 |
| 4.77 | `crates/sc-composer/src/validation/diagnostics.rs` | 1102 | 8 | 3 | 29.2 |
| 4.78 | `crates/sc-composer/src/composer.rs` | 690 | 8 | 2 | 39.3 |
| 4.93 | `crates/sc-composer/src/init_workspace.rs` | 221 | 13 | 3 | 15.9 |
| 5.05 | `crates/sc-composer/src/renderer.rs` | 1079 | 7 | 3 | 16.4 |
| 5.12 | `crates/sc-compose/tests/support/mod.rs` | 534 | 5 | 3 | 7.0 |
| 5.13 | `crates/sc-compose/src/commands/compose.rs` | 427 | 11 | 3 | — |
| 5.31 | `crates/sc-compose/src/commands/sc_lint.rs` | 506 | 12 | 3 | — |
| 5.32 | `crates/sc-composer/src/extract/toml.rs` | 509 | 18 | 4 | 49.9 |
| 5.33 | `crates/sc-compose/src/observer_impl.rs` | 967 | 8 | 2 | 17.4 |
| 5.40 | `crates/sc-compose/tests/cli/extract.rs` | 396 | 3 | 1 | 64.7 |

**Observations**
- `crates/sc-composer/src/extract/yaml.rs` (4.7/10, 706 NLOC) — high cyclomatic complexity, CCN=21, 48.81% duplication
- `crates/sc-compose/tests/cli/extract.rs` (5.4/10, 396 NLOC) — high duplication, 64.65% duplication
- `crates/sc-composer/src/extract/yaml.rs` (4.7/10, 706 NLOC) — deep nesting, CCN=21, 48.81% duplication

## Biomarker Findings

| Type | Count | What It Means |
|---|---|---|
| duplicated_assertion_block | 352 | Repeated assertion patterns — test helper opportunity |
| error_handling | 234 | Error handling gaps or inconsistencies |
| prior_defect | 165 | Files with bug-fix history — strong defect predictor |
| dry_violation | 157 | DRY violations — opportunities to extract shared code |
| hot_path_sync_io | 79 | Sync I/O on hot paths — should be async |
| co_change_scatter | 78 | Files that change together — high coupling |
| complex_method | 61 |  |
| change_entropy | 57 |  |
| primitive_obsession | 52 |  |
| large_method | 49 |  |
| io_in_loop | 48 |  |
| churn_risk | 38 | High recent change frequency — churn-driven risk |
| nested_complexity | 38 |  |
| function_hotspot | 32 |  |
| low_cohesion | 27 |  |
| untested_hotspot | 10 | Depended-upon files with no paired test coverage |
| large_assertion_block | 4 |  |
| complex_conditional | 3 |  |
| bumpy_road | 2 |  |
| nested_loop_with_io | 2 |  |

### Highest-Impact Findings (by type)

### [352] duplicated_assertion_block
Repeated assertion patterns — test helper opportunity
- **medium** `crates/sc-compose/tests/sc_lint_runner.rs` `(top-level)`: assertion block at lines 120-132 is duplicated in crates/sc-compose/tests/sc_lint_check_native.rs
- **medium** `crates/sc-compose/tests/cli/help.rs` `(top-level)`: assertion block at lines 124-129 is duplicated in crates/sc-compose/tests/cli/render.rs
- **medium** `crates/sc-compose/tests/sc_lint_identity_literals.rs` `(top-level)`: assertion block at lines 32-59 is duplicated in crates/sc-compose/tests/sc_lint_check_xwin.rs
- **medium** `crates/sc-compose/tests/sc_lint_identity_literals.rs` `(top-level)`: assertion block at lines 79-96 is duplicated in crates/sc-compose/tests/sc_lint_identity_literals.rs
- **medium** `crates/sc-compose/src/reporting/publish_manifest/tests.rs` `(top-level)`: assertion block at lines 29-33 is duplicated in crates/sc-compose/src/reporting/publish_manifest/tests.rs
- **medium** `crates/sc-compose/src/reporting/publish_manifest/tests.rs` `(top-level)`: assertion block at lines 70-75 is duplicated in crates/sc-compose/src/reporting/publish_manifest/tests.rs

### [234] error_handling
Error handling gaps or inconsistencies
- **low** `crates/sc-compose/src/main_tests.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-compose/src/main_tests.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-compose/src/main_tests.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-compose/src/var_file_yaml.rs` `(top-level)`: panic!/unreachable!/todo!/unimplemented! aborts the process unconditionally
- **low** `crates/sc-compose/src/commands/sc_lint.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-compose/src/commands/sc_lint.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash

### [165] prior_defect
Files with bug-fix history — strong defect predictor
- **critical** `crates/sc-compose/src/path_utils.rs` `(top-level)`: 17 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/reporting/catalog.rs` `(top-level)`: 12 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/reporting/index.rs` `(top-level)`: 5 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/reporting/init.rs` `(top-level)`: 20 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/reporting/output.rs` `(top-level)`: 6 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/reporting/render_many.rs` `(top-level)`: 20 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects

### [157] dry_violation
DRY violations — opportunities to extract shared code
- **high** `crates/sc-compose/src/commands/examples.rs` `(top-level)`: 34% of file duplicated; worst clone shares 13 lines with crates/sc-compose/src/commands/templates.rs (co-changed 3x)
- **high** `crates/sc-compose/src/reporting/catalog.rs` `(top-level)`: 27% of file duplicated; worst clone shares 20 lines with crates/sc-compose/src/reporting/templates.rs (co-changed 6x)
- **high** `crates/sc-compose/src/reporting/init.rs` `(top-level)`: 39% of file duplicated; worst clone shares 14 lines with crates/sc-compose/src/reporting/output.rs (co-changed 6x)
- **high** `crates/sc-compose/src/reporting/spec.rs` `(top-level)`: 29% of file duplicated; worst clone shares 12 lines with crates/sc-compose/src/reporting/init.rs (co-changed 4x)
- **high** `crates/sc-compose/tests/sc_lint_ci.rs` `(top-level)`: 56% of file duplicated; worst clone shares 32 lines with crates/sc-compose/tests/sc_lint_lint_full.rs (co-changed 3x)
- **high** `crates/sc-compose/tests/sc_lint_lint_full.rs` `(top-level)`: 49% of file duplicated; worst clone shares 32 lines with crates/sc-compose/tests/sc_lint_ci.rs (co-changed 3x)

### [79] hot_path_sync_io
Sync I/O on hot paths — should be async
- **low** `crates/sc-compose/src/main_tests.rs` `temp_root`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-compose/src/observer_impl.rs` `temp_root`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-compose/src/observer_impl.rs` `read_log_lines`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-compose/src/var_file.rs` `load_var_file`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-compose/src/commands/template_init.rs` `template_init_file`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-compose/src/commands/template_init.rs` `temp_root`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function

### [78] co_change_scatter
Files that change together — high coupling
- **high** `crates/sc-compose/src/template_store.rs` `(top-level)`: co-changes with 17 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-compose/src/commands/extract.rs` `(top-level)`: co-changes with 18 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-compose/tests/sc_lint_sc_boundary.rs` `(top-level)`: co-changes with 15 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/diagnostics/schema.rs` `(top-level)`: co-changes with 22 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/diagnostics/schema.rs` `(top-level)`: co-changes with 22 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/init_workspace.rs` `(top-level)`: co-changes with 25 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)

## Dead Code / Unreachable

| Kind | Count |
|---|---|
| unused_export | 143 |
| unreachable_file | 33 |
| zombie_package | 2 |

#### unused_export


#### unreachable_file


#### zombie_package
- `plugins` (2830 lines, conf 0.5): Package 'plugins' has no importers from other packages
- `prototype` (1900 lines, conf 0.5): Package 'prototype' has no importers from other packages

## Refactoring Targets (impact-per-effort ranked)

### #1: `crates/sc-composer/src/diagnostics.rs` — untested_hotspot (critical)

| Impact | Effort | ROI | Findings |
| 5.5 | S | 5.5 | 5 |

Reason: Hotspot with no paired test file and no coverage data — 37 dependents

### #2: `crates/sc-compose/tests/cli.rs` — co_change_scatter (high)

| Impact | Effort | ROI | Findings |
| 3.5 | S | 3.5 | 3 |

Reason: co-changes with 25 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)

### #3: `crates/sc-compose/tests/sc_lint_runner.rs` — co_change_scatter (medium)

| Impact | Effort | ROI | Findings |
| 4.5 | M | 2.3 | 11 |

Reason: co-changes with 12 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)

### #4: `crates/sc-composer-beads/src/error.rs` — untested_hotspot (high)

| Impact | Effort | ROI | Findings |
| 4.0 | M | 2.0 | 3 |

Reason: Hotspot with no paired test file and no coverage data — 4 dependents

### #5: `crates/sc-compose/tests/json_cli.rs` — prior_defect (critical)

| Impact | Effort | ROI | Findings |
| 2.0 | S | 2.0 | 1 |

Reason: 27 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects

### #6: `crates/sc-compose/tests/repo_boundaries.rs` — nested_complexity (medium)

| Impact | Effort | ROI | Findings |
| 5.8 | L | 1.9 | 22 |

Reason: assert_manifest_dependency_rules nests 4 levels deep
- **extract_method**: 

### #7: `crates/sc-composer/src/extract/xml_model.rs` — untested_hotspot (high)

| Impact | Effort | ROI | Findings |
| 5.6 | L | 1.9 | 6 |

Reason: Hotspot with no paired test file and no coverage data — 4 dependents

### #8: `crates/sc-compose/src/commands/mod.rs` — co_change_scatter (high)

| Impact | Effort | ROI | Findings |
| 3.6 | M | 1.8 | 4 |

Reason: co-changes with 23 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **extract_helper**: 

### #9: `crates/sc-compose/src/commands/examples.rs` — prior_defect (high)

| Impact | Effort | ROI | Findings |
| 1.8 | S | 1.8 | 2 |

Reason: 3 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects

### #10: `crates/sc-composer/src/discovery.rs` — prior_defect (critical)

| Impact | Effort | ROI | Findings |
| 5.3 | L | 1.8 | 6 |

Reason: 4 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **break_cycle**: 
- **extract_helper**: 

## Recommendations (derived from this run)

1. **Start with the worst file** — `crates/sc-compose/src/commands/template_lint.rs` (2.58/10, 576 NLOC). Decompose by responsibility before anything else.
2. **Close test gaps on depended-upon files** — 5 hotspot file(s) lack paired tests: `crates/sc-compose/src/commands/reports/mod.rs`, `crates/sc-compose/src/reporting/catalog.rs`, `crates/sc-compose/src/reporting/index.rs`, `crates/sc-compose/src/reporting/spec.rs`, `crates/sc-composer-beads/src/error.rs`.
3. **Extract duplication** — 352 duplicated assertion blocks; shared test-helper modules would remove the bulk.
4. **Audit sync I/O on hot paths** — 79 findings; either make async or document the intentional sync boundary.
5. **Fastest win** — refactoring target #1: `crates/sc-composer/src/diagnostics.rs` (S effort, ROI 5.5).

---
*Generated by `.sc/repowise/generate-report.py` from scoped repowise data (2026-08-31, b763d2ff). Scope per `.sc/repowise.yaml`: modules + annotated exclusions. No hardcoded prose — every figure is computed from the JSON in `.sc/repowise/data/`.*
