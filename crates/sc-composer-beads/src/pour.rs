//! Registry resolution and preview/persistent pour staging.

use crate::contract::{
    BeadComposeReceipt, BeadComposeRequest, BeadOperation, BeadOutcome, BeadStage, BeadStageReceipt,
};
use crate::error::BeadComposeError;
use crate::execute::{
    NormalizedRequest, StageFailure, append_variables, failed_last_stage_receipt, public_path_buf,
    receipt, run_stage, run_stage_with_output,
};
use crate::runner::{CommandSpec, ProcessRunner};
use crate::snapshot::InputSnapshot;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn execute_pour(
    request: &BeadComposeRequest,
    runner: &dyn ProcessRunner,
    normalized: NormalizedRequest,
    formula_input: &InputSnapshot,
    bd: PathBuf,
    mut stages: Vec<BeadStageReceipt>,
    diagnostics: &mut dyn FnMut(&BeadComposeError),
) -> Result<BeadComposeReceipt, BeadComposeError> {
    let formula_name = request
        .formula_name
        .as_ref()
        .ok_or(BeadComposeError::FormulaNameRequired)?;
    let where_spec = CommandSpec {
        executable: bd.clone(),
        args: vec![String::from("where"), String::from("--json")],
        working_directory: normalized.working_directory.clone(),
    };
    let where_output = match run_stage_with_output(
        runner,
        StageFailure::ResolveActiveRegistry,
        &where_spec,
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
    if normalized.rendered_formula.parent() != Some(active_beads_dir.join("formulas").as_path()) {
        // Only the active registry may sit outside `working_directory`; any
        // other destination is refused before it is written.
        if normalized.defers_publish(request.operation) {
            return Err(BeadComposeError::OutputOutsideWorkingDirectory {
                path: public_path_buf(&normalized.rendered_formula),
            });
        }
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
    if !request.relations.is_empty() {
        return Ok(refuse_registry_relations(request, normalized, stages));
    }
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
    // `bd mol pour` reads the formula by name from the registry.
    if normalized.defers_publish(request.operation) {
        formula_input.publish_copy(&normalized.rendered_formula)?;
    }

    let preview = request.operation == BeadOperation::PreviewPour;
    let pour = CommandSpec {
        executable: bd,
        args: pour_args(formula_name, request, preview),
        working_directory: normalized.working_directory,
    };
    let failure = if preview {
        StageFailure::PreviewPour
    } else {
        StageFailure::Pour
    };
    if let Some(failed) = run_stage(runner, failure, &pour, &mut stages)? {
        return Ok(receipt(
            request,
            normalized.rendered_formula,
            stages,
            failed,
        ));
    }
    let mut result = receipt(
        request,
        normalized.rendered_formula,
        stages,
        BeadOutcome::Succeeded,
    );
    result.pour_mode = Some(crate::BeadPourMode::Registry);
    Ok(result)
}

fn pour_args(
    formula_name: &crate::FormulaName,
    request: &BeadComposeRequest,
    preview: bool,
) -> Vec<String> {
    let mut args = vec![
        String::from("mol"),
        String::from("pour"),
        formula_name.to_string(),
    ];
    if preview {
        args.push(String::from("--dry-run"));
    }
    args.push(String::from("--json"));
    append_variables(&mut args, request);
    args
}

fn parse_active_beads_dir(stdout: &str) -> Option<PathBuf> {
    let value: Value = serde_json::from_str(stdout).ok()?;
    value.get("path").and_then(Value::as_str).map(PathBuf::from)
}

fn validate_active_registry_path(
    formula_name: &crate::FormulaName,
    rendered_formula: &Path,
    active_beads_dir: &Path,
) -> Result<(), BeadComposeError> {
    let toml = active_beads_dir
        .join("formulas")
        .join(format!("{formula_name}.formula.toml"));
    let json = active_beads_dir
        .join("formulas")
        .join(format!("{formula_name}.formula.json"));
    // A deferred destination is not written yet; count it as present.
    if (rendered_formula == toml || toml.is_file()) && (rendered_formula == json || json.is_file())
    {
        return Err(BeadComposeError::FormulaRegistryAmbiguous {
            formula_name: formula_name.to_string(),
        });
    }
    if rendered_formula != toml && rendered_formula != json {
        return Err(BeadComposeError::FormulaOutsideActiveRegistry {
            path: public_path_buf(rendered_formula),
        });
    }
    Ok(())
}

fn refuse_registry_relations(
    request: &BeadComposeRequest,
    normalized: NormalizedRequest,
    mut stages: Vec<BeadStageReceipt>,
) -> BeadComposeReceipt {
    let code = BeadComposeError::GraphRelationInvalid {
        index: 0,
        reason: crate::GraphRelationInvalidReason::RegistryPour,
    }
    .code()
    .to_owned();
    stages.push(crate::BeadStageReceipt {
        stage: if request.operation == BeadOperation::PreviewPour {
            BeadStage::PreviewPour
        } else {
            BeadStage::Pour
        },
        argv: Vec::new(),
        exit_status: None,
        elapsed_ms: 0,
        stdout_excerpt: String::new(),
        stderr_excerpt: "relations are unavailable for registry pour".into(),
        outcome: crate::BeadStageOutcome::Failed { code: code.clone() },
    });
    let mut result = receipt(
        request,
        normalized.rendered_formula,
        stages,
        BeadOutcome::Refused { code },
    );
    result.pour_mode = Some(crate::BeadPourMode::Registry);
    result
}

#[cfg(test)]
mod tests {
    use crate::execute::tests::{FakeRunner, request, success, where_output, workspace};
    use crate::execute::{execute_bead_request_with_runner, public_path_display};
    use crate::runner::{CommandSpec, ProcessOutput, ProcessRunner};
    use crate::{BeadComposeError, BeadOperation, BeadOutcome, BeadStage, PourAuthorization};
    use std::fs;
    use std::io;
    use std::path::Path;

    /// Runs `hook` before each fake bd command.
    struct HookRunner<F> {
        inner: FakeRunner,
        hook: F,
    }

    impl<F: Fn(&CommandSpec) + Sync> ProcessRunner for HookRunner<F> {
        fn run(&self, spec: &CommandSpec) -> io::Result<ProcessOutput> {
            (self.hook)(spec);
            self.inner.run(spec)
        }
    }

    fn private_inputs(directory: &Path) -> usize {
        fs::read_dir(directory)
            .expect("read directory")
            .filter(|entry| {
                entry
                    .as_ref()
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sc-compose-input-")
            })
            .count()
    }

    #[test]
    fn pour_outside_working_directory_is_a_request_error_before_any_write() {
        for operation in [BeadOperation::PreviewPour, BeadOperation::Pour] {
            let root = workspace();
            let other = workspace();
            let active_beads_dir = root.join(".beads");
            fs::create_dir_all(&active_beads_dir).expect("create active beads dir");
            let outside = other.join("victim.formula.toml");
            fs::write(&outside, "original").expect("write outside file");
            let mut request = request(&root, operation);
            request.rendered_formula = outside.clone();
            request.bead_variables.clear();
            request.pour_authorization = Some(PourAuthorization::CreatePersistentBeads);
            let runner = FakeRunner::with_outputs([success("{}"), where_output(&active_beads_dir)]);

            let error = execute_bead_request_with_runner(&request, &runner)
                .expect_err("outside output is a request error");
            assert!(
                matches!(
                    error,
                    BeadComposeError::OutputOutsideWorkingDirectory { .. }
                ),
                "{error:?}"
            );
            assert_eq!(error.code(), "BEADS_OUTPUT_OUTSIDE_WORKING_DIR");
            assert_eq!(fs::read_to_string(&outside).expect("outside"), "original");
            assert_eq!(private_inputs(&other), 0);
            assert_eq!(runner.calls.lock().expect("calls lock").len(), 2);
            fs::remove_dir_all(root).expect("cleanup");
            fs::remove_dir_all(other).expect("cleanup");
        }
    }

    #[test]
    fn redirected_registry_outside_working_directory_is_written_once_before_pour() {
        let root = workspace();
        let other = workspace();
        let active_beads_dir = other.join(".beads");
        let registry = active_beads_dir.join("formulas");
        fs::create_dir_all(&registry).expect("create active registry");
        let destination = fs::canonicalize(&registry)
            .expect("canonical registry")
            .join("example.formula.toml");
        let mut request = request(&root, BeadOperation::Pour);
        request.rendered_formula = destination.clone();
        request.pour_authorization = Some(PourAuthorization::CreatePersistentBeads);
        let runner = HookRunner {
            inner: FakeRunner::with_outputs([
                success("{}"),
                where_output(&active_beads_dir),
                success("{}"),
            ]),
            hook: |spec: &CommandSpec| match spec.args[0].as_str() {
                "cook" | "where" => assert!(!destination.exists(), "written before bd where"),
                _ => assert!(
                    fs::read_to_string(&destination)
                        .expect("published before pour")
                        .contains("Ada")
                ),
            },
        };

        let receipt = execute_bead_request_with_runner(&request, &runner).expect("receipt");
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded);
        assert_eq!(receipt.pour_mode, Some(crate::BeadPourMode::Registry));
        assert!(destination.is_file());
        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_dir_all(other).expect("cleanup");
    }

    #[test]
    fn successful_pour_is_never_discarded_by_a_later_write() {
        let root = workspace();
        let active_beads_dir = root.join(".beads");
        let registry = active_beads_dir.join("formulas");
        fs::create_dir_all(&registry).expect("create active registry");
        let mut request = request(&root, BeadOperation::Pour);
        request.rendered_formula = registry.join("example.formula.toml");
        request.pour_authorization = Some(PourAuthorization::CreatePersistentBeads);
        let destination = request.rendered_formula.clone();
        // After bd pours, the destination becomes unwritable; the pour already
        // created beads, so its receipt must survive.
        let runner = HookRunner {
            inner: FakeRunner::with_outputs([
                success("{}"),
                where_output(&active_beads_dir),
                success(r#"{"new_epic_id":"proj-1"}"#),
            ]),
            hook: |spec: &CommandSpec| {
                if spec.args[0] == "mol" {
                    fs::remove_file(&destination).expect("remove destination");
                    fs::create_dir(&destination).expect("block destination");
                }
            },
        };

        let receipt = execute_bead_request_with_runner(&request, &runner).expect("receipt");
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded);
        assert_eq!(
            receipt.stages.last().map(|s| s.stage),
            Some(BeadStage::Pour)
        );
        assert!(
            destination.is_dir(),
            "nothing rewrote the destination after pour"
        );
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
            receipt
                .stages
                .iter()
                .map(|stage| stage.stage)
                .collect::<Vec<_>>(),
            [
                BeadStage::Render,
                BeadStage::Validate,
                BeadStage::ResolveActiveRegistry,
                BeadStage::PreviewPour
            ]
        );
        assert_eq!(
            calls
                .iter()
                .map(|call| call.args[0].as_str())
                .collect::<Vec<_>>(),
            ["cook", "where", "mol"]
        );
        let cook_input = Path::new(&calls[0].args[1]);
        let canonical_registry = fs::canonicalize(&registry).expect("canonical registry");
        assert_eq!(
            cook_input.parent().map(public_path_display),
            Some(public_path_display(&canonical_registry))
        );
        assert!(cook_input.to_string_lossy().ends_with(".formula.toml"));
        assert_ne!(cook_input, request.rendered_formula);
        assert_eq!(
            &calls[0].args[2..],
            [
                "--dry-run",
                "--json",
                "--var",
                "alpha=first",
                "--var",
                "zebra=last"
            ]
        );
        assert_eq!(calls[1].args, ["where", "--json"]);
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
    fn preview_uses_graph_mode_outside_the_active_registry() {
        let root = workspace();
        let active_beads_dir = root.join(".beads");
        fs::create_dir_all(&active_beads_dir).expect("create active beads dir");
        let runner = FakeRunner::with_outputs([
            success("{}"),
            where_output(&active_beads_dir),
            success(
                r#"{"formula":"example","type":"workflow","steps":[{"id":"build","title":"Build"}]}"#,
            ),
            success("{}"),
        ]);
        let mut request = request(&root, BeadOperation::PreviewPour);
        request.bead_variables.clear();
        let receipt = execute_bead_request_with_runner(&request, &runner).expect("receipt");
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded);
        assert_eq!(receipt.pour_mode, Some(crate::BeadPourMode::Graph));
        let calls = runner.calls.lock().expect("calls lock");
        assert_eq!(&calls[3].args[..2], ["create", "--graph"]);
        assert!(calls[3].args.contains(&"--dry-run".to_owned()));
        drop(calls);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
