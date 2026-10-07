//! Render-to-`bd` operation staging.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::contract::{
    BEADS_SCHEMA_V1, BeadComposeReceipt, BeadComposeRequest, BeadOperation, BeadOutcome, BeadStage,
    BeadStageOutcome, BeadStageReceipt, PourAuthorization,
};
use crate::error::{BeadComposeError, short_cause};
use crate::render::{render_formula_in_root, validate_output_destination};
use crate::runner::{
    CommandSpec, PROCESS_OUTPUT_LIMIT_BYTES, ProcessOutput, ProcessRunner, StdProcessRunner,
    is_process_output_limit_error,
};
use crate::snapshot::InputSnapshot;

const OUTPUT_EXCERPT_LIMIT: usize = 16 * 1024;

/// Execute a Beads request through the production direct process runner.
///
/// # Errors
///
/// Returns a stable error for rejected request preconditions or an unavailable
/// executable. Process failures return a failed receipt with stage evidence.
pub fn execute_bead_request(
    request: &BeadComposeRequest,
) -> Result<BeadComposeReceipt, BeadComposeError> {
    execute_bead_request_with_runner(request, &StdProcessRunner)
}

/// Execute a request and report typed graph failures retained in its receipt.
///
/// Diagnostic callbacks receive borrowed errors before they become receipt
/// codes and stage excerpts. Receipt serialization is unchanged. Errors returned
/// directly from this function remain available through its `Result`.
///
/// # Errors
/// Returns the same request and process errors as [`execute_bead_request`].
pub fn execute_bead_request_with_diagnostics(
    request: &BeadComposeRequest,
    diagnostics: &mut dyn FnMut(&BeadComposeError),
) -> Result<BeadComposeReceipt, BeadComposeError> {
    execute_with_runner_and_diagnostics(request, &StdProcessRunner, diagnostics)
}

/// Execute a Beads request through an injected direct process runner.
///
/// # Errors
///
/// Returns a stable error for rejected request preconditions or an unavailable
/// executable. Process failures return a failed receipt with stage evidence.
pub fn execute_bead_request_with_runner(
    request: &BeadComposeRequest,
    runner: &dyn ProcessRunner,
) -> Result<BeadComposeReceipt, BeadComposeError> {
    execute_with_runner_and_diagnostics(request, runner, &mut |_| {})
}

#[allow(
    clippy::too_many_lines,
    reason = "The receipt-producing progression remains visible in one ordered function."
)]
fn execute_with_runner_and_diagnostics(
    request: &BeadComposeRequest,
    runner: &dyn ProcessRunner,
    diagnostics: &mut dyn FnMut(&BeadComposeError),
) -> Result<BeadComposeReceipt, BeadComposeError> {
    let normalized = validate_request(request)?;
    let mut stages = Vec::new();
    let render_started = Instant::now();
    let rendered = (|| {
        let input = InputSnapshot::reserve(&normalized.rendered_formula).map_err(|error| {
            if crate::graph::is_attach(request.operation) {
                crate::snapshot::output_error(&normalized.rendered_formula, error)
            } else {
                error
            }
        })?;
        render_formula_in_root(
            &normalized.template,
            input.path(),
            &request.compose_variables,
            &normalized.working_directory,
        )?;
        // Phase R consumes its named registry path. Graph operations retain
        // the private input until bd has read it, even when routing by path.
        if crate::graph::is_attach(request.operation) {
            input.publish_copy(&normalized.rendered_formula)?;
        } else if request.operation != BeadOperation::Render {
            crate::render::atomic_write(&normalized.rendered_formula, &input.read()?)?;
        }
        Ok::<_, BeadComposeError>(input)
    })();
    let formula_input = match rendered {
        Ok(input) => input,
        Err(error) => {
            stages.push(render_receipt(
                render_started,
                BeadStageOutcome::Failed {
                    code: error.code().to_owned(),
                },
                excerpt(&error.to_string()),
            ));
            return Ok(receipt(
                request,
                normalized.rendered_formula,
                stages,
                BeadOutcome::Failed {
                    code: error.code().to_owned(),
                },
            ));
        }
    };
    stages.push(render_receipt(
        render_started,
        BeadStageOutcome::Succeeded,
        String::new(),
    ));

    let destination = normalized.rendered_formula.clone();
    let result = execute_rendered_request(
        request,
        runner,
        normalized,
        &formula_input,
        stages,
        diagnostics,
    );
    if !crate::graph::is_attach(request.operation) {
        formula_input.publish(&destination)?;
    }
    result
}

fn execute_rendered_request(
    request: &BeadComposeRequest,
    runner: &dyn ProcessRunner,
    normalized: NormalizedRequest,
    formula_input: &InputSnapshot,
    mut stages: Vec<BeadStageReceipt>,
    diagnostics: &mut dyn FnMut(&BeadComposeError),
) -> Result<BeadComposeReceipt, BeadComposeError> {
    if request.operation == BeadOperation::Render {
        return Ok(receipt(
            request,
            normalized.rendered_formula,
            stages,
            BeadOutcome::Succeeded,
        ));
    }

    let bd = request
        .bd_executable
        .clone()
        .unwrap_or_else(|| PathBuf::from("bd"));
    if crate::graph::is_attach(request.operation) {
        return crate::graph::execute(
            request,
            runner,
            &normalized,
            formula_input,
            bd,
            stages,
            diagnostics,
        );
    }
    let cook_input = if matches!(
        request.operation,
        BeadOperation::PreviewPour | BeadOperation::Pour
    ) {
        formula_input.path()
    } else {
        normalized.rendered_formula.as_path()
    };
    let cook = CommandSpec {
        executable: bd.clone(),
        args: cook_args(cook_input, request),
        working_directory: normalized.working_directory.clone(),
    };
    let cook_result = run_stage(runner, StageFailure::Cook, &cook, &mut stages)?;
    present_snapshot_paths(
        &mut stages,
        formula_input.path(),
        &normalized.rendered_formula,
    );
    if let Some(failed) = cook_result {
        return Ok(receipt(
            request,
            normalized.rendered_formula,
            stages,
            failed,
        ));
    }
    if request.operation == BeadOperation::Validate {
        return Ok(receipt(
            request,
            normalized.rendered_formula,
            stages,
            BeadOutcome::Succeeded,
        ));
    }

    crate::pour::execute_pour(
        request,
        runner,
        normalized,
        formula_input,
        bd,
        stages,
        diagnostics,
    )
}

pub(crate) struct NormalizedRequest {
    pub(crate) working_directory: PathBuf,
    pub(crate) template: PathBuf,
    pub(crate) rendered_formula: PathBuf,
}

fn validate_request(request: &BeadComposeRequest) -> Result<NormalizedRequest, BeadComposeError> {
    if request.schema != BEADS_SCHEMA_V1 {
        return Err(BeadComposeError::UnknownSchema {
            actual: request.schema.clone(),
        });
    }
    if matches!(
        request.operation,
        BeadOperation::PreviewPour | BeadOperation::Pour
    ) && request.formula_name.is_none()
    {
        return Err(BeadComposeError::FormulaNameRequired);
    }
    if matches!(
        request.operation,
        BeadOperation::Pour | BeadOperation::Attach
    ) && request.pour_authorization != Some(PourAuthorization::CreatePersistentBeads)
    {
        return Err(BeadComposeError::PourAuthorizationRequired);
    }
    validate_request_shape(request)?;
    for key in request.bead_variables.keys() {
        if !valid_bead_key(key) {
            return Err(BeadComposeError::BeadVariableKeyInvalid { key: key.clone() });
        }
    }
    for (key, value) in &request.bead_variables {
        if value.contains('\0') {
            return Err(BeadComposeError::BeadVariableValueInvalid {
                key: key.clone(),
                value: value.clone(),
            });
        }
    }
    let working_directory = fs::canonicalize(&request.working_directory).map_err(|_error| {
        BeadComposeError::TemplatePathInvalid {
            path: request.working_directory.clone(),
        }
    })?;
    let template_path = if request.template.is_absolute() {
        request.template.clone()
    } else {
        working_directory.join(&request.template)
    };
    let template = fs::canonicalize(template_path).map_err(|_error| {
        BeadComposeError::TemplatePathInvalid {
            path: request.template.clone(),
        }
    })?;
    validate_utf8_path(&working_directory)?;
    validate_utf8_path(&template)?;
    if !template.is_file() {
        return Err(BeadComposeError::FormulaPathNotFile { path: template });
    }
    if !template.starts_with(&working_directory) {
        return Err(BeadComposeError::TemplateOutsideWorkingDirectory { path: template });
    }
    let rendered_formula = normalize_output(&request.rendered_formula)?;
    validate_utf8_path(&rendered_formula)?;
    if let Some(executable) = &request.bd_executable {
        validate_utf8_path(executable)?;
    }
    if !is_formula_path(&rendered_formula) {
        return Err(BeadComposeError::FormulaExtensionUnsupported {
            path: rendered_formula,
        });
    }
    if matches!(
        request.operation,
        BeadOperation::Render
            | BeadOperation::Validate
            | BeadOperation::PreviewAttach
            | BeadOperation::Attach
    ) && !rendered_formula.starts_with(&working_directory)
    {
        return Err(BeadComposeError::OutputOutsideWorkingDirectory {
            path: rendered_formula,
        });
    }
    validate_output_destination(&rendered_formula)?;
    Ok(NormalizedRequest {
        working_directory,
        template,
        rendered_formula,
    })
}

fn validate_request_shape(request: &BeadComposeRequest) -> Result<(), BeadComposeError> {
    let attach = crate::graph::is_attach(request.operation);
    let shape_error = if attach {
        if request.parent.is_none() {
            Some("attach requires parent")
        } else if request.ref_.is_none() {
            Some("attach requires ref")
        } else if !request.bead_variables.is_empty() {
            Some("attach forbids bead_variables")
        } else {
            None
        }
    } else {
        if request.parent.is_some() {
            Some("non-attach operations forbid parent")
        } else if request.ref_.is_some() {
            Some("non-attach operations forbid ref")
        } else if matches!(
            request.operation,
            BeadOperation::Render | BeadOperation::Validate
        ) && !request.relations.is_empty()
        {
            Some("render and validate forbid relations")
        } else {
            None
        }
    };
    if let Some(message) = shape_error {
        return Err(BeadComposeError::RequestDeserializationFailed {
            message: message.to_owned(),
        });
    }
    Ok(())
}

fn validate_utf8_path(path: &Path) -> Result<(), BeadComposeError> {
    if path.to_str().is_some() {
        Ok(())
    } else {
        Err(BeadComposeError::PathNotUtf8 { path: path.into() })
    }
}

fn normalize_output(path: &Path) -> Result<PathBuf, BeadComposeError> {
    let parent = path
        .parent()
        .ok_or_else(|| BeadComposeError::OutputPathInvalid {
            path: path.into(),
            rule: String::from("an existing parent directory and file name are required"),
        })?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        let rule = if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "parent directory `{}` for `{}` must exist",
                parent.display(),
                path.display()
            )
        } else {
            String::from("parent directory must be resolvable")
        };
        BeadComposeError::OutputPathInvalid {
            path: path.into(),
            rule,
        }
    })?;
    if !parent.is_dir() {
        return Err(BeadComposeError::OutputPathInvalid {
            path: path.into(),
            rule: format!("parent `{}` must be a directory", parent.display()),
        });
    }
    let name = path
        .file_name()
        .ok_or_else(|| BeadComposeError::OutputPathInvalid {
            path: path.into(),
            rule: String::from("path must include a file name"),
        })?;
    Ok(parent.join(name))
}

fn valid_bead_key(key: &str) -> bool {
    let mut chars = key.chars();
    matches!(chars.next(), Some(character) if character.is_ascii_alphabetic() || character == '_')
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn is_formula_path(path: &Path) -> bool {
    let value = path.to_string_lossy();
    value.ends_with(".formula.toml") || value.ends_with(".formula.json")
}

fn cook_args(rendered_formula: &Path, request: &BeadComposeRequest) -> Vec<String> {
    let mut args = vec![
        String::from("cook"),
        rendered_formula.to_string_lossy().into_owned(),
        String::from("--dry-run"),
        String::from("--json"),
    ];
    append_variables(&mut args, request);
    args
}

pub(crate) fn append_variables(args: &mut Vec<String>, request: &BeadComposeRequest) {
    for (key, value) in &request.bead_variables {
        args.push(String::from("--var"));
        args.push(format!("{key}={value}"));
    }
}

/// Closed mapping for process stages that return status-bearing failures.
#[derive(Clone, Copy)]
pub(crate) enum StageFailure {
    Cook,
    ResolveActiveRegistry,
    PreviewPour,
    Pour,
}

impl StageFailure {
    fn stage(self) -> BeadStage {
        match self {
            Self::Cook => BeadStage::Validate,
            Self::ResolveActiveRegistry => BeadStage::ResolveActiveRegistry,
            Self::PreviewPour => BeadStage::PreviewPour,
            Self::Pour => BeadStage::Pour,
        }
    }

    fn error(self, exit_status: Option<i32>, diagnostic: &str) -> BeadComposeError {
        match self {
            Self::Cook => BeadComposeError::CookFailed {
                exit_status,
                cause: short_cause(diagnostic),
            },
            Self::ResolveActiveRegistry => {
                BeadComposeError::ActiveRegistryResolutionFailed { exit_status }
            }
            Self::PreviewPour => BeadComposeError::PreviewPourFailed { exit_status },
            Self::Pour => BeadComposeError::PourFailed { exit_status },
        }
    }
}

pub(crate) fn run_stage(
    runner: &dyn ProcessRunner,
    failure: StageFailure,
    spec: &CommandSpec,
    stages: &mut Vec<BeadStageReceipt>,
) -> Result<Option<BeadOutcome>, BeadComposeError> {
    match run_stage_with_output(runner, failure, spec, stages)? {
        Ok(_) => Ok(None),
        Err(outcome) => Ok(Some(outcome)),
    }
}

pub(crate) fn run_stage_with_output(
    runner: &dyn ProcessRunner,
    failure: StageFailure,
    spec: &CommandSpec,
    stages: &mut Vec<BeadStageReceipt>,
) -> Result<Result<ProcessOutput, BeadOutcome>, BeadComposeError> {
    let stage = failure.stage();
    let output = runner.run(spec).map_err(|error| {
        if is_process_output_limit_error(&error) {
            BeadComposeError::ProcessOutputLimitExceeded {
                stage,
                limit_bytes: PROCESS_OUTPUT_LIMIT_BYTES,
            }
        } else if error.kind() == std::io::ErrorKind::InvalidInput {
            BeadComposeError::ProcessArgumentInvalid {
                executable: spec.executable.clone(),
                message: error.to_string(),
            }
        } else {
            BeadComposeError::BdUnavailable {
                executable: spec.executable.clone(),
            }
        }
    })?;
    if output.exit_status == Some(0) {
        stages.push(process_receipt(
            stage,
            spec,
            &output,
            BeadStageOutcome::Succeeded,
        ));
        Ok(Ok(output))
    } else {
        let diagnostic = if output.stderr.trim().is_empty() {
            &output.stdout
        } else {
            &output.stderr
        };
        let code = failure
            .error(output.exit_status, diagnostic)
            .code()
            .to_owned();
        stages.push(process_receipt(
            stage,
            spec,
            &output,
            BeadStageOutcome::Failed { code: code.clone() },
        ));
        Ok(Err(BeadOutcome::Failed { code }))
    }
}

fn present_snapshot_paths(stages: &mut [BeadStageReceipt], snapshot: &Path, source: &Path) {
    let snapshot = snapshot.to_string_lossy();
    let source = source.to_string_lossy().into_owned();
    for stage in stages {
        for argument in &mut stage.argv {
            if argument == snapshot.as_ref() {
                argument.clone_from(&source);
            }
        }
        stage.stderr_excerpt = stage
            .stderr_excerpt
            .replace(snapshot.as_ref(), source.as_ref());
        stage.stdout_excerpt = stage
            .stdout_excerpt
            .replace(snapshot.as_ref(), source.as_ref());
    }
}

pub(crate) fn receipt(
    request: &BeadComposeRequest,
    rendered_formula: PathBuf,
    stages: Vec<BeadStageReceipt>,
    outcome: BeadOutcome,
) -> BeadComposeReceipt {
    BeadComposeReceipt {
        schema: BEADS_SCHEMA_V1.to_owned(),
        operation: request.operation,
        rendered_formula,
        stages,
        outcome,
        pour_mode: None,
        graph: None,
        missing_edges: Vec::new(),
    }
}

fn render_receipt(
    started: Instant,
    outcome: BeadStageOutcome,
    stderr_excerpt: String,
) -> BeadStageReceipt {
    BeadStageReceipt {
        stage: BeadStage::Render,
        argv: Vec::new(),
        exit_status: None,
        elapsed_ms: elapsed_ms(started.elapsed()),
        stdout_excerpt: String::new(),
        stderr_excerpt,
        outcome,
    }
}

pub(crate) fn process_receipt(
    stage: BeadStage,
    spec: &CommandSpec,
    output: &ProcessOutput,
    outcome: BeadStageOutcome,
) -> BeadStageReceipt {
    BeadStageReceipt {
        stage,
        argv: spec.argv(),
        exit_status: output.exit_status,
        elapsed_ms: elapsed_ms(output.elapsed),
        stdout_excerpt: excerpt(&output.stdout),
        stderr_excerpt: excerpt(&output.stderr),
        outcome,
    }
}

fn mark_last_stage_failed(stages: &mut [BeadStageReceipt], code: String) {
    if let Some(receipt) = stages.last_mut() {
        receipt.outcome = BeadStageOutcome::Failed { code };
    }
}

pub(crate) fn failed_last_stage_receipt(
    request: &BeadComposeRequest,
    rendered_formula: PathBuf,
    mut stages: Vec<BeadStageReceipt>,
    error: &BeadComposeError,
) -> BeadComposeReceipt {
    let code = error.code().to_owned();
    mark_last_stage_failed(&mut stages, code.clone());
    receipt(
        request,
        rendered_formula,
        stages,
        BeadOutcome::Failed { code },
    )
}

fn elapsed_ms(duration: std::time::Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

pub(crate) fn excerpt(value: &str) -> String {
    value.chars().take(OUTPUT_EXCERPT_LIMIT).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde_json::{Map, json};

    use super::{BEADS_SCHEMA_V1, execute_bead_request_with_runner};
    #[cfg(unix)]
    use crate::StdProcessRunner;
    use crate::{
        BeadComposeError, BeadComposeRequest, BeadOperation, BeadOutcome, CommandSpec,
        ProcessOutput, ProcessRunner,
    };

    static WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    pub(crate) struct FakeRunner {
        outputs: Mutex<VecDeque<ProcessOutput>>,
        pub(crate) calls: Mutex<Vec<CommandSpec>>,
    }

    impl FakeRunner {
        pub(crate) fn with_outputs(outputs: impl IntoIterator<Item = ProcessOutput>) -> Self {
            Self {
                outputs: Mutex::new(outputs.into_iter().collect()),
                calls: Mutex::default(),
            }
        }
    }

    impl ProcessRunner for FakeRunner {
        fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
            self.calls.lock().expect("calls lock").push(spec.clone());
            self.outputs
                .lock()
                .expect("outputs lock")
                .pop_front()
                .ok_or_else(|| io::Error::other("missing fake process output"))
        }
    }

    struct UnavailableRunner;

    impl ProcessRunner for UnavailableRunner {
        fn run(&self, _spec: &CommandSpec) -> io::Result<ProcessOutput> {
            Err(io::Error::new(io::ErrorKind::NotFound, "bd not found"))
        }
    }

    struct InvalidInputRunner;

    impl ProcessRunner for InvalidInputRunner {
        fn run(&self, _spec: &CommandSpec) -> io::Result<ProcessOutput> {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "argument cannot be represented",
            ))
        }
    }

    struct OutputLimitRunner;

    impl ProcessRunner for OutputLimitRunner {
        fn run(&self, _spec: &CommandSpec) -> io::Result<ProcessOutput> {
            Err(crate::runner::process_output_limit_error())
        }
    }

    pub(crate) fn success(stdout: &str) -> ProcessOutput {
        ProcessOutput {
            exit_status: Some(0),
            stdout: stdout.to_owned(),
            stderr: String::new(),
            elapsed: Duration::from_millis(2),
        }
    }

    fn failure() -> ProcessOutput {
        ProcessOutput {
            exit_status: Some(7),
            stdout: String::new(),
            stderr: String::from("invalid formula"),
            elapsed: Duration::from_millis(2),
        }
    }

    pub(crate) fn where_output(active_beads_dir: &Path) -> ProcessOutput {
        success(&serde_json::json!({ "path": active_beads_dir }).to_string())
    }

    pub(crate) fn workspace() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = WORKSPACE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "sc-composer-beads-test-{}-{unique}-{sequence}",
            std::process::id(),
        ));
        fs::create_dir_all(&root).expect("create test workspace");
        root
    }

    pub(crate) fn request(root: &Path, operation: BeadOperation) -> BeadComposeRequest {
        let template = root.join("example.formula.toml.j2");
        fs::write(
            &template,
            "{% for person in people %}- {{{ person.name }}}\\n{% endfor %}runtime = \"{{ bead_var }}\"\\n",
        )
        .expect("write template");
        BeadComposeRequest {
            schema: String::from(BEADS_SCHEMA_V1),
            operation,
            working_directory: root.into(),
            template,
            rendered_formula: root.join("example.formula.toml"),
            compose_variables: Map::from_iter([(String::from("people"), json!([{"name": "Ada"}]))]),
            formula_name: Some(crate::FormulaName::new("example").expect("formula name")),
            bead_variables: BTreeMap::from([
                (String::from("zebra"), String::from("last")),
                (String::from("alpha"), String::from("first")),
            ]),
            bd_executable: Some(PathBuf::from("fake-bd")),
            pour_authorization: None,
            parent: None,
            ref_: None,
            relations: Vec::new(),
        }
    }

    #[test]
    fn attach_output_refusal_retains_exact_path_in_typed_diagnostic() {
        for operation in [BeadOperation::PreviewAttach, BeadOperation::Attach] {
            let root = fs::canonicalize(workspace()).unwrap();
            let mut request = request(&root, operation);
            request.parent = Some(crate::BeadId::new("proj-1").unwrap());
            request.ref_ = Some(crate::GraphRef::new("chain").unwrap());
            request.pour_authorization = Some(crate::PourAuthorization::CreatePersistentBeads);
            request.bead_variables.clear();
            fs::write(&request.template, "formula = \"example\"\n").unwrap();
            let mut path = request.rendered_formula.as_os_str().to_os_string();
            path.push(".graph.json");
            let path = PathBuf::from(path);
            fs::create_dir(&path).unwrap();
            let runner = FakeRunner::with_outputs([
                success(
                    r#"{"formula":"example","type":"workflow","steps":[{"id":"a","title":"A"}]}"#,
                ),
                success(r#"[{"id":"proj-1"}]"#),
            ]);
            let mut diagnostics = Vec::new();
            let receipt = super::execute_with_runner_and_diagnostics(&request, &runner, &mut |error| {
                assert!(matches!(error, BeadComposeError::OutputPathInvalid { path: actual, .. } if actual == &path));
                diagnostics.push(serde_json::to_value(error).unwrap());
            }).unwrap();
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(
                diagnostics[0]["details"]["value"],
                path.to_string_lossy().as_ref()
            );
            assert_eq!(
                receipt.outcome,
                BeadOutcome::Refused {
                    code: "BEADS_OUTPUT_PATH_INVALID".into()
                }
            );
            assert!(receipt.graph.is_none());
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn graph_diagnostic_callback_retains_typed_error_without_changing_receipt_json() {
        let root = workspace();
        let mut request = request(&root, BeadOperation::PreviewAttach);
        request.parent = Some(crate::BeadId::new("nosuch").unwrap());
        request.ref_ = Some(crate::GraphRef::new("r").unwrap());
        request.bead_variables.clear();
        fs::write(&request.template, "formula = \"example\"\n").unwrap();
        let runner = FakeRunner::with_outputs([
            success(r#"{"formula":"example","type":"workflow","steps":[{"id":"a","title":"A"}]}"#),
            success("[]"),
        ]);
        let mut diagnostics = Vec::new();
        let receipt = super::execute_with_runner_and_diagnostics(&request, &runner, &mut |error| {
            assert!(matches!(error, BeadComposeError::GraphParentNotFound { parent } if parent.as_str() == "nosuch"));
            diagnostics.push(serde_json::to_value(error).unwrap());
        }).unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["details"], json!({"parent":"nosuch"}));
        assert_eq!(
            diagnostics[0]["recovery"],
            "Create the parent or name an existing bead."
        );
        let wire = serde_json::to_value(&receipt).unwrap();
        assert_eq!(
            wire["outcome"],
            json!({"refused":{"code":"BEADS_GRAPH_PARENT_NOT_FOUND"}})
        );
        assert!(wire.get("error").is_none());
        assert!(wire.get("diagnostics").is_none());
        assert!(
            receipt
                .stages
                .last()
                .unwrap()
                .stderr_excerpt
                .contains("graph parent `nosuch` does not exist")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn render_keeps_beads_runtime_placeholders_and_renders_structured_values() {
        let root = workspace();
        let result = execute_bead_request_with_runner(
            &request(&root, BeadOperation::Render),
            &FakeRunner::default(),
        )
        .expect("render result");
        assert_eq!(result.outcome, BeadOutcome::Succeeded);
        let rendered =
            fs::read_to_string(root.join("example.formula.toml")).expect("rendered formula");
        assert!(rendered.contains("- Ada"));
        assert!(rendered.contains("{{ bead_var }}"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn validate_uses_direct_sorted_bd_arguments() {
        let root = workspace();
        let runner = FakeRunner::with_outputs([success("{}")]);
        let result =
            execute_bead_request_with_runner(&request(&root, BeadOperation::Validate), &runner)
                .expect("validate result");
        assert_eq!(result.outcome, BeadOutcome::Succeeded);
        let calls = runner.calls.lock().expect("calls lock");
        assert_eq!(calls.len(), 1);
        let canonical_output = fs::canonicalize(&root)
            .expect("canonical root")
            .join("example.formula.toml");
        assert_eq!(
            calls[0].args,
            vec![
                "cook",
                canonical_output.to_string_lossy().as_ref(),
                "--dry-run",
                "--json",
                "--var",
                "alpha=first",
                "--var",
                "zebra=last",
            ]
        );
        drop(calls);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn failed_validation_blocks_later_stages_and_records_receipt() {
        let root = workspace();
        let runner = FakeRunner::with_outputs([failure()]);
        let result =
            execute_bead_request_with_runner(&request(&root, BeadOperation::PreviewPour), &runner)
                .expect("failed receipt");
        assert_eq!(
            result.outcome,
            BeadOutcome::Failed {
                code: String::from("BEADS_COOK_FAILED")
            }
        );
        assert_eq!(result.stages.len(), 2);
        assert_eq!(runner.calls.lock().expect("calls lock").len(), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn unavailable_bd_returns_a_stable_error_before_a_process_receipt() {
        let root = workspace();
        let error = execute_bead_request_with_runner(
            &request(&root, BeadOperation::Validate),
            &UnavailableRunner,
        )
        .expect_err("missing executable must fail");
        assert_eq!(error.code(), "BEADS_BD_UNAVAILABLE");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn invalid_process_arguments_are_not_misattributed_as_bd_unavailable() {
        let root = workspace();
        let error = execute_bead_request_with_runner(
            &request(&root, BeadOperation::Validate),
            &InvalidInputRunner,
        )
        .expect_err("invalid process arguments must fail distinctly");
        assert!(matches!(
            error,
            BeadComposeError::ProcessArgumentInvalid { ref executable, ref message }
                if executable == Path::new("fake-bd")
                    && message == "argument cannot be represented"
        ));
        assert_eq!(error.code(), "BEADS_PROCESS_ARGUMENT_INVALID");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn bounded_process_output_failure_is_typed_and_names_the_stage() {
        let root = workspace();
        let error = execute_bead_request_with_runner(
            &request(&root, BeadOperation::Validate),
            &OutputLimitRunner,
        )
        .expect_err("output limit must be a typed failure");
        assert!(matches!(
            error,
            BeadComposeError::ProcessOutputLimitExceeded {
                stage: crate::BeadStage::Validate,
                limit_bytes: crate::runner::PROCESS_OUTPUT_LIMIT_BYTES,
            }
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn registry_process_failures_preserve_stage_codes_status_and_evidence() {
        for (operation, stage, code, successful_stages) in [
            (
                BeadOperation::Validate,
                crate::BeadStage::Validate,
                "BEADS_COOK_FAILED",
                0,
            ),
            (
                BeadOperation::PreviewPour,
                crate::BeadStage::ResolveActiveRegistry,
                "BEADS_WHERE_FAILED",
                1,
            ),
            (
                BeadOperation::PreviewPour,
                crate::BeadStage::PreviewPour,
                "BEADS_PREVIEW_POUR_FAILED",
                2,
            ),
            (
                BeadOperation::Pour,
                crate::BeadStage::Pour,
                "BEADS_POUR_FAILED",
                2,
            ),
        ] {
            for status in [Some(7), None] {
                let root = workspace();
                let beads_dir = root.join(".beads");
                let registry = beads_dir.join("formulas");
                fs::create_dir_all(&registry).expect("registry");
                let mut request = request(&root, operation);
                request.rendered_formula = registry.join("example.formula.toml");
                request.pour_authorization = Some(crate::PourAuthorization::CreatePersistentBeads);
                let mut outputs = Vec::new();
                if successful_stages > 0 {
                    outputs.push(success("{}"));
                }
                if successful_stages > 1 {
                    outputs.push(where_output(&beads_dir));
                }
                let mut failed_output = failure();
                failed_output.exit_status = status;
                failed_output.stdout = "partial output".into();
                outputs.push(failed_output);
                let runner = FakeRunner::with_outputs(outputs);

                let receipt = execute_bead_request_with_runner(&request, &runner).expect("receipt");
                assert_eq!(receipt.outcome, BeadOutcome::Failed { code: code.into() });
                assert_eq!(receipt.stages.len(), successful_stages + 2);
                let failed_stage = receipt.stages.last().expect("failed stage");
                assert_eq!(failed_stage.stage, stage);
                assert_eq!(
                    failed_stage.outcome,
                    crate::BeadStageOutcome::Failed { code: code.into() }
                );
                assert_eq!(failed_stage.exit_status, status);
                assert_eq!(failed_stage.stdout_excerpt, "partial output");
                assert_eq!(failed_stage.stderr_excerpt, "invalid formula");
                assert_eq!(failed_stage.elapsed_ms, 2);
                let calls = runner.calls.lock().expect("calls");
                assert_eq!(calls.len(), successful_stages + 1);
                assert_eq!(
                    failed_stage.argv,
                    calls.last().expect("attempted command").argv()
                );
                drop(calls);
                fs::remove_dir_all(root).expect("cleanup");
            }
        }
    }

    #[test]
    fn persistent_pour_without_authorization_spawns_nothing() {
        let root = workspace();
        let runner = FakeRunner::default();
        let error = execute_bead_request_with_runner(&request(&root, BeadOperation::Pour), &runner)
            .expect_err("authorization must be rejected");
        assert_eq!(error.code(), "BEADS_POUR_AUTH_REQUIRED");
        assert!(runner.calls.lock().expect("calls lock").is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn request_preconditions_reject_without_spawning_bd() {
        let root = workspace();
        let assert_rejected = |request: &BeadComposeRequest, expected_code: &str| {
            let runner = FakeRunner::default();
            let error = execute_bead_request_with_runner(request, &runner)
                .expect_err("request precondition must be rejected");
            assert_eq!(error.code(), expected_code);
            assert!(runner.calls.lock().expect("calls lock").is_empty());
        };

        let mut unknown_schema = request(&root, BeadOperation::PreviewPour);
        unknown_schema.schema = String::from("unsupported/v1");
        assert_rejected(&unknown_schema, "BEADS_UNKNOWN_SCHEMA");

        let mut missing_name = request(&root, BeadOperation::PreviewPour);
        missing_name.formula_name = None;
        assert_rejected(&missing_name, "BEADS_FORMULA_NAME_REQUIRED");

        let mut malformed_key = request(&root, BeadOperation::PreviewPour);
        malformed_key
            .bead_variables
            .insert(String::from("?bad"), String::from("value"));
        assert_rejected(&malformed_key, "BEADS_VARIABLE_KEY_INVALID");

        let mut unsupported_extension = request(&root, BeadOperation::PreviewPour);
        unsupported_extension.rendered_formula = root.join("example.invalid");
        assert_rejected(
            &unsupported_extension,
            "BEADS_FORMULA_EXTENSION_UNSUPPORTED",
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn template_outside_working_directory_is_rejected() {
        let root = workspace();
        let outside = workspace();
        let mut request = request(&root, BeadOperation::Render);
        request.template = outside.join("outside.formula.toml.j2");
        fs::write(&request.template, "name = \"{{{ name }}}\"").expect("write outside template");
        let error = execute_bead_request_with_runner(&request, &FakeRunner::default())
            .expect_err("template escape must fail");
        assert!(matches!(
            error,
            BeadComposeError::TemplateOutsideWorkingDirectory { .. }
        ));
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }

    #[cfg(unix)]
    #[test]
    fn final_output_symlink_is_rejected_without_touching_its_target() {
        use std::os::unix::fs::symlink;

        let root = workspace();
        let outside = workspace();
        let target = outside.join("outside.formula.toml");
        fs::write(&target, "keep this content").expect("write target");
        let request = request(&root, BeadOperation::Render);
        symlink(&target, &request.rendered_formula).expect("create output symlink");

        let runner = FakeRunner::default();
        let error = execute_bead_request_with_runner(&request, &runner)
            .expect_err("final output symlink must be rejected");

        assert!(matches!(error, BeadComposeError::OutputPathSymlink { .. }));
        assert_eq!(
            fs::read_to_string(&target).expect("read target"),
            "keep this content"
        );
        assert!(runner.calls.lock().expect("calls lock").is_empty());
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }

    #[cfg(windows)]
    #[test]
    fn final_output_symlink_is_rejected_without_touching_its_target() {
        use std::os::windows::fs::symlink_file;

        let root = workspace();
        let outside = workspace();
        let target = outside.join("outside.formula.toml");
        fs::write(&target, "keep this content").expect("write target");
        let request = request(&root, BeadOperation::Render);
        if let Err(error) = symlink_file(&target, &request.rendered_formula) {
            eprintln!(
                "skipping Windows symlink assertion because the test account lacks symlink permission: {error}"
            );
            fs::remove_dir_all(root).expect("cleanup root");
            fs::remove_dir_all(outside).expect("cleanup outside");
            return;
        }

        let runner = FakeRunner::default();
        let error = execute_bead_request_with_runner(&request, &runner)
            .expect_err("final output symlink must be rejected");

        assert!(matches!(error, BeadComposeError::OutputPathSymlink { .. }));
        assert_eq!(
            fs::read_to_string(&target).expect("read target"),
            "keep this content"
        );
        assert!(runner.calls.lock().expect("calls lock").is_empty());
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }

    // FUZZ-4177-ENC-01 (adversarial fuzz campaign 20260817-2, path-encoding-probe):
    // regression coverage for rejecting an argv-illegal value before process
    // launch and reporting the offending variable rather than bd unavailability.
    #[cfg(unix)]
    #[test]
    fn nul_byte_poisoned_bead_variable_value_is_rejected_as_invalid() {
        let root = workspace();
        let mut request = request(&root, BeadOperation::Validate);
        request.bd_executable = Some(PathBuf::from("/bin/echo"));
        request
            .bead_variables
            .insert(String::from("payload"), String::from("legit\u{0}withnul"));

        let error = execute_bead_request_with_runner(&request, &StdProcessRunner)
            .expect_err("a NUL-poisoned bead variable value must be rejected");

        assert!(matches!(
            error,
            BeadComposeError::BeadVariableValueInvalid { ref key, ref value }
                if key == "payload" && value == "legit\u{0}withnul"
        ));
        assert_eq!(error.code(), "BEADS_VARIABLE_VALUE_INVALID");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn direct_rust_non_utf8_path_is_rejected_before_rendering() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let root = workspace();
        let mut request = request(&root, BeadOperation::Render);
        request.rendered_formula =
            root.join(OsString::from_vec(b"output-\xff.formula.toml".to_vec()));

        let runner = FakeRunner::default();
        let error = execute_bead_request_with_runner(&request, &runner)
            .expect_err("non-UTF-8 direct Rust path must be rejected");

        assert!(matches!(error, BeadComposeError::PathNotUtf8 { .. }));
        assert!(runner.calls.lock().expect("calls lock").is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }
}

#[cfg(test)]
mod fuzz_055_tests {
    use super::*;

    #[test]
    fn fuzz_055_snapshot_paths_are_not_exposed_in_stage_evidence() {
        let snapshot = Path::new("/work/.sc-compose-input-1.formula.toml");
        let source = Path::new("/work/rendered.formula.toml");
        let mut stages = vec![BeadStageReceipt {
            stage: BeadStage::Validate,
            argv: vec!["cook".into(), snapshot.display().to_string()],
            exit_status: Some(7),
            elapsed_ms: 0,
            stdout_excerpt: snapshot.display().to_string(),
            stderr_excerpt: format!("failed {}", snapshot.display()),
            outcome: BeadStageOutcome::Failed {
                code: "BEADS_COOK_FAILED".into(),
            },
        }];
        present_snapshot_paths(&mut stages, snapshot, source);
        let evidence = format!("{:?}", stages[0]);
        assert!(evidence.contains("rendered.formula.toml"));
        assert!(!evidence.contains(".sc-compose-input-"));
    }
}
