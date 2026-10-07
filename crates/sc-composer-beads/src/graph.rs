//! Shared read/validate/plan/apply engine for attach and by-path pour.

mod plan;
mod validate;

use crate::contract::{
    BeadComposeReceipt, BeadComposeRequest, BeadEdgeAction, BeadGraph, BeadGraphMode, BeadId,
    BeadNodeAction, BeadOperation, BeadOutcome, BeadPourMode, BeadStage, BeadStageOutcome,
    BeadStageReceipt, GraphDependencyType,
};
use crate::error::{BeadComposeError, short_cause};
use crate::execute::{NormalizedRequest, process_receipt, receipt};
use crate::runner::{CommandSpec, ProcessOutput, ProcessRunner};
use plan::{GraphPlan, GraphReader, PendingCreate, PlanKey};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
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
    bd: PathBuf,
    stages: Vec<BeadStageReceipt>,
) -> Result<BeadComposeReceipt, BeadComposeError> {
    let attach = is_attach(request.operation);
    let stage = match request.operation {
        BeadOperation::PreviewAttach => BeadStage::PreviewAttach,
        BeadOperation::Attach => BeadStage::Attach,
        BeadOperation::PreviewPour => BeadStage::PreviewPour,
        _ => BeadStage::Pour,
    };
    let mut runtime = Runtime {
        runner,
        normalized,
        bd,
        stage: if attach { BeadStage::Validate } else { stage },
        stages,
    };
    let result = run(request, &mut runtime, stage);
    let missing_edges = match &result {
        Err(BeadComposeError::GraphEdgeMissing { edges }) => edges.clone(),
        _ => Vec::new(),
    };
    let (graph, outcome) = match result {
        Ok(graph) => (Some(graph), BeadOutcome::Succeeded),
        Err(error) => {
            if is_phase_r_process_error(&error) {
                return Err(error);
            }
            runtime.record_error(&error);
            let code = error.code().to_owned();
            let failed = matches!(
                error,
                BeadComposeError::GraphReadFailed { .. }
                    | BeadComposeError::GraphApplyFailed { .. }
                    | BeadComposeError::CookFailed { .. }
                    | BeadComposeError::RenderFailed { .. }
            );
            (
                None,
                if failed {
                    BeadOutcome::Failed { code }
                } else {
                    BeadOutcome::Refused { code }
                },
            )
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
    result.pour_mode = (!attach).then_some(BeadPourMode::Graph);
    Ok(result)
}

fn is_phase_r_process_error(error: &BeadComposeError) -> bool {
    matches!(
        error,
        BeadComposeError::BdUnavailable { .. }
            | BeadComposeError::ProcessArgumentInvalid { .. }
            | BeadComposeError::ProcessOutputLimitExceeded { .. }
    )
}

fn run(
    request: &BeadComposeRequest,
    runtime: &mut Runtime<'_>,
    stage: BeadStage,
) -> Result<BeadGraph, BeadComposeError> {
    validate::scope(request)?;
    let rendered = fs::read(&runtime.normalized.rendered_formula).map_err(|e| {
        BeadComposeError::RenderFailed {
            message: e.to_string(),
        }
    })?;
    // Validate UTF-8 before handing bytes to bd's parser.
    validate::digest(&rendered)?;
    let args = vec![
        "cook".into(),
        runtime
            .normalized
            .rendered_formula
            .to_string_lossy()
            .into_owned(),
        "--json".into(),
    ];
    let output = runtime.invoke(args)?;
    if output.exit_status != Some(0) {
        return Err(BeadComposeError::CookFailed {
            exit_status: output.exit_status,
            cause: process_failure_cause(&output),
        });
    }
    let cooked =
        serde_json::from_str(&output.stdout).map_err(|error| BeadComposeError::CookFailed {
            exit_status: output.exit_status,
            cause: short_cause(&error.to_string()),
        })?;
    let mode = if is_attach(request.operation) {
        BeadGraphMode::Attach
    } else {
        BeadGraphMode::Pour
    };
    let validated = validate::validate(request, cooked, &rendered, mode)?;
    runtime.stage = stage;
    match plan::plan(&validated, runtime)? {
        GraphPlan::Noop(graph) => Ok(graph),
        GraphPlan::Create(pending) => {
            if matches!(
                request.operation,
                BeadOperation::PreviewAttach | BeadOperation::PreviewPour
            ) {
                pending.preview(runtime)
            } else {
                pending.apply(runtime)
            }
        }
    }
}

struct Runtime<'a> {
    runner: &'a dyn ProcessRunner,
    normalized: &'a NormalizedRequest,
    bd: PathBuf,
    stage: BeadStage,
    stages: Vec<BeadStageReceipt>,
}

impl Runtime<'_> {
    fn invoke(&mut self, args: Vec<String>) -> Result<ProcessOutput, BeadComposeError> {
        let spec = CommandSpec {
            executable: self.bd.clone(),
            args,
            working_directory: self.normalized.working_directory.clone(),
        };
        let attempted = self.runner.run(&spec);
        if let Err(error) = &attempted {
            self.stages.push(process_receipt(
                self.stage,
                &spec,
                &ProcessOutput {
                    exit_status: None,
                    stdout: String::new(),
                    stderr: error.to_string(),
                    elapsed: Duration::ZERO,
                },
                BeadStageOutcome::Succeeded,
            ));
        }
        let output = attempted.map_err(|error| {
            if crate::runner::is_process_output_limit_error(&error) {
                BeadComposeError::ProcessOutputLimitExceeded {
                    stage: self.stage,
                    limit_bytes: crate::runner::PROCESS_OUTPUT_LIMIT_BYTES,
                }
            } else if error.kind() == std::io::ErrorKind::InvalidInput {
                BeadComposeError::ProcessArgumentInvalid {
                    executable: self.bd.clone(),
                    message: error.to_string(),
                }
            } else {
                BeadComposeError::BdUnavailable {
                    executable: self.bd.clone(),
                }
            }
        })?;
        self.stages.push(process_receipt(
            self.stage,
            &spec,
            &output,
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
    short_cause(diagnostic)
}

impl GraphReader for Runtime<'_> {
    fn issues(&mut self, ids: &[BeadId]) -> Result<BTreeMap<BeadId, Value>, BeadComposeError> {
        let mut args = vec!["show".into()];
        args.extend(ids.iter().map(ToString::to_string));
        args.push("--json".into());
        let output = self.invoke(args)?;
        if all_missing(&output) {
            return Ok(BTreeMap::new());
        }
        if output.exit_status != Some(0) {
            return Err(self.read_error(output.exit_status, &process_failure_cause(&output)));
        }
        let rows: Vec<Value> = serde_json::from_str(&output.stdout).map_err(|error| {
            self.read_error(output.exit_status, &short_cause(&error.to_string()))
        })?;
        let mut found = BTreeMap::new();
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
        Ok(found)
    }

    fn dependencies(
        &mut self,
        id: &BeadId,
    ) -> Result<Vec<(BeadId, GraphDependencyType)>, BeadComposeError> {
        let output = self.invoke(vec![
            "dep".into(),
            "list".into(),
            id.to_string(),
            "--json".into(),
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

impl PendingCreate {
    fn preview(self, runtime: &mut Runtime<'_>) -> Result<BeadGraph, BeadComposeError> {
        self.execute(runtime, true)
    }
    fn apply(self, runtime: &mut Runtime<'_>) -> Result<BeadGraph, BeadComposeError> {
        self.execute(runtime, false)
    }

    fn execute(
        mut self,
        runtime: &mut Runtime<'_>,
        preview: bool,
    ) -> Result<BeadGraph, BeadComposeError> {
        let mut path = runtime
            .normalized
            .rendered_formula
            .as_os_str()
            .to_os_string();
        path.push(".graph.json");
        let path = PathBuf::from(path);
        let parent = path
            .parent()
            .and_then(|p| fs::canonicalize(p).ok())
            .ok_or_else(|| BeadComposeError::TemplatePathInvalid { path: path.clone() })?;
        if !parent.starts_with(&runtime.normalized.working_directory) {
            return Err(BeadComposeError::OutputOutsideWorkingDirectory { path });
        }
        let bytes = serde_json::to_vec(&self.payload).expect("plan JSON serializes");
        crate::render::atomic_write(&path, &bytes)?;
        self.graph.plan_path = Some(path.clone());
        let mut args = vec![
            "create".into(),
            "--graph".into(),
            path.to_string_lossy().into_owned(),
        ];
        if preview {
            args.push("--dry-run".into());
        }
        args.push("--json".into());
        let command = CommandSpec {
            executable: runtime.bd.clone(),
            args: args.clone(),
            working_directory: runtime.normalized.working_directory.clone(),
        }
        .argv();
        let output = runtime.invoke(args)?;
        let failure = |cause: String| BeadComposeError::GraphApplyFailed {
            command: command.clone(),
            status: output.exit_status,
            cause,
        };
        if output.exit_status != Some(0) {
            return Err(failure(process_failure_cause(&output)));
        }
        if preview {
            return Ok(self.graph);
        }
        let value: Value = serde_json::from_str(&output.stdout)
            .map_err(|error| failure(short_cause(&error.to_string())))?;
        let assigned: BTreeMap<PlanKey, BeadId> = serde_json::from_value(
            value
                .get("ids")
                .cloned()
                .ok_or_else(|| failure("bd create --graph response is missing ids".to_owned()))?,
        )
        .map_err(|error| failure(short_cause(&error.to_string())))?;
        if assigned.len() != self.keys.len() || self.keys.keys().any(|k| !assigned.contains_key(k))
        {
            return Err(failure(
                "bd create --graph response ids do not match the planned steps".to_owned(),
            ));
        }
        for (key, step) in &self.keys {
            let id = assigned[key].clone();
            if let Some(step) = step {
                if self
                    .graph
                    .ids
                    .get(step)
                    .is_some_and(|expected| *expected != id)
                {
                    return Err(failure(
                        "bd create --graph assigned an id different from the planned id".to_owned(),
                    ));
                }
                self.graph.ids.insert(step.clone(), id);
            } else {
                self.graph.parent = Some(id);
            }
        }
        self.mark_created();
        Ok(self.graph)
    }

    fn mark_created(&mut self) {
        for node in &mut self.graph.nodes {
            if node.action == BeadNodeAction::Create {
                node.action = BeadNodeAction::Created;
                node.id = node
                    .step
                    .as_ref()
                    .and_then(|s| self.graph.ids.get(s))
                    .cloned()
                    .or_else(|| self.graph.parent.clone());
            }
        }
        let endpoints: Vec<_> = self
            .endpoints
            .iter()
            .map(|(from, to)| (from.resolve(&self.graph), to.resolve(&self.graph)))
            .collect();
        for (edge, (from, to)) in self.graph.edges.iter_mut().zip(endpoints) {
            edge.from = from;
            edge.to = to;
            if edge.action == BeadEdgeAction::Add {
                edge.action = BeadEdgeAction::Added;
            }
        }
    }
}
