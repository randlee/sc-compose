---
id: K.8
title: Report Output Materialization
phase: K
status: complete
branch: sprint/k-8-report-output
worktree: ../sc-compose-worktrees/sprint/k-8-report-output
target: integrate/phase-k
---

# Sprint K.8 — Report Output Materialization

## Purpose and evidence

Issue #311 ranks `crates/sc-compose/src/reporting/output.rs` at 4.21/10 and reports 28% duplication. The module combines report metadata models, latest/archive path policy, path validation, directory creation, artifact copying, metadata serialization, timestamp formatting, and output errors. This sprint makes those seams explicit without changing report artifacts.

## Goal

Produce a production-ready private decomposition of report path policy,
materialization, and metadata while preserving the artifact contract.

## Required work

- Record the baseline report metadata, path, ordering, and error
  characterization before moving implementation code.
- Implement only the seams listed under Exact targets and deliverables, retain
  reporting output call sites and catalog separation, and rerun the
  characterization suite after the move.
- Record ownership and production-NLOC evidence and complete every command in
  Required validation before claiming closure.

## Hard dependencies

The hard dependencies are this sprint's plan-gate approval and
`integrate/phase-k` as the merge-forward target. There is no hard dependency on
another Phase K sprint.

## Production-ready expectation

Every deliverable listed below must land at production-ready quality for this
sprint's behavior-preserving scope. Partial module movement, test-only work,
or an unmeasured ownership split cannot satisfy the acceptance criteria.

## Exact targets and deliverables

- `crates/sc-compose/src/reporting/output.rs`, especially
  `ReportOutputRequest`, `MaterializedReport`, `FinalizeReportRequest`,
  `write_report_metadata_and_archive`, `finalize_report_outputs`, path
  helpers, copy/archive helpers, and `OutputError`.
- Create private layout/validation, materialization, and metadata modules while preserving all current `reporting::output` call sites and serialized fields.
- Add or strengthen characterization tests for latest versus archive paths,
  timestamp names, relative-path containment, artifact ordering, metadata JSON,
  overwrite behavior, and each `OutputError` mapping before moving code.

## Planned seam

Path policy and materialization may become private modules, but the reporting
request/result structures and output functions retain their current shape:

```rust
pub(crate) struct FinalizeReportRequest {
    pub(crate) report_id: String,
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) entrypoint: PathBuf,
    pub(crate) artifacts: Vec<PathBuf>,
    pub(crate) archive: bool,
}

pub(crate) fn finalize_report_outputs(
    root: &Path,
    request: &FinalizeReportRequest,
) -> Result<MaterializedReport, OutputError>;
```

`crate::reporting::output` remains the sole current call-site surface. No
catalog field, report artifact path, or output source path is deleted or
renamed.

## Acceptance criteria

- Report directory layout, file names, metadata fields, artifact ordering, path separators, archive policy, and error behavior are unchanged.
- No catalog schema or report producer behavior changes; `catalog.rs` is not modified by this sprint.
- Production-NLOC evidence shows clear ownership of path policy versus I/O materialization.
- The sprint does not move report production into `catalog.rs` and does not
  add a second path-containment policy; a failed seam characterization leaves
  the current materialization path intact.

## Required validation

Run these focused commands against the baseline before the move and rerun the
same commands after the move:

- `cargo test -p sc-compose reporting::output::tests`
- `cargo test -p sc-compose --test json_cli -- report`
- `cargo fmt --all --check`
- `git diff --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

Record metadata, artifact-order, path-policy, and before/after production-NLOC
evidence.

## Dependencies and non-closure

Independent from K.1-K.7. No new report formats, publication destinations, or catalog fields are in scope.

## Completion evidence

- Baseline characterization was run at merge-forward baseline `f8071de` before
  moving implementation code. The direct output test filter matched 0 tests;
  the JSON report characterization passed 28 tests; formatting and diff
  checks passed.
- Direct characterization now covers latest versus archive layout, timestamped
  archive paths, relative-path containment, entrypoint-first artifact ordering,
  metadata JSON fields and ordering, overwrite behavior, all filesystem error
  categories, and `OutputError` display mappings. The post-move output suite
  passed all 5 tests, and the existing 28 JSON report tests remained green.
- The existing `reporting::output` call-site surface and serialized fields are
  unchanged. No catalog code, report producer behavior, report format, or
  publication destination was changed.
- Private ownership is explicit: `layout.rs` owns latest/archive roots and
  relative containment (37 production lines), `materialization.rs` owns
  metadata writes and artifact copying (50), and `metadata.rs` owns metadata
  shape, forward-slash serialization, and timestamps (44). The facade retains
  the established request/result/error types and orchestration. Using the
  same nonblank, noncomment count, the baseline output module had 257
  production lines; the post-move largest private owner is 50 lines and the
  facade is 182 lines including the unchanged public(crate) surface and error
  display implementation.
- Required post-move validation passed: `cargo test -p sc-compose
  reporting::output::tests` (5), `cargo test -p sc-compose --test json_cli --
  report` (28), `cargo fmt --all --check`, `git diff --check`, clippy with
  `-D warnings`, and `cargo test --workspace` (266 unit, 51 extraction
  integration, 16 integration).
