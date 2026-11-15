//! Shared read/validate/plan/apply engine for attach and by-path pour.

mod apply;
mod plan;
mod repour;
mod validate;

use crate::contract::{
    BeadComposeReceipt, BeadComposeRequest, BeadEdgeAction, BeadGraph, BeadGraphMode, BeadId,
    BeadNodeAction, BeadOperation, BeadOutcome, BeadPourMode, BeadStage, BeadStageOutcome,
    BeadStageReceipt, GraphDependencyType,
};
use crate::error::{BeadComposeError, short_cause};
use crate::execute::{NormalizedRequest, process_receipt, receipt};
use crate::paths::{public_path_buf, public_path_display};
use crate::runner::{CommandSpec, ProcessOutput, ProcessRunner};
use crate::snapshot::InputSnapshot;
use plan::{GraphPlan, GraphReader, PendingCreate, PlanKey};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) fn is_attach(operation: BeadOperation) -> bool {
    matches!(
        operation,
        BeadOperation::PreviewAttach | BeadOperation::Attach
    )
}

pub(crate) fn execute(
    request: &BeadComposeRequest,
    runner: &dyn ProcessRunner,
    normalized: &NormalizedRequest,
    formula_input: &InputSnapshot,
    bd: PathBuf,
    staging: (
        Vec<BeadStageReceipt>,
        Option<crate::revision::UncommittedPolicy>,
        bool,
    ),
    diagnostics: &mut dyn FnMut(&BeadComposeError),
) -> Result<BeadComposeReceipt, BeadComposeError> {
    let (stages, policy, compare_only) = staging;
    let attach = is_attach(request.operation);
    let stage = operation_stage(request.operation);
    let mut runtime = Runtime {
        runner,
        normalized,
        bd,
        stage: if attach { BeadStage::Validate } else { stage },
        stages,
        policy,
        compare_only,
    };
    let result = match graph_plan_path(&runtime) {
        Ok(_) => run(
            request,
            normalized.advanced,
            &mut runtime,
            formula_input,
            stage,
        ),
        Err(error) => {
            runtime.stage = stage;
            Err(error)
        }
    };
    let missing_edges = match &result {
        Err(BeadComposeError::GraphEdgeMissing { edges }) => edges.clone(),
        _ => Vec::new(),
    };
    let (graph, outcome) = match result {
        Ok(graph) => (Some(graph), BeadOutcome::Succeeded),
        Err(error) => {
            if policy.is_some()
                && matches!(
                    error,
                    BeadComposeError::GraphPlanChanged { .. }
                        | BeadComposeError::GraphDirty
                        | BeadComposeError::GraphDirtyUnknown { .. }
                        | BeadComposeError::GraphApplyPartial { .. }
                )
            {
                return Err(error);
            }
            if is_phase_r_process_error(&error) {
                return Err(error);
            }
            runtime.record_error(&error);
            diagnostics(&error);
            (None, classify(&error))
        }
    };
    let mut result = receipt(
        request,
        normalized.rendered_formula.clone(),
        runtime.stages,
        outcome,
    );
    result.graph = graph;
    result.missing_edges = missing_edges;
    result.pour_mode = (graph_mode_is_pour(request)).then_some(BeadPourMode::Graph);
    restore_public_formula_paths(&mut result, formula_input, normalized);
    Ok(result)
}

fn operation_stage(operation: BeadOperation) -> BeadStage {
    match operation {
        BeadOperation::PreviewAttach => BeadStage::PreviewAttach,
        BeadOperation::Attach => BeadStage::Attach,
        BeadOperation::Repour | BeadOperation::PreviewRepour => BeadStage::Repour,
        BeadOperation::PreviewPour => BeadStage::PreviewPour,
        _ => BeadStage::Pour,
    }
}

/// An execution failure is `Failed`; every other graph error is a refusal.
fn classify(error: &BeadComposeError) -> BeadOutcome {
    let code = error.code().to_owned();
    if matches!(
        error,
        BeadComposeError::GraphReadFailed { .. }
            | BeadComposeError::GraphApplyFailed { .. }
            | BeadComposeError::GraphApplyPartial { .. }
            | BeadComposeError::GraphApplyUnconfirmed { .. }
            | BeadComposeError::CookFailed { .. }
            | BeadComposeError::RenderFailed { .. }
    ) {
        BeadOutcome::Failed { code }
    } else {
        BeadOutcome::Refused { code }
    }
}

fn restore_public_formula_paths(
    result: &mut BeadComposeReceipt,
    formula_input: &InputSnapshot,
    normalized: &NormalizedRequest,
) {
    let snapshot = public_path_display(formula_input.path());
    let source = public_path_display(&normalized.rendered_formula);
    for stage in &mut result.stages {
        for argument in &mut stage.argv {
            if argument == snapshot.as_str() {
                argument.clone_from(&source);
            }
        }
        stage.stderr_excerpt = stage.stderr_excerpt.replace(snapshot.as_str(), &source);
        stage.stdout_excerpt = stage.stdout_excerpt.replace(snapshot.as_str(), &source);
    }
}

fn graph_mode_is_pour(request: &BeadComposeRequest) -> bool {
    !(is_attach(request.operation)
        || matches!(
            request.operation,
            BeadOperation::Repour | BeadOperation::PreviewRepour
        ) && request.ref_.is_some())
}

fn is_phase_r_process_error(error: &BeadComposeError) -> bool {
    // An output outside `working_directory` is a request error for every
    // operation, never a receipt.
    matches!(
        error,
        BeadComposeError::BdUnavailable { .. }
            | BeadComposeError::ProcessArgumentInvalid { .. }
            | BeadComposeError::ProcessOutputLimitExceeded { .. }
            | BeadComposeError::OutputOutsideWorkingDirectory { .. }
    )
}

fn run(
    request: &BeadComposeRequest,
    advanced: bool,
    runtime: &mut Runtime<'_>,
    formula_input: &InputSnapshot,
    stage: BeadStage,
) -> Result<BeadGraph, BeadComposeError> {
    validate::scope(request)?;
    let rendered = formula_input.read()?;
    // Validate UTF-8 before handing bytes to bd's parser.
    validate::digest(&rendered)?;
    let args = vec![
        "cook".into(),
        public_path_display(formula_input.path()),
        "--json".into(),
    ];
    let output = runtime.invoke(args)?;
    // Map the private snapshot path to its public source before the typed
    // cause exists; the raw output is only parsed, never reported.
    let snapshot = public_path_display(formula_input.path());
    let source = public_path_display(&runtime.normalized.rendered_formula);
    if output.exit_status != Some(0) {
        let mut presented = output.clone();
        presented.stdout = presented.stdout.replace(snapshot.as_str(), &source);
        presented.stderr = presented.stderr.replace(snapshot.as_str(), &source);
        return Err(BeadComposeError::CookFailed {
            exit_status: output.exit_status,
            cause: process_failure_cause(&presented),
        });
    }
    let cooked =
        serde_json::from_str(&output.stdout).map_err(|error| BeadComposeError::CookFailed {
            exit_status: output.exit_status,
            cause: short_cause(&error.to_string().replace(snapshot.as_str(), &source)),
        })?;
    let mode = if is_attach(request.operation)
        || (matches!(
            request.operation,
            BeadOperation::Repour | BeadOperation::PreviewRepour
        ) && request.ref_.is_some())
    {
        BeadGraphMode::Attach
    } else {
        BeadGraphMode::Pour
    };
    let validated = validate::validate(request, cooked, &rendered, mode)?;
    runtime.stage = stage;
    // A repour always carries ownership provenance v2; it never takes bd's
    // missing-edge repair route.
    let advanced = advanced
        || matches!(
            request.operation,
            BeadOperation::Repour | BeadOperation::PreviewRepour
        );
    execute_revision_plan(request.operation, &validated, advanced, runtime)
}

fn execute_revision_plan(
    operation: BeadOperation,
    validated: &validate::ValidatedGraph,
    advanced: bool,
    runtime: &mut Runtime<'_>,
) -> Result<BeadGraph, BeadComposeError> {
    let plan = plan::plan(validated, advanced, runtime).map_err(|error| {
        if runtime.compare_only
            && let BeadComposeError::GraphEdgeConflict {
                from,
                to,
                existing,
                requested,
            } = error
        {
            return BeadComposeError::GraphPlanChanged {
                reason: crate::GraphConflictReason::PlanChanged,
                changes: vec![crate::GraphChange::EdgeTypeChange {
                    from,
                    to,
                    before: existing,
                    after: requested,
                }],
            };
        }
        error
    })?;
    check_revision_preflight(operation, &plan, runtime)?;
    let graph = execute_plan(operation, plan, runtime)?;
    if operation == BeadOperation::Repour && changed(&graph) {
        confirm_readback(validated, advanced, &graph, runtime).map_err(|error| {
            BeadComposeError::GraphApplyPartial {
                details: crate::GraphApplyPartial {
                    applied: Vec::new(),
                    pending: graph
                        .changes
                        .iter()
                        .filter(|change| {
                            !matches!(change, crate::GraphChange::RetainExternalEdge { .. })
                        })
                        .cloned()
                        .collect(),
                    failed_operation:
                        "read-back (write outcomes are uncertain; reconcile before retry)".into(),
                    cause: error.to_string(),
                },
            }
        })?;
    }
    Ok(graph)
}
fn check_revision_preflight(
    operation: BeadOperation,
    plan: &GraphPlan,
    runtime: &mut Runtime<'_>,
) -> Result<(), BeadComposeError> {
    if runtime.compare_only {
        let graph = match plan {
            GraphPlan::Noop(graph) => graph,
            GraphPlan::Create(pending) => &pending.graph,
        };
        let changes: Vec<_> = graph
            .changes
            .iter()
            .filter(|change| !matches!(change, crate::GraphChange::RetainExternalEdge { .. }))
            .cloned()
            .collect();
        if !changes.is_empty() {
            return Err(BeadComposeError::GraphPlanChanged {
                reason: crate::GraphConflictReason::PlanChanged,
                changes,
            });
        }
    } else if matches!(
        operation,
        BeadOperation::Repour | BeadOperation::PreviewRepour
    ) {
        check_dirty(runtime)?;
    }
    Ok(())
}

/// Probe the entire database via bd; unsupported or malformed output is unknown.
fn check_dirty(runtime: &mut Runtime<'_>) -> Result<(), BeadComposeError> {
    if runtime.policy == Some(crate::revision::UncommittedPolicy::Allow) {
        return Ok(());
    }
    let output = runtime
        .invoke(vec![
            "sql".into(),
            "--readonly".into(),
            "SELECT table_name, staged, status FROM dolt_status".into(),
            "--json".into(),
        ])
        .map_err(|error| BeadComposeError::GraphDirtyUnknown {
            cause: error.to_string(),
        })?;
    if output.exit_status != Some(0) {
        return Err(BeadComposeError::GraphDirtyUnknown {
            cause: process_failure_cause(&output),
        });
    }
    let rows: Vec<Value> = serde_json::from_str(&output.stdout).map_err(|error| {
        BeadComposeError::GraphDirtyUnknown {
            cause: error.to_string(),
        }
    })?;
    if rows.is_empty() {
        Ok(())
    } else {
        Err(BeadComposeError::GraphDirty)
    }
}

fn execute_plan(
    operation: BeadOperation,
    plan: GraphPlan,
    runtime: &mut Runtime<'_>,
) -> Result<BeadGraph, BeadComposeError> {
    match plan {
        GraphPlan::Noop(graph) => Ok(graph),
        GraphPlan::Create(pending) => {
            if matches!(
                operation,
                BeadOperation::PreviewAttach
                    | BeadOperation::PreviewPour
                    | BeadOperation::PreviewRepour
            ) {
                pending.preview(runtime)
            } else {
                pending.apply(runtime)
            }
        }
    }
}

/// Say how far a repour got when one of its writes failed.
///
/// Only a repour writes more than once, so a single-write apply is reported
/// exactly as before.
fn with_progress(error: BeadComposeError, applied: usize, total: usize) -> BeadComposeError {
    match error {
        BeadComposeError::GraphApplyFailed {
            command,
            status,
            cause,
        } if total > 1 => BeadComposeError::GraphApplyFailed {
            command,
            status,
            cause: format!(
                "{cause}; applied {applied} of {total} writes, {} pending; rerun the same command to finish",
                total - applied
            ),
        },
        error => error,
    }
}

fn take_changes(
    pending: &mut Vec<crate::GraphChange>,
    applied: &mut Vec<crate::GraphChange>,
    predicate: impl Fn(&crate::GraphChange) -> bool,
) {
    let mut remaining = Vec::new();
    for change in pending.drain(..) {
        if predicate(&change) {
            applied.push(change);
        } else {
            remaining.push(change);
        }
    }
    *pending = remaining;
}

fn partial_error(
    error: BeadComposeError,
    applied: &[crate::GraphChange],
    pending: &[crate::GraphChange],
    command: &[String],
    count: usize,
    total: usize,
    runtime: &Runtime<'_>,
) -> BeadComposeError {
    if runtime.stage != BeadStage::Repour {
        return with_progress(error, count, total);
    }
    BeadComposeError::GraphApplyPartial {
        details: crate::GraphApplyPartial {
            applied: applied.to_vec(),
            pending: pending.to_vec(),
            failed_operation: format!("{command:?} (outcome uncertain; reconcile before retry)"),
            cause: error.to_string(),
        },
    }
}

/// Run one bd write, reporting a non-zero exit as an apply failure.
fn write(runtime: &mut Runtime<'_>, args: Vec<String>) -> Result<(), BeadComposeError> {
    let output = runtime.invoke(args)?;
    if output.exit_status == Some(0) {
        return Ok(());
    }
    Err(BeadComposeError::GraphApplyFailed {
        command: runtime
            .stages
            .last()
            .map_or_else(Vec::new, |s| s.argv.clone()),
        status: output.exit_status,
        cause: short_cause(&process_failure_cause(&output)),
    })
}

/// Whether an applied graph wrote anything.
fn changed(graph: &BeadGraph) -> bool {
    graph.nodes.iter().any(|node| {
        matches!(
            node.action,
            BeadNodeAction::Created | BeadNodeAction::Updated
        )
    }) || graph.changes.iter().any(|change| {
        matches!(
            change,
            crate::GraphChange::RemoveInternalEdge { .. }
                | crate::GraphChange::MarkRemovedNode { .. }
        )
    }) || graph
        .edges
        .iter()
        .any(|edge| edge.action == BeadEdgeAction::Added)
}

/// Read the stored graph back and require that nothing is left to apply.
fn confirm_readback(
    validated: &validate::ValidatedGraph,
    advanced: bool,
    graph: &BeadGraph,
    runtime: &mut Runtime<'_>,
) -> Result<(), BeadComposeError> {
    let readback = plan::plan(validated, advanced, runtime).map_err(|error| match error {
        BeadComposeError::GraphReadFailed {
            command,
            status,
            cause,
        } => BeadComposeError::GraphApplyFailed {
            command,
            status,
            cause: format!("read-back failed after writes: {cause}"),
        },
        error => error,
    })?;
    match readback {
        GraphPlan::Noop(_) => Ok(()),
        GraphPlan::Create(_) => Err(BeadComposeError::GraphApplyUnconfirmed {
            command: Vec::new(),
            cause: "read-back still differs from the plan after apply".into(),
            ids: graph
                .parent
                .iter()
                .chain(graph.ids.values())
                .cloned()
                .collect(),
        }),
    }
}

struct Runtime<'a> {
    runner: &'a dyn ProcessRunner,
    normalized: &'a NormalizedRequest,
    bd: PathBuf,
    stage: BeadStage,
    stages: Vec<BeadStageReceipt>,
    policy: Option<crate::revision::UncommittedPolicy>,
    compare_only: bool,
}

impl Runtime<'_> {
    fn invoke(&mut self, args: Vec<String>) -> Result<ProcessOutput, BeadComposeError> {
        self.invoke_presenting(args, None)
    }

    fn invoke_presenting(
        &mut self,
        args: Vec<String>,
        source: Option<(&Path, &Path)>,
    ) -> Result<ProcessOutput, BeadComposeError> {
        let present = |text: &str| {
            source.map_or_else(
                || text.to_owned(),
                |(private, public)| {
                    text.replace(
                        private.to_string_lossy().as_ref(),
                        public.to_string_lossy().as_ref(),
                    )
                },
            )
        };
        let spec = CommandSpec {
            executable: self.bd.clone(),
            args,
            working_directory: self.normalized.working_directory.clone(),
        };
        let attempted = self.runner.run_graph(&spec);
        let mut presented_spec = spec.clone();
        for argument in &mut presented_spec.args {
            *argument = present(argument);
        }
        if let Err(error) = &attempted {
            self.stages.push(process_receipt(
                self.stage,
                &presented_spec,
                &ProcessOutput {
                    exit_status: None,
                    stdout: String::new(),
                    stderr: present(&error.to_string()),
                    elapsed: Duration::ZERO,
                },
                BeadStageOutcome::Succeeded,
            ));
        }
        let output = attempted.map_err(|error| {
            if let Some(limit_bytes) = crate::runner::output_limit_bytes(&error) {
                BeadComposeError::ProcessOutputLimitExceeded {
                    stage: self.stage,
                    limit_bytes,
                }
            } else if error.kind() == std::io::ErrorKind::InvalidInput {
                BeadComposeError::ProcessArgumentInvalid {
                    executable: self.bd.clone(),
                    message: present(&error.to_string()),
                }
            } else {
                BeadComposeError::BdUnavailable {
                    executable: self.bd.clone(),
                }
            }
        })?;
        // Normalize complete streams before receipt excerpts truncate them,
        // retaining the original stdout for semantic response parsing.
        let mut presented_output = output.clone();
        presented_output.stdout = present(&output.stdout);
        presented_output.stderr = present(&output.stderr);
        self.stages.push(process_receipt(
            self.stage,
            &presented_spec,
            &presented_output,
            BeadStageOutcome::Succeeded,
        ));
        Ok(output)
    }

    fn read_error(&self, status: Option<i32>, cause: &str) -> BeadComposeError {
        BeadComposeError::GraphReadFailed {
            command: self.stages.last().map_or_else(Vec::new, |s| s.argv.clone()),
            status,
            cause: short_cause(cause),
        }
    }

    fn record_error(&mut self, error: &BeadComposeError) {
        if self.stages.last().is_none_or(|s| s.stage != self.stage) {
            self.stages.push(process_receipt(
                self.stage,
                &CommandSpec {
                    executable: self.bd.clone(),
                    args: Vec::new(),
                    working_directory: self.normalized.working_directory.clone(),
                },
                &ProcessOutput {
                    exit_status: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    elapsed: Duration::ZERO,
                },
                BeadStageOutcome::Succeeded,
            ));
            self.stages
                .last_mut()
                .expect("just added stage")
                .argv
                .clear();
        }
        let stage = self.stages.last_mut().expect("error stage exists");
        stage.outcome = BeadStageOutcome::Failed {
            code: error.code().into(),
        };
        if !stage.stderr_excerpt.is_empty() {
            stage.stderr_excerpt.push('\n');
        }
        stage.stderr_excerpt.push_str(&error.to_string());
    }
}

// This is the sole recognition point for bd's all-ids-missing response.
fn all_missing(output: &ProcessOutput) -> bool {
    output.exit_status == Some(1)
        && serde_json::from_str::<Value>(&output.stdout)
            .ok()
            .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_owned))
            .is_some_and(|s| s == "no issues found matching the provided IDs")
}

fn process_failure_cause(output: &ProcessOutput) -> String {
    let diagnostic = if output.stderr.trim().is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    if let Ok(value) = serde_json::from_str::<Value>(diagnostic)
        && let Some(cause) = value.get("error").and_then(Value::as_str)
    {
        return short_cause(cause);
    }
    short_cause(diagnostic)
}

impl GraphReader for Runtime<'_> {
    fn issues(&mut self, ids: &[BeadId]) -> Result<BTreeMap<BeadId, Value>, BeadComposeError> {
        // bd prints a not-found diagnostic for each absent id. Keep every
        // show invocation bounded while retaining the existing stream caps.
        const SHOW_BATCH_SIZE: usize = 64;
        let mut found = BTreeMap::new();
        for ids in ids.chunks(SHOW_BATCH_SIZE) {
            let mut args = vec!["show".into(), "--json".into()];
            args.push("--".into());
            args.extend(ids.iter().map(ToString::to_string));
            let output = self.invoke(args)?;
            if all_missing(&output) {
                continue;
            }
            if output.exit_status != Some(0) {
                return Err(self.read_error(output.exit_status, &process_failure_cause(&output)));
            }
            let rows: Vec<Value> = serde_json::from_str(&output.stdout).map_err(|error| {
                self.read_error(output.exit_status, &short_cause(&error.to_string()))
            })?;
            for row in rows {
                let id = row
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|s| BeadId::new(s).ok())
                    .ok_or_else(|| {
                        self.read_error(
                            output.exit_status,
                            "bd show returned a row without a valid id",
                        )
                    })?;
                if !ids.contains(&id) || found.insert(id, row).is_some() {
                    return Err(self.read_error(
                        output.exit_status,
                        "bd show returned an unexpected or duplicate issue id",
                    ));
                }
            }
        }
        Ok(found)
    }

    fn children(&mut self, root: &BeadId) -> Result<BTreeMap<BeadId, Value>, BeadComposeError> {
        let output = self.invoke(vec![
            "children".into(),
            "--json".into(),
            "--".into(),
            root.to_string(),
        ])?;
        if output.exit_status != Some(0) {
            return Err(self.read_error(output.exit_status, &process_failure_cause(&output)));
        }
        let rows: Vec<Value> = serde_json::from_str(&output.stdout).map_err(|error| {
            self.read_error(output.exit_status, &short_cause(&error.to_string()))
        })?;
        rows.into_iter()
            .map(|row| {
                let id = row
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|s| BeadId::new(s).ok())
                    .ok_or_else(|| {
                        self.read_error(
                            output.exit_status,
                            "bd children returned a row without a valid id",
                        )
                    })?;
                Ok((id, row))
            })
            .collect()
    }

    fn dependents(
        &mut self,
        id: &BeadId,
    ) -> Result<Vec<(BeadId, GraphDependencyType)>, BeadComposeError> {
        self.dependencies_in_direction(id, "up")
    }

    fn dependencies(
        &mut self,
        id: &BeadId,
    ) -> Result<Vec<(BeadId, GraphDependencyType)>, BeadComposeError> {
        self.dependencies_in_direction(id, "down")
    }
}

impl Runtime<'_> {
    fn dependencies_in_direction(
        &mut self,
        id: &BeadId,
        direction: &str,
    ) -> Result<Vec<(BeadId, GraphDependencyType)>, BeadComposeError> {
        let mut args = vec!["dep".into(), "list".into()];
        if self.stage == BeadStage::Repour || direction == "up" {
            args.push(format!("--direction={direction}"));
        }
        args.extend(["--json".into(), "--".into(), id.to_string()]);
        let output = self.invoke(args)?;
        if output.exit_status != Some(0) {
            return Err(self.read_error(output.exit_status, &process_failure_cause(&output)));
        }
        let rows: Vec<Value> = serde_json::from_str(&output.stdout).map_err(|error| {
            self.read_error(output.exit_status, &short_cause(&error.to_string()))
        })?;
        rows.into_iter()
            .map(|row| {
                let id = row
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|s| BeadId::new(s).ok())
                    .ok_or_else(|| {
                        self.read_error(output.exit_status, "bd dep list returned an invalid id")
                    })?;
                let kind = row
                    .get("dependency_type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        self.read_error(
                            output.exit_status,
                            "bd dep list returned a row without dependency_type",
                        )
                    })?;
                let kind = GraphDependencyType::try_from(kind.to_owned()).map_err(|error| {
                    self.read_error(output.exit_status, &short_cause(&error.to_string()))
                })?;
                Ok((id, kind))
            })
            .collect()
    }
}

fn graph_plan_path(runtime: &Runtime<'_>) -> Result<(PathBuf, PathBuf), BeadComposeError> {
    let mut path = runtime
        .normalized
        .rendered_formula
        .as_os_str()
        .to_os_string();
    path.push(".graph.json");
    let path = PathBuf::from(path);
    let public_path = public_path_buf(&path);
    let parent = path
        .parent()
        .and_then(|p| fs::canonicalize(p).ok())
        .ok_or_else(|| BeadComposeError::GraphPlanPathInvalid {
            path: public_path.clone(),
            rule: "output parent must exist and be accessible".into(),
        })?;
    if !parent.starts_with(&runtime.normalized.working_directory) {
        return Err(BeadComposeError::OutputOutsideWorkingDirectory { path: public_path });
    }
    crate::render::validate_output_destination(&path).map_err(graph_plan_path_error)?;
    Ok((path, public_path))
}

fn graph_plan_path_error(error: BeadComposeError) -> BeadComposeError {
    match error {
        BeadComposeError::OutputPathInvalid { path, rule } => {
            BeadComposeError::GraphPlanPathInvalid { path, rule }
        }
        BeadComposeError::OutputPathSymlink { path } => BeadComposeError::GraphPlanPathInvalid {
            path,
            rule: "graph plan destination must not be a symbolic link".into(),
        },
        error => error,
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    use std::io;

    #[cfg(unix)]
    #[test]
    fn graph_plan_symlink_error_has_plan_path_diagnostic() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "sc-graph-plan-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target");
        let path = root.join("formula.formula.toml.graph.json");
        fs::write(&target, "untouched").unwrap();
        symlink(&target, &path).unwrap();

        let error = crate::render::validate_output_destination(&path)
            .map_err(graph_plan_path_error)
            .unwrap_err();
        let diagnostic = crate::BeadDiagnostic::refused(&error).unwrap();
        let wire = serde_json::to_value(diagnostic).unwrap();
        assert_eq!(wire["code"], "BEADS_OUTPUT_PATH_INVALID");
        assert_eq!(wire["details"]["field"], "graph_plan");
        assert_eq!(
            wire["recovery"],
            "Choose a writable graph plan path beside rendered_formula."
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "untouched");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn graph_plan_presentation_does_not_change_raw_response() {
        struct Response;
        impl ProcessRunner for Response {
            fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
                Ok(ProcessOutput {
                    exit_status: Some(0),
                    stdout: format!("{{\"source\":\"{}\"}}", spec.args[2]),
                    stderr: spec.args[2].clone(),
                    elapsed: Duration::ZERO,
                })
            }
        }
        let private = Path::new("/work/.sc-compose-input-123.json");
        let public = Path::new("/work/sample.formula.toml.graph.json");
        let normalized = NormalizedRequest {
            working_directory: "/work".into(),
            template: "/work/sample.formula.toml.j2".into(),
            rendered_formula: "/work/sample.formula.toml".into(),
            advanced: false,
        };
        let mut runtime = Runtime {
            runner: &Response,
            normalized: &normalized,
            bd: "fake-bd".into(),
            stage: BeadStage::Attach,
            stages: Vec::new(),
            policy: None,
            compare_only: false,
        };
        let output = runtime
            .invoke_presenting(
                vec![
                    "create".into(),
                    "--graph".into(),
                    private.to_string_lossy().into_owned(),
                ],
                Some((private, public)),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&output.stdout).unwrap()["source"],
            private.to_string_lossy().as_ref()
        );
        assert_eq!(output.stderr, private.to_string_lossy());
        let wire = serde_json::to_string(&runtime.stages).unwrap();
        assert!(wire.contains(public.to_string_lossy().as_ref()));
        assert!(!wire.contains(".sc-compose-input-"));
    }

    #[test]
    fn graph_plan_invoke_error_preserves_public_stage_and_typed_message() {
        struct ArgumentFailure;
        impl ProcessRunner for ArgumentFailure {
            fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
                assert!(spec.args[2].contains(".sc-compose-input-"));
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid input {}", spec.args[2]),
                ))
            }
        }
        let private = Path::new("/work/.sc-compose-input-123.json");
        let public = Path::new("/work/sample.formula.toml.graph.json");
        let normalized = NormalizedRequest {
            working_directory: "/work".into(),
            template: "/work/sample.formula.toml.j2".into(),
            rendered_formula: "/work/sample.formula.toml".into(),
            advanced: false,
        };
        let mut runtime = Runtime {
            runner: &ArgumentFailure,
            normalized: &normalized,
            bd: "fake-bd".into(),
            stage: BeadStage::Attach,
            stages: Vec::new(),
            policy: None,
            compare_only: false,
        };
        let error = runtime
            .invoke_presenting(
                vec![
                    "create".into(),
                    "--graph".into(),
                    private.to_string_lossy().into_owned(),
                ],
                Some((private, public)),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            BeadComposeError::ProcessArgumentInvalid { .. }
        ));
        let wire = serde_json::to_string(&(error, runtime.stages)).unwrap();
        assert!(wire.contains(public.to_string_lossy().as_ref()), "{wire}");
        assert!(!wire.contains(".sc-compose-input-"), "{wire}");
    }
}
