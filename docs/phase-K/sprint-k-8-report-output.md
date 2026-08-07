---
id: K.8
title: Report Output Materialization
phase: K
status: planned
branch: sprint/k-8-report-output
worktree: ../sc-compose-worktrees/sprint/k-8-report-output
target: integrate/phase-k
---

# Sprint K.8 — Report Output Materialization

## Purpose and evidence

Issue #311 ranks `crates/sc-compose/src/reporting/output.rs` at 4.21/10 and reports 28% duplication. The module combines report metadata models, latest/archive path policy, path validation, directory creation, artifact copying, metadata serialization, timestamp formatting, and output errors. This sprint makes those seams explicit without changing report artifacts.

## Exact targets and deliverables

- `crates/sc-compose/src/reporting/output.rs:1-276`, especially `ReportOutputRequest`, `MaterializedReport`, `FinalizeReportRequest`, `write_report_metadata_and_archive`, path helpers, copy/archive helpers, and `OutputError`.
- Create private layout/validation, materialization, and metadata modules while preserving all current `reporting::output` call sites and serialized fields.
- Characterize latest versus archive paths, timestamp names, relative-path containment, artifact ordering, metadata JSON, overwrite behavior, and each `OutputError` mapping before moving code.

## Acceptance criteria

- Report directory layout, file names, metadata fields, artifact ordering, path separators, archive policy, and error behavior are unchanged.
- No catalog schema or report producer behavior changes; `catalog.rs` is not modified by this sprint.
- Production-NLOC evidence shows clear ownership of path policy versus I/O materialization.

## Required validation

Run focused reporting/output tests before and after, `cargo fmt --all --check`, `git diff --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.

## Dependencies and non-closure

Independent from K.1-K.7. No new report formats, publication destinations, or catalog fields are in scope.
