//! Render-to-`bd` operation staging.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::Value;

use crate::contract::{
    BEADS_SCHEMA_V1, BeadComposeReceipt, BeadComposeRequest, BeadOperation, BeadOutcome, BeadStage,
    BeadStageOutcome, BeadStageReceipt, PourAuthorization,
};
use crate::error::BeadComposeError;
use crate::render::render_formula;
use crate::runner::{CommandSpec, ProcessOutput, ProcessRunner, StdProcessRunner};

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

/// Execute a Beads request through an injected direct process runner.
///
/// # Errors
///
/// Returns a stable error for rejected request preconditions or an unavailable
/// executable. Process failures return a failed receipt with stage evidence.
#[allow(
    clippy::too_many_lines,
    reason = "The receipt-producing render, validate, registry, and pour progression is intentionally visible in one ordered function."
)]
pub fn execute_bead_request_with_runner(
    request: &BeadComposeRequest,
    runner: &dyn ProcessRunner,
) -> Result<BeadComposeReceipt, BeadComposeError> {
    let normalized = validate_request(request)?;
    let mut stages = Vec::new();
    let render_started = Instant::now();
    if let Err(error) = render_formula(
        &normalized.template,
        &normalized.rendered_formula,
        &request.compose_variables,
    ) {
        stages.push(render_receipt(
            render_started,
            BeadStageOutcome::Failed {
                code: error.code().to_owned(),
            },
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
    stages.push(render_receipt(render_started, BeadStageOutcome::Succeeded));

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
    let cook = CommandSpec {
        executable: bd.clone(),
        args: cook_args(&normalized.rendered_formula, request),
        working_directory: normalized.working_directory.clone(),
    };
    if let Some(failed) = run_stage(
        runner,
        BeadStage::Validate,
        &cook,
        BeadComposeError::CookFailed { exit_status: None },
        &mut stages,
    )? {
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

    let formula_name = request
        .formula_name
        .as_deref()
        .ok_or(BeadComposeError::FormulaNameRequired)?;
    let where_spec = CommandSpec {
        executable: bd.clone(),
        args: vec![String::from("where"), String::from("--json")],
        working_directory: normalized.working_directory.clone(),
    };
    let where_output = match run_stage_with_output(
        runner,
        BeadStage::ResolveActiveRegistry,
        &where_spec,
        BeadComposeError::ActiveRegistryResolutionFailed { exit_status: None },
        &mut stages,
    )? {
        Ok(output) => output,
        Err(outcome) => {
            return Ok(receipt(
                request,
                normalized.rendered_formula,
                stages,
                outcome,
            ));
        }
    };
    let Some(active_beads_dir) =
        parse_active_beads_dir(&where_output.stdout).and_then(|path| fs::canonicalize(path).ok())
    else {
        return Ok(failed_last_stage_receipt(
            request,
            normalized.rendered_formula,
            stages,
            &BeadComposeError::ActiveRegistryResolutionFailed { exit_status: None },
        ));
    };
    if let Err(error) = validate_active_registry_path(
        formula_name,
        &normalized.rendered_formula,
        &active_beads_dir,
    ) {
        return Ok(failed_last_stage_receipt(
            request,
            normalized.rendered_formula,
            stages,
            &error,
        ));
    }

    let preview = request.operation == BeadOperation::PreviewPour;
    let pour = CommandSpec {
        executable: bd,
        args: pour_args(formula_name, request, preview),
        working_directory: normalized.working_directory,
    };
    let stage = if preview {
        BeadStage::PreviewPour
    } else {
        BeadStage::Pour
    };
    let failed_error = if preview {
        BeadComposeError::PreviewPourFailed { exit_status: None }
    } else {
        BeadComposeError::PourFailed { exit_status: None }
    };
    if let Some(failed) = run_stage(runner, stage, &pour, failed_error, &mut stages)? {
        return Ok(receipt(
            request,
            normalized.rendered_formula,
            stages,
            failed,
        ));
    }
    Ok(receipt(
        request,
        normalized.rendered_formula,
        stages,
        BeadOutcome::Succeeded,
    ))
}

struct NormalizedRequest {
    working_directory: PathBuf,
    template: PathBuf,
    rendered_formula: PathBuf,
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
    ) && request.formula_name.as_deref().is_none_or(str::is_empty)
    {
        return Err(BeadComposeError::FormulaNameRequired);
    }
    if request.operation == BeadOperation::Pour
        && request.pour_authorization != Some(PourAuthorization::CreatePersistentBeads)
    {
        return Err(BeadComposeError::PourAuthorizationRequired);
    }
    for key in request.bead_variables.keys() {
        if !valid_bead_key(key) {
            return Err(BeadComposeError::BeadVariableKeyInvalid { key: key.clone() });
        }
    }
    let working_directory = fs::canonicalize(&request.working_directory).map_err(|_error| {
        BeadComposeError::TemplatePathInvalid {
            path: request.working_directory.clone(),
        }
    })?;
    let template = fs::canonicalize(&request.template).map_err(|_error| {
        BeadComposeError::TemplatePathInvalid {
            path: request.template.clone(),
        }
    })?;
    if !template.is_file() {
        return Err(BeadComposeError::FormulaPathNotFile { path: template });
    }
    if !template.starts_with(&working_directory) {
        return Err(BeadComposeError::TemplateOutsideWorkingDirectory { path: template });
    }
    let rendered_formula = normalize_output(&request.rendered_formula)?;
    if !is_formula_path(&rendered_formula) {
        return Err(BeadComposeError::FormulaExtensionUnsupported {
            path: rendered_formula,
        });
    }
    if matches!(
        request.operation,
        BeadOperation::Render | BeadOperation::Validate
    ) && !rendered_formula.starts_with(&working_directory)
    {
        return Err(BeadComposeError::OutputOutsideWorkingDirectory {
            path: rendered_formula,
        });
    }
    Ok(NormalizedRequest {
        working_directory,
        template,
        rendered_formula,
    })
}

fn normalize_output(path: &Path) -> Result<PathBuf, BeadComposeError> {
    let parent = path
        .parent()
        .ok_or_else(|| BeadComposeError::TemplatePathInvalid { path: path.into() })?;
    let parent = fs::canonicalize(parent)
        .map_err(|_error| BeadComposeError::TemplatePathInvalid { path: path.into() })?;
    let name = path
        .file_name()
        .ok_or_else(|| BeadComposeError::TemplatePathInvalid { path: path.into() })?;
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

fn pour_args(formula_name: &str, request: &BeadComposeRequest, preview: bool) -> Vec<String> {
    let mut args = vec![
        String::from("mol"),
        String::from("pour"),
        formula_name.to_owned(),
    ];
    if preview {
        args.push(String::from("--dry-run"));
    }
    args.push(String::from("--json"));
    append_variables(&mut args, request);
    args
}

fn append_variables(args: &mut Vec<String>, request: &BeadComposeRequest) {
    for (key, value) in &request.bead_variables {
        args.push(String::from("--var"));
        args.push(format!("{key}={value}"));
    }
}

fn run_stage(
    runner: &dyn ProcessRunner,
    stage: BeadStage,
    spec: &CommandSpec,
    template_error: BeadComposeError,
    stages: &mut Vec<BeadStageReceipt>,
) -> Result<Option<BeadOutcome>, BeadComposeError> {
    match run_stage_with_output(runner, stage, spec, template_error, stages)? {
        Ok(_) => Ok(None),
        Err(outcome) => Ok(Some(outcome)),
    }
}

fn run_stage_with_output(
    runner: &dyn ProcessRunner,
    stage: BeadStage,
    spec: &CommandSpec,
    template_error: BeadComposeError,
    stages: &mut Vec<BeadStageReceipt>,
) -> Result<Result<ProcessOutput, BeadOutcome>, BeadComposeError> {
    let output = runner
        .run(spec)
        .map_err(|_error| BeadComposeError::BdUnavailable {
            executable: spec.executable.clone(),
        })?;
    let successful = output.exit_status == Some(0);
    let code = error_with_status(template_error, output.exit_status)
        .code()
        .to_owned();
    stages.push(process_receipt(
        stage,
        spec,
        &output,
        if successful {
            BeadStageOutcome::Succeeded
        } else {
            BeadStageOutcome::Failed { code: code.clone() }
        },
    ));
    if successful {
        Ok(Ok(output))
    } else {
        Ok(Err(BeadOutcome::Failed { code }))
    }
}

fn error_with_status(error: BeadComposeError, exit_status: Option<i32>) -> BeadComposeError {
    match error {
        BeadComposeError::CookFailed { .. } => BeadComposeError::CookFailed { exit_status },
        BeadComposeError::ActiveRegistryResolutionFailed { .. } => {
            BeadComposeError::ActiveRegistryResolutionFailed { exit_status }
        }
        BeadComposeError::PreviewPourFailed { .. } => {
            BeadComposeError::PreviewPourFailed { exit_status }
        }
        BeadComposeError::PourFailed { .. } => BeadComposeError::PourFailed { exit_status },
        other => other,
    }
}

fn receipt(
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
    }
}

fn render_receipt(started: Instant, outcome: BeadStageOutcome) -> BeadStageReceipt {
    BeadStageReceipt {
        stage: BeadStage::Render,
        argv: Vec::new(),
        exit_status: None,
        elapsed_ms: elapsed_ms(started.elapsed()),
        stdout_excerpt: String::new(),
        stderr_excerpt: String::new(),
        outcome,
    }
}

fn process_receipt(
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

fn failed_last_stage_receipt(
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

fn excerpt(value: &str) -> String {
    value.chars().take(OUTPUT_EXCERPT_LIMIT).collect()
}

fn parse_active_beads_dir(stdout: &str) -> Option<PathBuf> {
    let value: Value = serde_json::from_str(stdout).ok()?;
    value.get("path").and_then(Value::as_str).map(PathBuf::from)
}

fn validate_active_registry_path(
    formula_name: &str,
    rendered_formula: &Path,
    active_beads_dir: &Path,
) -> Result<(), BeadComposeError> {
    let toml = active_beads_dir
        .join("formulas")
        .join(format!("{formula_name}.formula.toml"));
    let json = active_beads_dir
        .join("formulas")
        .join(format!("{formula_name}.formula.json"));
    if toml.is_file() && json.is_file() {
        return Err(BeadComposeError::FormulaRegistryAmbiguous {
            formula_name: formula_name.to_owned(),
        });
    }
    if rendered_formula != toml && rendered_formula != json {
        return Err(BeadComposeError::FormulaOutsideActiveRegistry {
            path: rendered_formula.into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde_json::{Map, json};

    use super::{BEADS_SCHEMA_V1, execute_bead_request_with_runner};
    use crate::{
        BeadComposeError, BeadComposeRequest, BeadOperation, BeadOutcome, CommandSpec,
        ProcessOutput, ProcessRunner,
    };

    static WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct FakeRunner {
        outputs: Mutex<VecDeque<ProcessOutput>>,
        calls: Mutex<Vec<CommandSpec>>,
    }

    impl FakeRunner {
        fn with_outputs(outputs: impl IntoIterator<Item = ProcessOutput>) -> Self {
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

    fn success(stdout: &str) -> ProcessOutput {
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

    fn where_output(active_beads_dir: &Path) -> ProcessOutput {
        success(&serde_json::json!({ "path": active_beads_dir }).to_string())
    }

    fn workspace() -> PathBuf {
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

    fn request(root: &Path, operation: BeadOperation) -> BeadComposeRequest {
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
            formula_name: Some(String::from("example")),
            bead_variables: BTreeMap::from([
                (String::from("zebra"), String::from("last")),
                (String::from("alpha"), String::from("first")),
            ]),
            bd_executable: Some(PathBuf::from("fake-bd")),
            pour_authorization: None,
        }
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
    fn malformed_where_output_marks_the_attempted_stage_failed() {
        let root = workspace();
        let active_registry = root.join(".beads").join("formulas");
        fs::create_dir_all(&active_registry).expect("create active registry");
        let mut request = request(&root, BeadOperation::PreviewPour);
        request.rendered_formula = active_registry.join("example.formula.toml");
        let runner = FakeRunner::with_outputs([success("{}"), success("{\"not_path\":true}")]);

        let result = execute_bead_request_with_runner(&request, &runner).expect("receipt");
        assert_eq!(
            result.outcome,
            BeadOutcome::Failed {
                code: String::from("BEADS_WHERE_FAILED")
            }
        );
        assert_eq!(result.stages.len(), 3);
        assert_eq!(
            result.stages[2].outcome,
            crate::BeadStageOutcome::Failed {
                code: String::from("BEADS_WHERE_FAILED")
            }
        );
        assert_eq!(runner.calls.lock().expect("calls lock").len(), 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn preview_uses_the_canonical_active_registry_and_direct_pour_argv() {
        let root = workspace();
        let active_beads_dir = root.join(".beads");
        let registry = active_beads_dir.join("formulas");
        fs::create_dir_all(&registry).expect("create active registry");
        let mut request = request(&root, BeadOperation::PreviewPour);
        request.rendered_formula = registry.join("example.formula.toml");
        let runner = FakeRunner::with_outputs([
            success("{}"),
            where_output(&active_beads_dir),
            success("{}"),
        ]);

        let receipt = execute_bead_request_with_runner(&request, &runner).expect("receipt");
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded);
        assert_eq!(receipt.stages.len(), 4);
        let calls = runner.calls.lock().expect("calls lock");
        assert_eq!(calls.len(), 3);
        assert_eq!(
            calls[2].args,
            vec![
                "mol",
                "pour",
                "example",
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
    fn same_name_toml_json_pair_blocks_preview_before_pour() {
        let root = workspace();
        let active_beads_dir = root.join(".beads");
        let registry = active_beads_dir.join("formulas");
        fs::create_dir_all(&registry).expect("create active registry");
        let toml = registry.join("example.formula.toml");
        let json = registry.join("example.formula.json");
        fs::write(&toml, "formula = \"example\"").expect("write TOML shadow");
        fs::write(&json, "{\"formula\":\"example\"}").expect("write JSON shadow");
        let mut request = request(&root, BeadOperation::PreviewPour);
        request.rendered_formula = toml;
        let runner = FakeRunner::with_outputs([success("{}"), where_output(&active_beads_dir)]);

        let receipt = execute_bead_request_with_runner(&request, &runner).expect("receipt");
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Failed {
                code: String::from("BEADS_FORMULA_REGISTRY_AMBIGUOUS")
            }
        );
        assert_eq!(runner.calls.lock().expect("calls lock").len(), 2);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn preview_rejects_an_output_outside_the_active_registry_before_pour() {
        let root = workspace();
        let active_beads_dir = root.join(".beads");
        fs::create_dir_all(active_beads_dir.join("formulas")).expect("create active registry");
        let runner = FakeRunner::with_outputs([success("{}"), where_output(&active_beads_dir)]);

        let receipt =
            execute_bead_request_with_runner(&request(&root, BeadOperation::PreviewPour), &runner)
                .expect("receipt");
        assert_eq!(
            receipt.outcome,
            BeadOutcome::Failed {
                code: String::from("BEADS_FORMULA_OUTSIDE_ACTIVE_REGISTRY")
            }
        );
        assert_eq!(runner.calls.lock().expect("calls lock").len(), 2);
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
}
