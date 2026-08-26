//! Thin command-line presentation for the versioned Beads request protocol.

use std::fs;

use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError, BeadComposeReceipt, BeadOperation, BeadOutcome,
    BeadStageOutcome, execute_bead_request, parse_request,
};

use crate::CommandError;
use crate::cli::{BeadArgs, BeadSubcommand};
use crate::exit_codes;
use crate::print_json;

/// Run one Beads request selected by the CLI subcommand.
pub(crate) fn run_bead(args: &BeadArgs) -> Result<i32, CommandError> {
    let (request_path, operation, json) = match &args.command {
        BeadSubcommand::Render(args) => (&args.request, BeadOperation::Render, args.json),
        BeadSubcommand::Validate(args) => (&args.request, BeadOperation::Validate, args.json),
        BeadSubcommand::PreviewPour(args) => (&args.request, BeadOperation::PreviewPour, args.json),
        BeadSubcommand::Pour(args) => (&args.request, BeadOperation::Pour, args.json),
    };
    let input = match fs::read_to_string(request_path) {
        Ok(input) => input,
        Err(error) => {
            let error = BeadComposeError::RequestDeserializationFailed {
                message: format!("read {}: {error}", request_path.display()),
            };
            return print_bead_error(&error, operation, json);
        }
    };
    let mut request = match parse_request(&input) {
        Ok(request) => request,
        Err(error) => return print_bead_error(&error, operation, json),
    };
    request.operation = operation;

    match execute_bead_request(&request) {
        Ok(receipt) => print_receipt(receipt, json),
        Err(error) => print_bead_error(&error, operation, json),
    }
}

fn print_receipt(receipt: BeadComposeReceipt, json: bool) -> Result<i32, CommandError> {
    let exit_code = match &receipt.outcome {
        BeadOutcome::Succeeded => exit_codes::SUCCESS,
        BeadOutcome::Refused { .. } | BeadOutcome::Failed { .. } => {
            exit_codes::VALIDATION_OR_RENDER_FAIL
        }
    };
    if json {
        print_json(receipt, Vec::new()).map_err(CommandError::usage)?;
    } else {
        print_human_receipt(&receipt);
    }
    Ok(exit_code)
}

fn print_bead_error(
    error: &BeadComposeError,
    operation: BeadOperation,
    json: bool,
) -> Result<i32, CommandError> {
    let exit_code = match &error {
        BeadComposeError::RequestDeserializationFailed { .. }
        | BeadComposeError::UnknownSchema { .. }
        | BeadComposeError::FormulaPathNotFile { .. }
        | BeadComposeError::FormulaExtensionUnsupported { .. }
        | BeadComposeError::TemplatePathInvalid { .. }
        | BeadComposeError::TemplateOutsideWorkingDirectory { .. }
        | BeadComposeError::OutputOutsideWorkingDirectory { .. }
        | BeadComposeError::BeadVariableKeyInvalid { .. }
        | BeadComposeError::BeadVariableKeyDuplicate { .. }
        | BeadComposeError::FormulaNameRequired
        | BeadComposeError::PourAuthorizationRequired
        | BeadComposeError::PourAuthorizationInvalid => exit_codes::USAGE_FAIL,
        BeadComposeError::BdUnavailable { .. }
        | BeadComposeError::RenderFailed { .. }
        | BeadComposeError::CookFailed { .. }
        | BeadComposeError::ActiveRegistryResolutionFailed { .. }
        | BeadComposeError::FormulaOutsideActiveRegistry { .. }
        | BeadComposeError::FormulaRegistryAmbiguous { .. }
        | BeadComposeError::PreviewPourFailed { .. }
        | BeadComposeError::PourFailed { .. } => exit_codes::VALIDATION_OR_RENDER_FAIL,
    };
    if json {
        print_json(
            serde_json::json!({
                "schema": BEADS_SCHEMA_V1,
                "operation": operation,
                "error": { "code": error.code(), "message": error.to_string() },
            }),
            Vec::new(),
        )
        .map_err(CommandError::usage)?;
    } else {
        eprintln!("{}: {error}", error.code());
    }
    Ok(exit_code)
}

fn print_human_receipt(receipt: &BeadComposeReceipt) {
    println!("rendered_formula: {}", receipt.rendered_formula.display());
    println!("outcome: {}", outcome_summary(&receipt.outcome));
    for stage in &receipt.stages {
        let state = match &stage.outcome {
            BeadStageOutcome::Succeeded => "succeeded".to_owned(),
            BeadStageOutcome::Skipped => "skipped".to_owned(),
            BeadStageOutcome::Failed { code } => format!("failed ({code})"),
        };
        println!("stage {:?}: {state}", stage.stage);
    }
}

fn outcome_summary(outcome: &BeadOutcome) -> &str {
    match outcome {
        BeadOutcome::Succeeded => "succeeded",
        BeadOutcome::Refused { .. } => "refused",
        BeadOutcome::Failed { .. } => "failed",
    }
}
