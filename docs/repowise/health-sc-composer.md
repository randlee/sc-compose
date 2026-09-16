# crates/sc-composer — Repowise Code Health

**Version:** release-candidate-v1.6.1-10-gb763d2ff | **Commit:** b763d2ff | **Generated:** 2026-08-31
**Analyzed:** 79 files | **Biomarker findings:** 359 | **Data:** repowise health + dead-code + refactoring-targets

![health 8.32/10](https://img.shields.io/badge/health-83%2F100-brightgreen)

## Quick Summary

| Metric | Value |
|---|---|
| Overall Health | **8.32/10** |
| Hotspot Health | 5.21/10 |
| Worst In-Scope File | `.github/scripts/release_artifacts.py` (1.90/10) |
| Files Scored (this scope) | 79 |
| Files in Full Index | 1017 (repo-wide scan; this table = module only) |
| Maintainability | 9.09 avg / 7.45 hotspot |
| Performance | 9.77 avg / 9.24 hotspot |

**Read:** in-scope average 8.32/10 with hotspot 5.21/10 — 0 of 79 scored files sit below 3.0/10 and carry the hotspot drag.

## Worst 20 Files by Health Score

| Score | File | NLOC | CCN | Nest | Dup% |
|---|---|---|---|---|---|
| 4.44 | `crates/sc-composer/src/extract/xml_model.rs` | 227 | 13 | 4 | — |
| 4.44 | `crates/sc-composer/src/resolver.rs` | 714 | 7 | 4 | 30.3 |
| 4.50 | `crates/sc-composer/src/diagnostics.rs` | 9 | 1 | 0 | — |
| 4.65 | `crates/sc-composer-beads/src/execute.rs` | 936 | 16 | 2 | 20.7 |
| 4.70 | `crates/sc-composer/src/extract/yaml.rs` | 706 | 21 | 6 | 48.8 |
| 4.71 | `crates/sc-composer/src/discovery.rs` | 271 | 9 | 4 | 3.9 |
| 4.75 | `crates/sc-composer-beads/src/runner.rs` | 406 | 9 | 3 | 3.7 |
| 4.77 | `crates/sc-composer/src/validation/diagnostics.rs` | 1102 | 8 | 3 | 29.2 |
| 4.78 | `crates/sc-composer/src/composer.rs` | 690 | 8 | 2 | 39.3 |
| 4.93 | `crates/sc-composer/src/init_workspace.rs` | 221 | 13 | 3 | 15.9 |
| 5.05 | `crates/sc-composer/src/renderer.rs` | 1079 | 7 | 3 | 16.4 |
| 5.32 | `crates/sc-composer/src/extract/toml.rs` | 509 | 18 | 4 | 49.9 |
| 5.50 | `crates/sc-composer/src/types.rs` | 575 | 9 | 3 | 21.4 |
| 5.54 | `crates/sc-composer/tests/integration.rs` | 502 | 2 | 1 | 48.6 |
| 5.78 | `crates/sc-composer/src/extract/xml.rs` | 296 | 6 | 3 | 9.6 |
| 5.90 | `crates/sc-composer/src/include.rs` | 958 | 4 | 2 | 70.9 |
| 5.98 | `crates/sc-composer-beads/src/error.rs` | 135 | 2 | 1 | — |
| 6.15 | `crates/sc-composer/src/lib.rs` | 230 | 1 | 0 | 21.3 |
| 6.15 | `crates/sc-composer/src/error.rs` | 260 | 2 | 1 | 31.0 |
| 6.24 | `crates/sc-composer/src/validation/required_paths.rs` | 529 | 11 | 4 | 43.7 |

**Observations**
- `crates/sc-composer/src/extract/yaml.rs` (4.7/10, 706 NLOC) — high cyclomatic complexity, CCN=21, 48.81% duplication
- `crates/sc-composer/src/include.rs` (5.9/10, 958 NLOC) — high duplication, 70.91% duplication
- `crates/sc-composer/src/extract/yaml.rs` (4.7/10, 706 NLOC) — deep nesting, CCN=21, 48.81% duplication

## Biomarker Findings

| Type | Count | What It Means |
|---|---|---|
| prior_defect | 55 | Files with bug-fix history — strong defect predictor |
| error_handling | 49 | Error handling gaps or inconsistencies |
| dry_violation | 43 | DRY violations — opportunities to extract shared code |
| hot_path_sync_io | 29 | Sync I/O on hot paths — should be async |
| complex_method | 25 |  |
| primitive_obsession | 22 |  |
| co_change_scatter | 20 | Files that change together — high coupling |
| large_method | 19 |  |
| change_entropy | 17 |  |
| nested_complexity | 16 |  |
| duplicated_assertion_block | 16 | Repeated assertion patterns — test helper opportunity |
| low_cohesion | 12 |  |
| function_hotspot | 11 |  |
| io_in_loop | 11 |  |
| churn_risk | 10 | High recent change frequency — churn-driven risk |
| untested_hotspot | 3 | Depended-upon files with no paired test coverage |
| nested_loop_with_io | 1 |  |

### Highest-Impact Findings (by type)

### [55] prior_defect
Files with bug-fix history — strong defect predictor
- **critical** `crates/sc-composer/tests/extract_integration.rs` `(top-level)`: 6 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer/src/frontmatter/parser.rs` `(top-level)`: 5 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer-beads/src/render.rs` `(top-level)`: 4 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer/src/error.rs` `(top-level)`: 5 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer/src/extract/xml.rs` `(top-level)`: 4 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer-beads/src/runner.rs` `(top-level)`: 5 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects

### [49] error_handling
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

### [29] hot_path_sync_io
Sync I/O on hot paths — should be async
- **low** `crates/sc-composer/src/composer.rs` `temp_root`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/composer.rs` `write_file`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/frontmatter_init.rs` `frontmatter_init`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/frontmatter_init.rs` `temp_root`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/frontmatter_init.rs` `write_file`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function
- **low** `crates/sc-composer/src/include.rs` `new`: a blocking filesystem call runs on a hot, request-reachable path; its latency is paid on every call through this function

### [25] complex_method
- **high** `crates/sc-composer/src/extract/yaml_limits.rs` `validate_parse_depth`: validate_parse_depth has cyclomatic complexity 20
- **high** `crates/sc-composer/src/extract/json.rs` `match_json`: match_json has cyclomatic complexity 15
- **medium** `crates/sc-composer/src/discovery.rs` `discover_tokens_with_delimiters`: discover_tokens_with_delimiters has cyclomatic complexity 9
- **medium** `crates/sc-composer/src/init_workspace.rs` `init_workspace`: init_workspace has cyclomatic complexity 13
- **medium** `crates/sc-composer/src/types.rs` `input_value_from_yaml`: input_value_from_yaml has cyclomatic complexity 9
- **medium** `crates/sc-composer/src/extract/json_limits.rs` `validate_parse_depth`: validate_parse_depth has cyclomatic complexity 11

### [22] primitive_obsession
- **medium** `crates/sc-composer/src/include/expansion.rs` `expand_file`: expand_file takes 7 parameters
- **medium** `crates/sc-composer/src/extract/xml_match.rs` `match_child_sequence`: match_child_sequence takes 7 parameters
- **medium** `crates/sc-composer/src/extract/xml_match.rs` `match_element`: match_element takes 7 parameters
- **low** `crates/sc-composer/src/composer.rs` `render_all_with_observer`: render_all_with_observer takes 6 parameters
- **low** `crates/sc-composer/src/resolver.rs` `resolve_profile_impl`: resolve_profile_impl takes 5 parameters
- **low** `crates/sc-composer/src/extract/error.rs` `format_error_with_source`: format_error_with_source takes 5 parameters

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

1. **Start with the worst file** — `crates/sc-composer/src/extract/xml_model.rs` (4.44/10, 227 NLOC). Decompose by responsibility before anything else.
2. **Close test gaps on depended-upon files** — 3 hotspot file(s) lack paired tests: `crates/sc-composer-beads/src/error.rs`, `crates/sc-composer/src/diagnostics.rs`, `crates/sc-composer/src/extract/xml_model.rs`.
3. **Extract duplication** — 16 duplicated assertion blocks; shared test-helper modules would remove the bulk.
4. **Audit sync I/O on hot paths** — 29 findings; either make async or document the intentional sync boundary.
5. **Fastest win** — refactoring target #1: `crates/sc-composer/src/diagnostics.rs` (S effort, ROI 5.5).

---
*Generated by `.sc/repowise/generate-report.py` from scoped repowise data (2026-08-31, b763d2ff). Scope per `.sc/repowise.yaml`: modules + annotated exclusions. No hardcoded prose — every figure is computed from the JSON in `.sc/repowise/data/`.*
