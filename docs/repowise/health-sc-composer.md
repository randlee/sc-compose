# crates/sc-composer — Repowise Code Health

**Version:** v1.6.0-15-g54fff289 | **Commit:** 54fff289 | **Generated:** 2026-08-29
**Analyzed:** 76 files | **Biomarker findings:** 433 | **Data:** repowise health + dead-code + refactoring-targets

![health 8.26/10](https://img.shields.io/badge/health-82%2F100-brightgreen)

## Quick Summary

| Metric | Value |
|---|---|
| Overall Health | **8.26/10** |
| Hotspot Health | 5.15/10 |
| Worst In-Scope File | `crates/sc-composer/src/extract/yaml.rs` (2.54/10) |
| Files Scored (this scope) | 76 |
| Files in Full Index | 999 (repo-wide scan; this table = module only) |
| Maintainability | 9.07 avg / 7.43 hotspot |
| Performance | 9.70 avg / 9.06 hotspot |

**Read:** in-scope average 8.26/10 with hotspot 5.15/10 — 1 of 76 scored file sit below 3.0/10 and carry the hotspot drag.

## Worst 20 Files by Health Score

| Score | File | NLOC | CCN | Nest | Dup% |
|---|---|---|---|---|---|
| 2.54 | `crates/sc-composer/src/extract/yaml.rs` | 789 | 21 | 6 | 43.4 |
| 3.79 | `crates/sc-composer/src/extract/xml.rs` | 324 | 8 | 5 | 22.1 |
| 4.06 | `crates/sc-composer/src/extract/json.rs` | 618 | 15 | 4 | 55.3 |
| 4.42 | `crates/sc-composer-beads/src/runner.rs` | 254 | 12 | 4 | 5.0 |
| 4.44 | `crates/sc-composer/src/extract/xml_model.rs` | 227 | 13 | 4 | — |
| 4.44 | `crates/sc-composer/src/resolver.rs` | 714 | 7 | 4 | 30.3 |
| 4.50 | `crates/sc-composer/src/diagnostics.rs` | 9 | 1 | 0 | — |
| 4.65 | `crates/sc-composer-beads/src/execute.rs` | 936 | 16 | 2 | 20.7 |
| 4.71 | `crates/sc-composer/src/discovery.rs` | 271 | 9 | 4 | 3.9 |
| 4.77 | `crates/sc-composer/src/validation/diagnostics.rs` | 1102 | 8 | 3 | 29.2 |
| 4.78 | `crates/sc-composer/src/composer.rs` | 690 | 8 | 2 | 39.3 |
| 4.93 | `crates/sc-composer/src/init_workspace.rs` | 221 | 13 | 3 | 15.9 |
| 5.05 | `crates/sc-composer/src/renderer.rs` | 1079 | 7 | 3 | 16.4 |
| 5.12 | `crates/sc-composer/tests/extract_integration.rs` | 1349 | 7 | 3 | 17.1 |
| 5.32 | `crates/sc-composer/src/extract/toml.rs` | 509 | 18 | 4 | 62.9 |
| 5.50 | `crates/sc-composer/src/types.rs` | 575 | 9 | 3 | 21.4 |
| 5.54 | `crates/sc-composer/tests/integration.rs` | 502 | 2 | 1 | 48.6 |
| 5.90 | `crates/sc-composer/src/include.rs` | 958 | 4 | 2 | 70.6 |
| 5.98 | `crates/sc-composer-beads/src/error.rs` | 135 | 2 | 1 | — |
| 6.15 | `crates/sc-composer/src/lib.rs` | 230 | 1 | 0 | 21.3 |

**Observations**
- `crates/sc-composer/src/extract/yaml.rs` (2.5/10, 789 NLOC) — high cyclomatic complexity, CCN=21, 43.36% duplication
- `crates/sc-composer/src/extract/json.rs` (4.1/10, 618 NLOC) — high duplication, CCN=15, 55.34% duplication
- `crates/sc-composer/src/extract/yaml.rs` (2.5/10, 789 NLOC) — deep nesting, CCN=21, 43.36% duplication

## Biomarker Findings

| Type | Count | What It Means |
|---|---|---|
| hot_path_sync_io | 86 | Sync I/O on hot paths — should be async |
| prior_defect | 51 | Files with bug-fix history — strong defect predictor |
| error_handling | 50 | Error handling gaps or inconsistencies |
| dry_violation | 43 | DRY violations — opportunities to extract shared code |
| co_change_scatter | 25 | Files that change together — high coupling |
| complex_method | 25 |  |
| primitive_obsession | 22 |  |
| duplicated_assertion_block | 21 | Repeated assertion patterns — test helper opportunity |
| large_method | 20 |  |
| nested_complexity | 18 |  |
| change_entropy | 17 |  |
| churn_risk | 14 | High recent change frequency — churn-driven risk |
| function_hotspot | 14 |  |
| low_cohesion | 11 |  |
| io_in_loop | 11 |  |
| untested_hotspot | 3 | Depended-upon files with no paired test coverage |
| nested_loop_with_io | 1 |  |
| brain_method | 1 |  |

### Highest-Impact Findings (by type)

### [86] hot_path_sync_io
Sync I/O on hot paths — should be async
- **low** `crates/sc-composer/src/composer.rs` `temp_root`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/composer.rs` `write_file`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/frontmatter_init.rs` `frontmatter_init`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/frontmatter_init.rs` `temp_root`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/frontmatter_init.rs` `write_file`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/include.rs` `directory_include_target_reports_is_a_directory`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function

### [51] prior_defect
Files with bug-fix history — strong defect predictor
- **critical** `crates/sc-composer/src/frontmatter/parser.rs` `(top-level)`: 5 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer-beads/src/render.rs` `(top-level)`: 4 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer/src/error.rs` `(top-level)`: 5 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **high** `crates/sc-composer/src/frontmatter_init.rs` `(top-level)`: 3 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **high** `crates/sc-composer/src/observer.rs` `(top-level)`: 4 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **high** `crates/sc-composer/src/validate.rs` `(top-level)`: 4 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects

### [50] error_handling
Error handling gaps or inconsistencies
- **low** `crates/sc-composer/src/extract/tests.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-composer/src/extract/xml_match.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-composer/src/include/expansion.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-composer/src/include/expansion.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-composer/src/validation/diagnostics.rs` `(top-level)`: panic!/unreachable!/todo!/unimplemented! aborts the process unconditionally
- **low** `crates/sc-composer/src/validation/diagnostics.rs` `(top-level)`: panic!/unreachable!/todo!/unimplemented! aborts the process unconditionally

### [43] dry_violation
DRY violations — opportunities to extract shared code
- **high** `crates/sc-composer/src/composer.rs` `(top-level)`: 39% of file duplicated; worst clone shares 31 lines with crates/sc-composer/src/validate.rs (co-changed 10x)
- **high** `crates/sc-composer/src/include.rs` `(top-level)`: 71% of file duplicated; worst clone shares 16 lines with crates/sc-composer/src/resolver.rs (co-changed 8x)
- **high** `crates/sc-composer/src/resolver.rs` `(top-level)`: 30% of file duplicated; worst clone shares 16 lines with crates/sc-composer/src/include.rs (co-changed 8x)
- **high** `crates/sc-composer/src/validate.rs` `(top-level)`: 51% of file duplicated; worst clone shares 31 lines with crates/sc-composer/src/composer.rs (co-changed 10x)
- **high** `crates/sc-composer/src/verify.rs` `(top-level)`: 42% of file duplicated; worst clone shares 10 lines with crates/sc-compose/src/commands/template_init.rs (co-changed 3x)
- **high** `crates/sc-composer/src/diagnostics/schema.rs` `(top-level)`: 36% of file duplicated; worst clone shares 170 lines with crates/sc-composer/src/lib.rs (co-changed 3x)

### [25] co_change_scatter
Files that change together — high coupling
- **high** `crates/sc-composer/src/diagnostics/schema.rs` `(top-level)`: co-changes with 22 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/extract/mod.rs` `(top-level)`: co-changes with 21 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/extract/yaml.rs` `(top-level)`: co-changes with 19 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/init_workspace.rs` `(top-level)`: co-changes with 25 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/error.rs` `(top-level)`: co-changes with 24 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/composer.rs` `(top-level)`: co-changes with 25 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)

### [25] complex_method
- **medium** `crates/sc-composer/src/discovery.rs` `discover_tokens_with_delimiters`: discover_tokens_with_delimiters has cyclomatic complexity 9
- **medium** `crates/sc-composer/src/init_workspace.rs` `init_workspace`: init_workspace has cyclomatic complexity 13
- **medium** `crates/sc-composer/src/types.rs` `input_value_from_yaml`: input_value_from_yaml has cyclomatic complexity 9
- **medium** `crates/sc-composer/src/extract/xml_evidence.rs` `collect_template_occurrences`: collect_template_occurrences has cyclomatic complexity 9
- **medium** `crates/sc-composer/src/extract/xml_prefix.rs` `normalize_rendered`: normalize_rendered has cyclomatic complexity 12
- **medium** `crates/sc-composer/src/validation/required_paths.rs` `required_variable_location`: required_variable_location has cyclomatic complexity 11

## Dead Code / Unreachable

| Kind | Count |
|---|---|
| unused_export | 143 |
| unreachable_file | 32 |
| zombie_package | 2 |

#### unused_export


#### unreachable_file


#### zombie_package
- `plugins` (2820 lines, conf 0.5): Package 'plugins' has no importers from other packages
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

### #3: `crates/sc-compose/src/cli/capability.rs` — complex_method (high)

| Impact | Effort | ROI | Findings |
| 4.8 | M | 2.4 | 5 |

Reason: command_wants_json has cyclomatic complexity 22

### #4: `crates/sc-compose/tests/sc_lint_runner.rs` — co_change_scatter (medium)

| Impact | Effort | ROI | Findings |
| 4.5 | M | 2.3 | 12 |

Reason: co-changes with 12 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)

### #5: `crates/sc-compose/src/commands/compose.rs` — untested_hotspot (high)

| Impact | Effort | ROI | Findings |
| 6.6 | L | 2.2 | 12 |

Reason: Hotspot with no paired test file and no coverage data — 4 dependents
- **extract_method**: 

### #6: `crates/sc-compose/tests/repo_boundaries.rs` — complex_method (high)

| Impact | Effort | ROI | Findings |
| 6.4 | L | 2.1 | 20 |

Reason: repo_keeps_standalone_boundary_rules has cyclomatic complexity 18
- **extract_method**: 

### #7: `crates/sc-composer/src/extract/xml.rs` — nested_complexity (high)

| Impact | Effort | ROI | Findings |
| 6.2 | L | 2.1 | 9 |

Reason: map_raw_text_error nests 5 levels deep
- **extract_helper**: 
- **split_file**: 

### #8: `crates/sc-composer-beads/src/error.rs` — untested_hotspot (high)

| Impact | Effort | ROI | Findings |
| 4.0 | M | 2.0 | 3 |

Reason: Hotspot with no paired test file and no coverage data — 8 dependents

### #9: `crates/sc-compose/src/path_utils.rs` — untested_hotspot (critical)

| Impact | Effort | ROI | Findings |
| 4.0 | M | 2.0 | 2 |

Reason: Hotspot with no paired test file and no coverage data — 22 dependents

### #10: `crates/sc-compose/tests/json_cli.rs` — prior_defect (critical)

| Impact | Effort | ROI | Findings |
| 2.0 | S | 2.0 | 1 |

Reason: 27 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects

## Recommendations (derived from this run)

1. **Start with the worst file** — `crates/sc-composer/src/extract/yaml.rs` (2.54/10, 789 NLOC), CCN=21. Decompose by responsibility before anything else.
2. **Close test gaps on depended-upon files** — 3 hotspot file(s) lack paired tests: `crates/sc-composer-beads/src/error.rs`, `crates/sc-composer/src/diagnostics.rs`, `crates/sc-composer/src/extract/xml_model.rs`.
3. **Extract duplication** — 21 duplicated assertion blocks; shared test-helper modules would remove the bulk.
4. **Audit sync I/O on hot paths** — 86 findings; either make async or document the intentional sync boundary.
5. **Fastest win** — refactoring target #1: `crates/sc-composer/src/diagnostics.rs` (S effort, ROI 5.5).

---
*Generated by `.sc/repowise/generate-report.py` from scoped repowise data (2026-08-29, 54fff289). Scope per `.sc/repowise.yaml`: modules + annotated exclusions. No hardcoded prose — every figure is computed from the JSON in `.sc/repowise/data/`.*
