# sc-compose — Repowise Code Health Analysis

**Version:** v1.2.0-85-g252f283 | **Commit:** 252f283 | **Generated:** 2026-07-23
**Analyzed by:** repowise health + dead-code + refactoring-targets

## Quick Summary

| Metric | Value |
|---|---|
| Overall Health | **7.9/10** |
| Hotspot Health | 4.7/10 |
| Worst File | `crates/sc-compose/src/cli.rs` (1.6/10) |
| Files Indexed | 286 |
| Biomarker Findings | 516 |
| Dead Code Items | 61 |
| Refactoring Targets | 20 |

### Health Dimensions

| Dimension | Average | Hotspot |
|---|---|---|
| Maintainability | 9.0/10 | 7.4/10 |
| Performance | 9.8/10 | 9.4/10 |
| Overall | 7.9/10 | 4.7/10 |

*Interpretation:* 8.2/10 average with 5.2 hotspot health means most files are healthy but a concentrated few drag the score down. Maintainability (9.1) and performance (9.8) average are excellent, but the hotspot maintainability (7.3) reveals files needing modularization. The worst file (`validation.rs`) at 2.6/10 accounts for most of the hotspot drag.

## Worst 20 Files by Health Score

| Score | File | NLOC | CCN | Nest | Dup% |
|---|---|---|---|---|---|
| 1.6 | `crates/sc-compose/src/cli.rs` | 489 | 17 | 4 | 27.7 |
| 2.4 | `crates/sc-composer/src/validation.rs` | 1655 | 11 | 4 | 25.2 |
| 3.0 | `crates/sc-composer/src/types.rs` | 561 | 11 | 6 | 26.9 |
| 4.0 | `crates/sc-compose/src/reporting/output.rs` | 257 | 4 | 2 | 30.7 |
| 4.0 | `crates/sc-compose/src/render_request.rs` | 323 | 6 | 4 | 5.0 |
| 4.0 | `crates/sc-composer/src/diagnostics.rs` | 150 | 2 | 1 | 29.8 |
| 4.3 | `crates/sc-compose/src/reporting/catalog.rs` | 201 | 3 | 2 | 27.4 |
| 4.4 | `crates/sc-compose/src/commands/compose.rs` | 536 | 8 | 2 | 19.6 |
| 4.5 | `crates/sc-compose/src/path_utils.rs` | 69 | 6 | 2 | — |
| 4.8 | `crates/sc-compose/tests/cli.rs` | 3339 | 4 | 3 | 64.9 |
| 4.8 | `crates/sc-composer/src/frontmatter.rs` | 372 | 8 | 3 | 9.7 |
| 4.9 | `crates/sc-composer/src/composer.rs` | 584 | 5 | 2 | 49.2 |
| 5.0 | `crates/sc-compose/src/observer_impl.rs` | 819 | 8 | 2 | 16.2 |
| 5.1 | `crates/sc-composer/src/init_workspace.rs` | 221 | 13 | 3 | 15.9 |
| 5.4 | `crates/sc-compose/tests/json_cli.rs` | 1671 | 2 | 1 | 75.8 |
| 5.8 | `crates/sc-compose/src/reporting/init.rs` | 153 | 1 | 0 | 38.6 |
| 5.8 | `crates/sc-composer/src/renderer.rs` | 169 | 2 | 1 | — |
| 5.8 | `crates/sc-compose/src/reporting/publish_manifest.rs` | 265 | 6 | 3 | 14.0 |
| 6.0 | `crates/sc-composer/src/error.rs` | 583 | 4 | 2 | 56.4 |
| 6.2 | `crates/sc-compose/src/reporting/index.rs` | 156 | 6 | 3 | 17.9 |

**Key observations:**
- `validation.rs` (2.6/10, 1388 NLOC, CCN=11): the single biggest problem — large, complex, and duplicated (28% dup). This is a prime candidate for decomposition.
- Test files dominate the worst list: `cli.rs` (2586 NLOC, 61% dup), `json_cli.rs` (1640 NLOC, 76% dup) — these are expected for thorough testing but the duplication indicates test helper opportunities.
- `types.rs` (4.8/10, CCN=11, nest=6): deeply nested validation logic — the 6-deep nesting in `validate_input_value_at` is flagged separately.

## Best 10 Files (for contrast)

| Score | File | NLOC |
|---|---|---|
| 10.0 | `crates/sc-composer/Cargo.toml` | 22 |
| 10.0 | `crates/sc-compose/src/json_output.rs` | 48 |
| 10.0 | `crates/sc-compose/src/exit_codes.rs` | 3 |
| 10.0 | `crates/sc-compose/Cargo.toml` | 28 |
| 9.8 | `crates/sc-compose/src/reporting/path.rs` | 56 |
| 9.8 | `crates/sc-compose/src/commands/verify.rs` | 124 |
| 9.8 | `crates/sc-compose/src/command_error.rs` | 149 |
| 9.7 | `crates/sc-compose/src/reporting/scaffold.rs` | 134 |
| 9.7 | `crates/sc-compose/src/commands/workspace.rs` | 104 |
| 9.6 | `crates/sc-compose/src/reporting/report_context.rs` | 139 |

## Biomarker Findings

| Type | Count | What It Means |
|---|---|---|
| duplicated_assertion_block | 130 | Repeated assertion patterns — test helper opportunity |
| hot_path_sync_io | 52 | Sync I/O on hot paths — should be async |
| prior_defect | 46 | Files with bug-fix history — strong defect predictor |
| dry_violation | 38 | DRY violations — opportunities to extract shared code |
| error_handling | 30 | Error handling gaps or inconsistencies |
| hidden_coupling | 21 | Implicit dependencies between modules |
| co_change_scatter | 21 | Files that change together → high coupling |
| io_in_loop | 22 | |
| large_method | 16 | |
| function_hotspot | 16 | |
| change_entropy | 13 | |
| complex_method | 11 | |
| low_cohesion | 11 | |
| untested_hotspot | 10 | |
| primitive_obsession | 10 | |
| churn_risk | 9 | |
| nested_complexity | 7 | |
| brain_method | 1 | |
| bumpy_road | 1 | |
| nested_loop_with_io | 1 | |

### prior_defect (53 findings)

- **critical** `crates/sc-compose/src/commands/compose.rs` `(top-level)`: 3 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/tests/support/mod.rs` `(top-level)`: 6 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/reporting/output.rs` `(top-level)`: 6 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-composer/src/renderer.rs` `(top-level)`: 5 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/observability.rs` `(top-level)`: 18 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- **critical** `crates/sc-compose/src/reporting/catalog.rs` `(top-level)`: 12 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects
- *... and 47 more*

### untested_hotspot (10 findings)

- **critical** `crates/sc-compose/src/path_utils.rs` `(top-level)`: Hotspot with no paired test file and no coverage data — 18 dependents
- **critical** `crates/sc-composer/src/diagnostics.rs` `(top-level)`: Hotspot with no paired test file and no coverage data — 15 dependents
- **high** `crates/sc-compose/src/cli.rs` `(top-level)`: Hotspot with no paired test file and no coverage data — 9 dependents
- **high** `crates/sc-compose/src/render_request.rs` `(top-level)`: Hotspot with no paired test file and no coverage data — 5 dependents
- **high** `crates/sc-compose/src/template_store.rs` `(top-level)`: Hotspot with no paired test file and no coverage data — 5 dependents
- **high** `crates/sc-compose/src/commands/compose.rs` `(top-level)`: Hotspot with no paired test file and no coverage data — 4 dependents
- *... and 4 more*

### co_change_scatter (26 findings)

- **high** `crates/sc-composer/src/diagnostics.rs` `(top-level)`: co-changes with 18 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/init_workspace.rs` `(top-level)`: co-changes with 16 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/renderer.rs` `(top-level)`: co-changes with 17 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/frontmatter.rs` `(top-level)`: co-changes with 16 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/types.rs` `(top-level)`: co-changes with 21 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- **high** `crates/sc-composer/src/composer.rs` `(top-level)`: co-changes with 22 distinct files — editing this file tends to ripple across the codebase (shotgun surgery)
- *... and 20 more*

### churn_risk (9 findings)

- **critical** `crates/sc-compose/src/var_file.rs` `(top-level)`: 90-day churn rewrote 6.4x the file's size (588 lines over 92 NLOC, top 24% of repo churn)
- **critical** `crates/sc-compose/src/reporting/init.rs` `(top-level)`: 90-day churn rewrote 8.6x the file's size (1311 lines over 153 NLOC, top 7% of repo churn)
- **critical** `crates/sc-compose/src/main.rs` `(top-level)`: 90-day churn rewrote 50.8x the file's size (3606 lines over 71 NLOC, top 1% of repo churn)
- **high** `crates/sc-compose/src/render_request.rs` `(top-level)`: 90-day churn rewrote 2.7x the file's size (878 lines over 323 NLOC, top 8% of repo churn)
- **medium** `crates/sc-compose/src/reporting/render_many.rs` `(top-level)`: 90-day churn rewrote 2.3x the file's size (735 lines over 325 NLOC, top 18% of repo churn)
- **low** `crates/sc-compose/src/commands/compose.rs` `(top-level)`: 90-day churn rewrote 1.4x the file's size (771 lines over 536 NLOC, top 16% of repo churn)
- *... and 3 more*

### hidden_coupling (20 findings)

- **medium** `crates/sc-composer/src/validate.rs` `(top-level)`: crates/sc-composer/src/composer.rs co-changes with this file 5 times (56% of shared commits) but no static dependency exists
- **medium** `crates/sc-compose/src/reporting/output.rs` `(top-level)`: crates/sc-compose/src/main.rs co-changes with this file 5 times (50% of shared commits) but no static dependency exists
- **medium** `crates/sc-compose/src/commands/mod.rs` `(top-level)`: crates/sc-compose/src/var_file.rs co-changes with this file 4 times (67% of shared commits) but no static dependency exists
- **medium** `crates/sc-compose/src/reporting/index.rs` `(top-level)`: crates/sc-compose/src/reporting/render_many.rs co-changes with this file 3 times (50% of shared commits) but no static dependency exists
- **medium** `crates/sc-compose/src/reporting/index.rs` `(top-level)`: crates/sc-compose/src/reporting/templates.rs co-changes with this file 3 times (50% of shared commits) but no static dependency exists
- **medium** `crates/sc-compose/src/reporting/index.rs` `(top-level)`: crates/sc-compose/src/main.rs co-changes with this file 3 times (50% of shared commits) but no static dependency exists
- *... and 14 more*

### error_handling (34 findings)

- **low** `crates/sc-compose/src/main_tests.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-compose/src/main_tests.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-composer/src/renderer.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-composer/src/validation.rs` `(top-level)`: panic!/unreachable!/todo!/unimplemented! aborts the process unconditionally
- **low** `crates/sc-compose/tests/repo_boundaries.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- **low** `crates/sc-compose/tests/repo_boundaries.rs` `(top-level)`: unwrap/expect turns a recoverable error into a crash
- *... and 28 more*

## Refactoring Targets

Prioritized by impact-per-effort ratio (highest ROI first).

### #1: `crates/sc-composer/src/diagnostics.rs` (4.0/10, 150 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **co_change_scatter** (high) |
| Impact Score | 6.0 |
| Effort | M |
| ROI | 3.0 |
| Finding Count | 4 |
| Reason | co-changes with 18 distinct files — editing this file tends to ripple across the codebase (shotgun surgery) |

### #2: `crates/sc-compose/src/path_utils.rs` (4.5/10, 69 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **untested_hotspot** (critical) |
| Impact Score | 5.5 |
| Effort | M |
| ROI | 2.7 |
| Finding Count | 4 |
| Reason | Hotspot with no paired test file and no coverage data — 18 dependents |

### #3: `crates/sc-compose/src/reporting/output.rs` (4.0/10, 257 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **prior_defect** (critical) |
| Impact Score | 6.0 |
| Effort | L |
| ROI | 2.0 |
| Finding Count | 8 |
| Reason | 6 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects |

- **extract_helper**: {'occurrences': [{'file': 'crates/sc-compose/src/reporting/output.rs', 'line_start': 115, 'line_end': 122}, {'file': 'crates/sc-compose/src/reporting/
- **extract_helper**: {'occurrences': [{'file': 'crates/sc-compose/src/reporting/output.rs', 'line_start': 184, 'line_end': 191}, {'file': 'crates/sc-compose/src/reporting/

### #4: `crates/sc-compose/src/render_request.rs` (4.0/10, 323 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **untested_hotspot** (high) |
| Impact Score | 6.0 |
| Effort | L |
| ROI | 2.0 |
| Finding Count | 11 |
| Reason | Hotspot with no paired test file and no coverage data — 5 dependents |

- **extract_helper**: {'occurrences': [{'file': 'crates/sc-compose/src/render_request.rs', 'line_start': 240, 'line_end': 247}, {'file': 'crates/sc-compose/src/render_reque

### #5: `crates/sc-compose/src/reporting/mod.rs` (8.1/10, 13 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **co_change_scatter** (medium) |
| Impact Score | 1.9 |
| Effort | S |
| ROI | 1.9 |
| Finding Count | 3 |
| Reason | co-changes with 8 distinct files — editing this file tends to ripple across the codebase (shotgun surgery) |

### #6: `crates/sc-compose/src/reporting/catalog.rs` (4.3/10, 201 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **prior_defect** (critical) |
| Impact Score | 5.7 |
| Effort | L |
| ROI | 1.9 |
| Finding Count | 6 |
| Reason | 12 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects |

- **extract_helper**: {'occurrences': [{'file': 'crates/sc-compose/src/reporting/catalog.rs', 'line_start': 138, 'line_end': 162}, {'file': 'crates/sc-compose/src/reporting
- **extract_helper**: {'occurrences': [{'file': 'crates/sc-compose/src/reporting/catalog.rs', 'line_start': 33, 'line_end': 40}, {'file': 'crates/sc-compose/src/reporting/i

### #7: `crates/sc-compose/src/commands/mod.rs` (6.3/10, 81 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **prior_defect** (critical) |
| Impact Score | 3.6 |
| Effort | M |
| ROI | 1.8 |
| Finding Count | 5 |
| Reason | 10 bug-fixes touched this file in the last ~6 months; recent defect history is the strongest cost-effective predictor of further defects |

- **extract_helper**: {'occurrences': [{'file': 'crates/sc-compose/src/commands/mod.rs', 'line_start': 1, 'line_end': 8}, {'file': 'crates/sc-compose/src/reporting/mod.rs',

### #8: `crates/sc-compose/src/main.rs` (6.5/10, 71 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **change_entropy** (critical) |
| Impact Score | 3.5 |
| Effort | M |
| ROI | 1.8 |
| Finding Count | 7 |
| Reason | changes are scattered across noisy commits (top 4% change entropy); a strong history-based fault predictor |

### #9: `crates/sc-compose/src/var_file.rs` (6.5/10, 92 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **churn_risk** (critical) |
| Impact Score | 3.5 |
| Effort | M |
| ROI | 1.8 |
| Finding Count | 5 |
| Reason | 90-day churn rewrote 6.4x the file's size (588 lines over 92 NLOC, top 24% of repo churn) |

### #10: `crates/sc-composer/src/frontmatter.rs` (4.8/10, 372 NLOC)

| Metric | Value |
|---|---|
| Biomarker | **co_change_scatter** (high) |
| Impact Score | 5.2 |
| Effort | L |
| ROI | 1.7 |
| Finding Count | 8 |
| Reason | co-changes with 16 distinct files — editing this file tends to ripple across the codebase (shotgun surgery) |

## Dead Code Analysis

**Note:** 51 `unused_export` findings in `bindings/python/python/sc_compose/_native.pyi` are PyO3 auto-generated type stubs — not genuine dead code. They are excluded from the actionable count below.

| Kind | Total | Actionable | Action |
|---|---|---|---|
| unreachable_file | 5 | 4 | Review — may be dead or scripts/prototypes |
| unused_export | 55 | 24 | 24 clean-up candidates |
| zombie_package | 1 | 1 | Review prototype/ package |

### Unreachable Files

- `prototype/multipass/e2e_demo.py` (10 lines) — File has no importers (in_degree=0) [risks: none]
- `prototype/multipass/run_tests.py` (200 lines) — File has no importers (in_degree=0) [risks: none]
- `scripts/atm-nudge.py` (280 lines) — File has no importers (in_degree=0) [risks: script]
- `scripts/release_artifacts.py` (200 lines) — File has no importers (in_degree=0) [risks: script]

### Actionable Unused Exports (excl. PyO3 stubs)

- `prototype/multipass/examples.py`: `write_temp` — Public symbol 'write_temp' has no importers
- `prototype/multipass/run_tests.py`: `test_parse_no_headers` — Public symbol 'test_parse_no_headers' has no importers
- `prototype/multipass/run_tests.py`: `test_parse_single_header_no_pass_field` — Public symbol 'test_parse_single_header_no_pass_field' has no importers
- `prototype/multipass/run_tests.py`: `test_parse_single_header_with_pass` — Public symbol 'test_parse_single_header_with_pass' has no importers
- `prototype/multipass/run_tests.py`: `test_parse_stacked_headers` — Public symbol 'test_parse_stacked_headers' has no importers
- `prototype/multipass/run_tests.py`: `test_parse_empty_headers` — Public symbol 'test_parse_empty_headers' has no importers
- `prototype/multipass/run_tests.py`: `test_parse_with_dots_delimiter` — Public symbol 'test_parse_with_dots_delimiter' has no importers
- `prototype/multipass/run_tests.py`: `test_discover_standard_braces` — Public symbol 'test_discover_standard_braces' has no importers
- `prototype/multipass/run_tests.py`: `test_discover_triple_braces` — Public symbol 'test_discover_triple_braces' has no importers
- `prototype/multipass/run_tests.py`: `test_discover_quadruple_braces` — Public symbol 'test_discover_quadruple_braces' has no importers
- *... and 14 more*

## Top Recommendations

### 1. Decompose `validation.rs` (2.6/10, 1388 NLOC)
The largest and worst-scoring file. It has 20 biomarker findings, CCN=11, 28% duplication, and co-changes with 24 other files. Split into per-category validators: `var_validation.rs`, `frontmatter_validation.rs`, `include_validation.rs`.

### 2. Add tests for untested hotspots
11 files flagged as untested hotspots — `path_utils.rs` (16 dependents), `diagnostics.rs` (13 dependents), `cli.rs` (7 dependents). These are heavily depended-upon files with no paired test coverage. Prioritize `path_utils.rs` first (critical severity, highest ROI refactoring target).

### 3. Extract test helpers (`cli.rs` 61% dup, `json_cli.rs` 76% dup)
130 duplicated assertion blocks in test files — extract shared assertion helpers. The high duplication percentage in test files is expected but the volume (130 findings) signals a real maintenance burden.

### 4. Address sync I/O on hot paths (52 findings)
52 hot path sync I/O findings — likely from file I/O in the rendering pipeline. Consider async or at minimum document the sync I/O is intentional for CLI tools.

### 5. Review prototype/ package visibility
The `prototype/` directory is flagged as a zombie package. If actively used for experimentation, add to the repowise config's `annotated` section. Otherwise, consider archiving.

---
*Generated by repowise v1.x — codebase intelligence for developers. Config: .sc/repowise.yaml with modules `crates/sc-compose`, `crates/sc-composer` and annotated paths `bindings/python`, `prototype`, `scripts`.*