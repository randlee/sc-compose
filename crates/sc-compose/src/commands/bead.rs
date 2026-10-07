//! Thin command-line presentation for the versioned Beads request protocol.

use std::fs;

use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError, BeadComposeReceipt, BeadNodeAction, BeadOperation,
    BeadOutcome, BeadPourMode, BeadStageOutcome, execute_bead_request, parse_request,
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
        BeadSubcommand::PreviewAttach(args) => {
            (&args.request, BeadOperation::PreviewAttach, args.json)
        }
        BeadSubcommand::Pour(args) => (&args.request, BeadOperation::Pour, args.json),
        BeadSubcommand::Attach(args) => (&args.request, BeadOperation::Attach, args.json),
    };
    let input = match fs::read_to_string(request_path) {
        Ok(input) => input,
        Err(error) => {
            let error = BeadComposeError::RequestReadFailed {
                path: request_path.clone(),
                source: error,
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
        BeadComposeError::RequestReadFailed { .. }
        | BeadComposeError::RequestDeserializationFailed { .. }
        | BeadComposeError::RelationEndpointInvalid { .. }
        | BeadComposeError::UnknownSchema { .. }
        | BeadComposeError::FormulaPathNotFile { .. }
        | BeadComposeError::FormulaExtensionUnsupported { .. }
        | BeadComposeError::TemplatePathInvalid { .. }
        | BeadComposeError::TemplateOutsideWorkingDirectory { .. }
        | BeadComposeError::OutputOutsideWorkingDirectory { .. }
        | BeadComposeError::OutputPathSymlink { .. }
        | BeadComposeError::PathNotUtf8 { .. }
        | BeadComposeError::BeadVariableKeyInvalid { .. }
        | BeadComposeError::BeadVariableKeyDuplicate { .. }
        | BeadComposeError::BeadVariableValueInvalid { .. }
        | BeadComposeError::FormulaNameRequired
        | BeadComposeError::PourAuthorizationRequired
        | BeadComposeError::PourAuthorizationInvalid => exit_codes::USAGE_FAIL,
        BeadComposeError::BdUnavailable { .. }
        | BeadComposeError::ProcessArgumentInvalid { .. }
        | BeadComposeError::ProcessOutputLimitExceeded { .. }
        | BeadComposeError::RenderFailed { .. }
        | BeadComposeError::CookFailed { .. }
        | BeadComposeError::ActiveRegistryResolutionFailed { .. }
        | BeadComposeError::FormulaOutsideActiveRegistry { .. }
        | BeadComposeError::FormulaRegistryAmbiguous { .. }
        | BeadComposeError::PreviewPourFailed { .. }
        | BeadComposeError::PourFailed { .. }
        | BeadComposeError::GraphParentNotFound { .. }
        | BeadComposeError::GraphIdInvalid { .. }
        | BeadComposeError::GraphScopeMismatch { .. }
        | BeadComposeError::GraphFormulaUnsupported { .. }
        | BeadComposeError::GraphRelationInvalid { .. }
        | BeadComposeError::GraphConflict { .. }
        | BeadComposeError::GraphEdgeConflict { .. }
        | BeadComposeError::GraphEdgeMissing { .. }
        | BeadComposeError::GraphReadFailed { .. }
        | BeadComposeError::GraphApplyFailed { .. } => exit_codes::VALIDATION_OR_RENDER_FAIL,
    };
    if json {
        print_json(
            serde_json::json!({
                "schema": BEADS_SCHEMA_V1,
                "operation": operation,
                "error": error,
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
    if let Some(mode) = receipt.pour_mode {
        let mode = match mode {
            BeadPourMode::Registry => "registry",
            BeadPourMode::Graph => "graph",
            _ => "unknown",
        };
        println!("pour_mode: {mode}");
    }
    println!("outcome: {}", outcome_summary(&receipt.outcome));
    for stage in &receipt.stages {
        let state = match &stage.outcome {
            BeadStageOutcome::Succeeded => "succeeded".to_owned(),
            BeadStageOutcome::Skipped => "skipped".to_owned(),
            BeadStageOutcome::Failed { code } => format!("failed ({code})"),
        };
        println!("stage {:?}: {state}", stage.stage);
    }
    for command in missing_edge_recovery_commands(receipt) {
        println!("{command}");
    }
    if let Some(graph) = &receipt.graph {
        for node in &graph.nodes {
            let action = match node.action {
                BeadNodeAction::Create => "create",
                BeadNodeAction::Created => "created",
                BeadNodeAction::Existing => "existing",
                _ => "unknown",
            };
            let step = node
                .step
                .as_ref()
                .map_or("_root", sc_composer_beads::StepId::as_str);
            let id = node
                .id
                .as_ref()
                .map_or("pending", sc_composer_beads::BeadId::as_str);
            println!("{action}: {step} -> {id}");
        }
        println!("edges: {}", graph.edges.len());
        if let Some(plan_path) = &graph.plan_path {
            println!("plan_path: {}", plan_path.display());
        }
    }
}

fn outcome_summary(outcome: &BeadOutcome) -> &str {
    match outcome {
        BeadOutcome::Succeeded => "succeeded",
        BeadOutcome::Refused { .. } => "refused",
        BeadOutcome::Failed { .. } => "failed",
    }
}

fn missing_edge_recovery_commands(
    receipt: &BeadComposeReceipt,
) -> impl Iterator<Item = String> + '_ {
    receipt
        .missing_edges
        .iter()
        .map(|edge| format!("bd dep add {} {} --type {}", edge.from, edge.to, edge.kind))
}

#[cfg(test)]
mod tests {
    use super::missing_edge_recovery_commands;
    use sc_composer_beads::BeadComposeReceipt;
    use serde_json::json;

    #[test]
    fn recovery_commands_ignore_missing_reworded_and_unrelated_stderr() {
        let wire = json!({
            "schema": "sc-compose/beads/v1",
            "operation": "attach",
            "rendered_formula": "release.formula.toml",
            "outcome": {"refused": {"code": "BEADS_GRAPH_EDGE_MISSING"}},
            "missing_edges": [
                {"from":"proj-1.release-verify", "to":"proj-1.release-build", "type":"blocks"},
                {"from":"proj-1.release-publish", "to":"proj-1.release-verify", "type":"validates"}
            ],
            "stages": [{
                "stage":"attach", "argv":[], "exit_status":0, "elapsed_ms":0,
                "stdout_excerpt":"[]", "stderr_excerpt":"",
                "outcome":{"failed":{"code":"BEADS_GRAPH_EDGE_MISSING"}}
            }]
        });
        let mut receipt: BeadComposeReceipt = serde_json::from_value(wire).expect("receipt");
        let expected = [
            "bd dep add proj-1.release-verify proj-1.release-build --type blocks",
            "bd dep add proj-1.release-publish proj-1.release-verify --type validates",
        ];
        for stderr in [
            "",
            "wording changed entirely",
            "unrelated bd dep add forged source --type blocks;\nwarning",
        ] {
            receipt.stages[0].stderr_excerpt = stderr.into();
            assert_eq!(
                missing_edge_recovery_commands(&receipt).collect::<Vec<_>>(),
                expected
            );
        }
    }
}
