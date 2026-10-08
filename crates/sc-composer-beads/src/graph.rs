//! Shared read/validate/plan/apply engine for attach and by-path pour.

mod plan;
mod validate;

use crate::contract::{
    BeadComposeReceipt, BeadComposeRequest, BeadEdgeAction, BeadGraph, BeadGraphMode, BeadId,
    BeadNodeAction, BeadOperation, BeadOutcome, BeadPourMode, BeadStage, BeadStageOutcome,
    BeadStageReceipt, GraphDependencyType,
};
use crate::error::{BeadComposeError, short_cause};
use crate::execute::{
    NormalizedRequest, process_receipt, public_path_buf, public_path_display, receipt,
};
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
    stages: Vec<BeadStageReceipt>,
    diagnostics: &mut dyn FnMut(&BeadComposeError),
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
    let result = run(request, &mut runtime, formula_input, stage);
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
            diagnostics(&error);
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
    Ok(result)
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

    fn dependencies(
        &mut self,
        id: &BeadId,
    ) -> Result<Vec<(BeadId, GraphDependencyType)>, BeadComposeError> {
        let output = self.invoke(vec![
            "dep".into(),
            "list".into(),
            "--json".into(),
            "--".into(),
            id.to_string(),
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
        let public_path = public_path_buf(&path);
        let parent = path
            .parent()
            .and_then(|p| fs::canonicalize(p).ok())
            .ok_or_else(|| BeadComposeError::OutputPathInvalid {
                path: public_path.clone(),
                rule: "output parent must exist and be accessible".into(),
            })?;
        if !parent.starts_with(&runtime.normalized.working_directory) {
            return Err(BeadComposeError::OutputOutsideWorkingDirectory { path: public_path });
        }
        crate::render::validate_output_destination(&path)?;
        let bytes = serde_json::to_vec(&self.payload).expect("plan JSON serializes");
        let plan_input = InputSnapshot::write(&path, &bytes)
            .map_err(|error| crate::snapshot::output_error(&path, error))?;
        plan_input.publish_copy(&path)?;
        self.graph.plan_path = Some(public_path.clone());
        let mut args = vec![
            "create".into(),
            "--graph".into(),
            plan_input.path().to_string_lossy().into_owned(),
        ];
        if preview {
            args.push("--dry-run".into());
        }
        args.push("--json".into());
        let mut presented_args = args.clone();
        presented_args[2] = public_path.to_string_lossy().into_owned();
        let command = CommandSpec {
            executable: runtime.bd.clone(),
            args: presented_args,
            working_directory: runtime.normalized.working_directory.clone(),
        }
        .argv();
        let output = runtime.invoke_presenting(args, Some((plan_input.path(), &public_path)))?;
        let present = |text: &str| {
            text.replace(
                plan_input.path().to_string_lossy().as_ref(),
                public_path.to_string_lossy().as_ref(),
            )
        };
        let failure = |cause: String| BeadComposeError::GraphApplyFailed {
            command: command.clone(),
            status: output.exit_status,
            cause: short_cause(&present(&cause)),
        };
        if output.exit_status != Some(0) {
            let mut presented_output = output.clone();
            presented_output.stdout = present(&output.stdout);
            presented_output.stderr = present(&output.stderr);
            return Err(failure(process_failure_cause(&presented_output)));
        }
        if preview {
            return Ok(self.graph);
        }
        let value: Value =
            serde_json::from_str(&output.stdout).map_err(|error| failure(error.to_string()))?;
        let assigned: BTreeMap<PlanKey, BeadId> = serde_json::from_value(
            value
                .get("ids")
                .cloned()
                .ok_or_else(|| failure("bd create --graph response is missing ids".to_owned()))?,
        )
        .map_err(|error| failure(error.to_string()))?;
        if assigned.len() != self.keys.len() || self.keys.keys().any(|k| !assigned.contains_key(k))
        {
            return Err(failure(
                "bd create --graph response ids do not match the planned steps".to_owned(),
            ));
        }
        for (key, step) in &self.keys {
            let id = assigned.get(key).cloned().ok_or_else(|| {
                failure("bd create --graph response is missing a planned key".to_owned())
            })?;
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
        self.mark_created()?;
        Ok(self.graph)
    }

    fn mark_created(&mut self) -> Result<(), BeadComposeError> {
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
            .map(|(from, to)| {
                let from = from.resolve(&self.graph).ok_or_else(|| {
                    BeadComposeError::GraphApplyFailed {
                        command: vec!["resolve created graph edge source".into()],
                        status: None,
                        cause: "planned edge source did not resolve".into(),
                    }
                })?;
                let to =
                    to.resolve(&self.graph)
                        .ok_or_else(|| BeadComposeError::GraphApplyFailed {
                            command: vec!["resolve created graph edge destination".into()],
                            status: None,
                            cause: "planned edge destination did not resolve".into(),
                        })?;
                Ok((from, to))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (edge, (from, to)) in self.graph.edges.iter_mut().zip(endpoints) {
            edge.from = from;
            edge.to = to;
            if edge.action == BeadEdgeAction::Add {
                edge.action = BeadEdgeAction::Added;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    use std::io;

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
        };
        let mut runtime = Runtime {
            runner: &Response,
            normalized: &normalized,
            bd: "fake-bd".into(),
            stage: BeadStage::Attach,
            stages: Vec::new(),
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
        };
        let mut runtime = Runtime {
            runner: &ArgumentFailure,
            normalized: &normalized,
            bd: "fake-bd".into(),
            stage: BeadStage::Attach,
            stages: Vec::new(),
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
