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

Run `cargo test -p sc-compose reporting::output::tests` and
`cargo test -p sc-compose --test json_cli -- report` against the baseline
before the move and rerun the same commands after the move. Then run `cargo
fmt --all --check`, `git diff --check`, `cargo clippy --all-targets
--all-features -- -D warnings`, and `cargo test --workspace`. Record metadata,
artifact-order, path-policy, and before/after production-NLOC evidence.

## Dependencies and non-closure

Independent from K.1-K.7. No new report formats, publication destinations, or catalog fields are in scope.
