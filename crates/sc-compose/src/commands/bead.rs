//! Thin command-line presentation for the versioned Beads request protocol.

use std::fs;

use sc_composer_beads::error::shell_quote;
use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError, BeadComposeReceipt, BeadDiagnostic, BeadNodeAction,
    BeadOperation, BeadOutcome, BeadPourMode, BeadStageOutcome, RefusedBeadComposeReceipt,
    RequestParseOutcome, execute_bead_request_with_diagnostics, parse_request_with_outcome,
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
    let mut request = match parse_request_with_outcome(&input) {
        Ok(RequestParseOutcome::Ready(request)) => request,
        Ok(RequestParseOutcome::Refused(mut receipt)) => {
            receipt.receipt.operation = operation;
            return print_refused_receipt(receipt, json);
        }
        Err(error) => return print_bead_error(&error, operation, json),
    };
    request.operation = operation;

    let mut diagnostics = Vec::new();
    let mut identifier_diagnostic = None;
    let result = execute_bead_request_with_diagnostics(&request, &mut |error| {
        identifier_diagnostic = BeadDiagnostic::graph_id_invalid(error);
        if !json {
            diagnostics.push(serde_json::to_value(error));
        }
    });
    match result {
        Ok(receipt) => {
            if let Some(error) = identifier_diagnostic {
                return print_refused_receipt(RefusedBeadComposeReceipt { receipt, error }, json);
            }
            let diagnostics = diagnostics
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| CommandError::usage(error.into()))?;
            print_receipt(receipt, json, &diagnostics)
        }
        Err(error) => print_bead_error(&error, operation, json),
    }
}

fn print_refused_receipt(
    receipt: RefusedBeadComposeReceipt,
    json: bool,
) -> Result<i32, CommandError> {
    if json {
        print_json(receipt, Vec::new()).map_err(CommandError::usage)?;
    } else {
        let diagnostic = serde_json::to_value(&receipt.error)
            .map_err(|error| CommandError::usage(error.into()))?;
        print_human_receipt(&receipt.receipt, &[diagnostic]);
    }
    Ok(exit_codes::VALIDATION_OR_RENDER_FAIL)
}

fn print_receipt(
    receipt: BeadComposeReceipt,
    json: bool,
    diagnostics: &[serde_json::Value],
) -> Result<i32, CommandError> {
    let exit_code = match &receipt.outcome {
        BeadOutcome::Succeeded => exit_codes::SUCCESS,
        BeadOutcome::Refused { .. } | BeadOutcome::Failed { .. } => {
            exit_codes::VALIDATION_OR_RENDER_FAIL
        }
    };
    if json {
        print_json(receipt, Vec::new()).map_err(CommandError::usage)?;
    } else {
        print_human_receipt(&receipt, diagnostics);
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
        | BeadComposeError::OutputPathInvalid { .. }
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
        eprintln!(
            "{}",
            human_bead_error(error).map_err(|error| CommandError::usage(error.into()))?
        );
    }
    Ok(exit_code)
}

/// Present the library's canonical error envelope without duplicating its recovery rules.
fn human_bead_error(error: &BeadComposeError) -> Result<String, serde_json::Error> {
    let envelope = serde_json::to_value(error)?;
    let mut output = format!("{}: {error}", error.code());
    output.push_str(&human_error_fields(&envelope));
    Ok(output)
}

fn human_error_fields(envelope: &serde_json::Value) -> String {
    let mut output = String::new();
    if let Some(fields) = envelope.as_object() {
        for (name, value) in fields {
            if matches!(name.as_str(), "code" | "message") {
                continue;
            }
            output.push('\n');
            output.push_str(name);
            output.push_str(": ");
            if let Some(text) = value.as_str() {
                output.push_str(text);
            } else {
                output.push_str(&value.to_string());
            }
        }
    }
    output
}

fn print_human_receipt(receipt: &BeadComposeReceipt, diagnostics: &[serde_json::Value]) {
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
        if !matches!(stage.outcome, BeadStageOutcome::Failed { .. })
            || stage.stderr_excerpt.is_empty()
        {
            println!("stage {:?}: {state}", stage.stage);
        } else {
            println!("stage {:?}: {state}: {}", stage.stage, stage.stderr_excerpt);
        }
    }
    for diagnostic in diagnostics {
        println!(
            "{}",
            human_error_fields(diagnostic).trim_start_matches('\n')
        );
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
    receipt.missing_edges.iter().map(|edge| {
        format!(
            "bd dep add {} {} --type {}",
            shell_quote(edge.from.as_str()),
            shell_quote(edge.to.as_str()),
            shell_quote(&edge.kind.to_string())
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{human_bead_error, missing_edge_recovery_commands};
    use sc_composer_beads::{
        BeadComposeError, BeadComposeReceipt, BeadId, GraphConflictReason, GraphDependencyType,
        GraphIdField, MissingEdge,
    };
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
            "bd dep add 'proj-1.release-verify' 'proj-1.release-build' --type 'blocks'",
            "bd dep add 'proj-1.release-publish' 'proj-1.release-verify' --type 'validates'",
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
    #[test]
    fn missing_edge_recovery_commands_escape_controls_and_bidi_for_bash() {
        let from = "source'\\\u{7}\u{7f}\u{80}\u{202e}";
        let to = "target\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}\u{200e}\u{200f}";
        let receipt: BeadComposeReceipt = serde_json::from_value(json!({
            "schema": "sc-compose/beads/v1",
            "operation": "attach",
            "rendered_formula": "release.formula.toml",
            "outcome": {"refused": {"code": "BEADS_GRAPH_EDGE_MISSING"}},
            "missing_edges": [{"from":from, "to":to, "type":"blocks"}],
            "stages": []
        }))
        .expect("receipt with unusual IDs");
        let commands: Vec<_> = missing_edge_recovery_commands(&receipt).collect();
        assert_eq!(commands.len(), 1);
        let command = &commands[0];
        assert!(!command.chars().any(char::is_control), "{command:?}");
        assert!(
            !command.chars().any(|c| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}')),
            "{command:?}"
        );
        let arguments = command
            .strip_prefix("bd dep add ")
            .unwrap()
            .strip_suffix(" --type 'blocks'")
            .unwrap();
        let output = std::process::Command::new("bash")
            .args(["-c", &format!("printf '%s\\0' {arguments}")])
            .env("LC_ALL", "C.UTF-8")
            .output()
            .expect("isolated Bash printf");
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, format!("{from}\0{to}\0").into_bytes());
    }

    #[test]
    fn human_errors_preserve_library_recovery_details_and_causes() {
        let errors = [
            BeadComposeError::GraphEdgeMissing {
                edges: vec![MissingEdge {
                    from: BeadId::new("parent.build").unwrap(),
                    to: BeadId::new("parent.test").unwrap(),
                    kind: GraphDependencyType::try_from("blocks".to_owned()).unwrap(),
                }],
            },
            BeadComposeError::GraphConflict {
                id: BeadId::new("parent.build").unwrap(),
                reason: GraphConflictReason::NotOwned,
            },
            BeadComposeError::GraphScopeMismatch {
                field: GraphIdField::Parent,
                value: "other-parent".into(),
            },
            BeadComposeError::RequestReadFailed {
                path: "missing-request.json".into(),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            },
            BeadComposeError::GraphReadFailed {
                command: vec!["bd".into(), "show".into(), "parent".into()],
                status: Some(7),
                cause: "malformed JSON from bd".into(),
            },
        ];
        for error in errors {
            let envelope = serde_json::to_value(&error).unwrap();
            let human = human_bead_error(&error).unwrap();
            assert!(human.starts_with(&format!("{}: {error}\n", error.code())));
            assert!(human.contains(&format!(
                "\nrecovery: {}",
                envelope["recovery"].as_str().unwrap()
            )));
            let details = human
                .lines()
                .find_map(|line| line.strip_prefix("details: "))
                .unwrap();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(details).unwrap(),
                envelope["details"]
            );
        }
    }

    #[test]
    fn human_errors_without_additive_fields_keep_existing_text() {
        let error = BeadComposeError::UnknownSchema {
            actual: "future".into(),
        };
        assert_eq!(
            human_bead_error(&error).unwrap(),
            format!("{}: {error}", error.code())
        );
    }
}
