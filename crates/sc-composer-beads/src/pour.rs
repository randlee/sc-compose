//! Registry resolution and preview/persistent pour staging.

use crate::contract::{
    BeadComposeReceipt, BeadComposeRequest, BeadOperation, BeadOutcome, BeadStage, BeadStageReceipt,
};
use crate::error::BeadComposeError;
use crate::execute::{
    NormalizedRequest, append_variables, failed_last_stage_receipt, receipt, run_stage,
    run_stage_with_output,
};
use crate::runner::{CommandSpec, ProcessRunner};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn execute_pour(
    request: &BeadComposeRequest,
    runner: &dyn ProcessRunner,
    normalized: NormalizedRequest,
    bd: PathBuf,
    mut stages: Vec<BeadStageReceipt>,
) -> Result<BeadComposeReceipt, BeadComposeError> {
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
    use crate::execute::execute_bead_request_with_runner;
    use crate::execute::tests::{FakeRunner, request, success, where_output, workspace};
    use crate::{BeadOperation, BeadOutcome};
    use std::fs;

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
}
