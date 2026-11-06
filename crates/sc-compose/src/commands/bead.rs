//! Thin command-line presentation for the versioned Beads request protocol.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sc_composer_beads::error::{escape_human_text, shell_quote};
use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeError, BeadComposeReceipt, BeadComposeRequest, BeadDiagnostic,
    BeadErrorClass, BeadExecutionOptions, BeadNodeAction, BeadOperation, BeadOutcome, BeadPourMode,
    BeadRenderBy, BeadStageOutcome, FormulaName, PourAuthorization, RefusedBeadComposeReceipt,
    RequestParseOutcome, execute_bead_request_with_options_and_diagnostics,
    parse_request_for_operation,
};

use crate::CommandError;
use crate::cli::{BeadArgs, BeadRenderByArg, BeadRequestArgs, BeadSubcommand};
use crate::exit_codes;
use crate::print_json;

/// Run one Beads request selected by the CLI subcommand.
pub(crate) fn run_bead(args: &BeadArgs) -> Result<i32, CommandError> {
    let (args, operation) = match &args.command {
        BeadSubcommand::Render(args) => (args, BeadOperation::Render),
        BeadSubcommand::Validate(args) => (args, BeadOperation::Validate),
        BeadSubcommand::PreviewPour(args) => (args, BeadOperation::PreviewPour),
        BeadSubcommand::PreviewAttach(args) => (args, BeadOperation::PreviewAttach),
        BeadSubcommand::Pour(args) => (args, BeadOperation::Pour),
        BeadSubcommand::Attach(args) => (args, BeadOperation::Attach),
    };
    let json = args.json;
    let options = BeadExecutionOptions {
        render_by: match args.render_by {
            BeadRenderByArg::ScCompose => BeadRenderBy::ScCompose,
            BeadRenderByArg::Bd => BeadRenderBy::Bd,
        },
        strict: args.strict,
    };
    let mut staging = None;
    let mut request = if let Some(request_path) = &args.request {
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
        match parse_request_for_operation(&input, operation) {
            Ok(RequestParseOutcome::Ready(request)) => request,
            Ok(RequestParseOutcome::Refused(mut receipt)) => {
                receipt.receipt.operation = operation;
                return print_refused_receipt(&receipt, json, &[]);
            }
            // An identifier error that survives parsing is outside the attach
            // family (attach identifiers become refused receipts): ADR-0023 calls
            // it a request error, exit 3, keeping the native typed error.
            Err(error @ BeadComposeError::GraphIdInvalid { .. }) => {
                return print_bead_error_with_exit(&error, operation, json, exit_codes::USAGE_FAIL);
            }
            Err(error) => return print_bead_error(&error, operation, json),
        }
    } else {
        match direct_pour_request(args, operation) {
            Ok((request, dir)) => {
                staging = Some(dir);
                request
            }
            Err(error) => return print_bead_error(&error, operation, json),
        }
    };
    request.operation = operation;

    let mut diagnostics = Vec::new();
    let mut identifier_diagnostic = None;
    let result =
        execute_bead_request_with_options_and_diagnostics(&request, &options, &mut |error| {
            identifier_diagnostic = BeadDiagnostic::graph_id_invalid(error);
            if !json {
                diagnostics.push(serde_json::to_value(error));
            }
        });
    // The staged files are removed before the receipt is printed, so any
    // receipt path inside the staging directory is flagged as ephemeral.
    let ephemeral = match (&result, &staging) {
        (Ok(receipt), Some(staging)) => staging.ephemeral_fields(receipt),
        _ => Vec::new(),
    };
    drop(staging);
    match result {
        Ok(receipt) => {
            if let Some(error) = identifier_diagnostic {
                return print_refused_receipt(
                    &RefusedBeadComposeReceipt { receipt, error },
                    json,
                    &ephemeral,
                );
            }
            let diagnostics = diagnostics
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| CommandError::usage(error.into()))?;
            print_receipt(&receipt, json, &diagnostics, &ephemeral)
        }
        Err(error) => print_bead_error(&error, operation, json),
    }
}

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Receipt fields a direct pour reports inside its private staging directory.
const RENDERED_FORMULA_FIELD: &str = "rendered_formula";
const PLAN_PATH_FIELD: &str = "graph.plan_path";

/// Private staging directory of a direct pour, removed when the command ends.
struct DirectStaging {
    path: PathBuf,
    canonical: Option<PathBuf>,
}

impl DirectStaging {
    fn new(path: PathBuf) -> Self {
        let canonical = fs::canonicalize(&path).ok();
        Self { path, canonical }
    }

    fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.path)
            || self
                .canonical
                .as_deref()
                .is_some_and(|canonical| path.starts_with(canonical))
    }

    /// Receipt fields whose paths are removed with this staging directory.
    fn ephemeral_fields(&self, receipt: &BeadComposeReceipt) -> Vec<&'static str> {
        let plan_path = receipt
            .graph
            .as_ref()
            .and_then(|graph| graph.plan_path.as_deref());
        [
            (
                RENDERED_FORMULA_FIELD,
                Some(receipt.rendered_formula.as_path()),
            ),
            (PLAN_PATH_FIELD, plan_path),
        ]
        .into_iter()
        .filter_map(|(field, path)| path.filter(|path| self.contains(path)).map(|_| field))
        .collect()
    }
}

impl Drop for DirectStaging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn direct_pour_request(
    args: &BeadRequestArgs,
    operation: BeadOperation,
) -> Result<(BeadComposeRequest, DirectStaging), BeadComposeError> {
    let bad = |message: String| BeadComposeError::RequestDeserializationFailed { message };
    if operation != BeadOperation::Pour {
        return Err(bad("a direct formula is supported only by bead pour".into()));
    }
    let mut variables = std::collections::BTreeMap::new();
    for (key, value) in &args.vars {
        if variables.insert(key.clone(), value.clone()).is_some() {
            return Err(BeadComposeError::BeadVariableKeyDuplicate { key: key.clone() });
        }
    }
    let root = std::env::current_dir().map_err(|error| bad(error.to_string()))?;
    let operand = args
        .formula
        .as_deref()
        .ok_or_else(|| bad("formula is required".into()))?;
    let mut source = resolve_direct_formula(&root, operand)
        .ok_or_else(|| bad(format!("formula source not found: {operand}")))?;
    let original_source = fs::canonicalize(&source).map_err(|error| bad(error.to_string()))?;
    let canonical_root = fs::canonicalize(&root).map_err(|error| bad(error.to_string()))?;
    let external_snapshot = capture_external_source(&original_source, &canonical_root, operand)?;
    let name = direct_formula_name(&source)?;
    let formula_name = FormulaName::new(&name)?;
    let dir = root.join(format!(
        ".sc-compose-pour-{}-{}",
        std::process::id(),
        STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).map_err(|error| {
        bad(format!(
            "cannot create private formula staging directory: {error}"
        ))
    })?;
    let staging = DirectStaging::new(dir.clone());
    if let Some((source_root, files)) = external_snapshot {
        source = write_source_snapshot(&dir, &source_root, &original_source, files)?;
    }
    let extension = if source.to_string_lossy().ends_with(".formula.json.j2")
        || source.to_string_lossy().ends_with(".formula.json")
    {
        "json"
    } else {
        "toml"
    };
    let staged_output = dir.join(format!("{name}.formula.{extension}"));
    let rendered_formula = if matches!(args.render_by, BeadRenderByArg::Bd) {
        active_formulas(&root)
            .filter(|registry| {
                original_source
                    .parent()
                    .and_then(|parent| fs::canonicalize(parent).ok())
                    == fs::canonicalize(registry).ok()
            })
            .map_or(staged_output, |registry| {
                registry.join(format!("{name}.formula.{extension}"))
            })
    } else {
        staged_output
    };
    let request = BeadComposeRequest {
        schema: BEADS_SCHEMA_V1.into(),
        operation,
        working_directory: root,
        template: source,
        rendered_formula,
        compose_variables: serde_json::Map::new(),
        formula_name: Some(formula_name),
        bead_variables: variables,
        bd_executable: None,
        pour_authorization: Some(PourAuthorization::CreatePersistentBeads),
        parent: None,
        ref_: None,
        relations: Vec::new(),
    };
    Ok((request, staging))
}

type SourceSnapshot = (PathBuf, std::collections::BTreeMap<PathBuf, String>);

fn capture_external_source(
    source: &Path,
    workspace: &Path,
    operand: &str,
) -> Result<Option<SourceSnapshot>, BeadComposeError> {
    if source.starts_with(workspace) || operand.contains(['/', '\\']) {
        return Ok(None);
    }
    let bad = |message: String| BeadComposeError::RequestDeserializationFailed { message };
    let root = source
        .parent()
        .ok_or_else(|| bad("formula source has no parent".into()))?
        .to_path_buf();
    let confined =
        sc_composer::ConfiningRoot::new(&root).map_err(|error| bad(error.to_string()))?;
    let expanded =
        sc_composer::expand_includes(source, &confined, &sc_composer::ComposePolicy::default())
            .map_err(|error| bad(format!("cannot snapshot formula source: {error}")))?;
    Ok(Some((root, expanded.source_texts)))
}

fn direct_formula_name(source: &Path) -> Result<String, BeadComposeError> {
    source.file_name().and_then(|value| value.to_str()).and_then(|value| {
        value.strip_suffix(".formula.toml.j2")
            .or_else(|| value.strip_suffix(".formula.json.j2"))
            .or_else(|| value.strip_suffix(".formula.toml"))
            .or_else(|| value.strip_suffix(".formula.json"))
    }).map(str::to_owned).ok_or_else(|| BeadComposeError::RequestDeserializationFailed {
        message: "formula source must end in .formula.toml or .formula.json, optionally followed by .j2".into(),
    })
}

fn write_source_snapshot(
    dir: &Path,
    source_root: &Path,
    source: &Path,
    files: std::collections::BTreeMap<PathBuf, String>,
) -> Result<PathBuf, BeadComposeError> {
    let bad = |message: String| BeadComposeError::RequestDeserializationFailed { message };
    let snapshot_root = dir.join("source");
    for (path, contents) in files {
        let relative = path
            .strip_prefix(source_root)
            .map_err(|_error| bad("formula include escapes its registry root".into()))?;
        let destination = snapshot_root.join(relative);
        let parent = destination
            .parent()
            .ok_or_else(|| bad("formula snapshot has no parent".into()))?;
        fs::create_dir_all(parent)
            .and_then(|()| fs::write(&destination, contents))
            .map_err(|error| bad(format!("cannot snapshot formula source: {error}")))?;
    }
    let relative = source
        .strip_prefix(source_root)
        .map_err(|_error| bad("formula escapes registry root".into()))?;
    Ok(snapshot_root.join(relative))
}

fn active_formulas(root: &Path) -> Option<PathBuf> {
    std::process::Command::new("bd")
        .args(["where", "--json"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| serde_json::from_slice::<serde_json::Value>(&output.stdout).ok())
        .and_then(|value| {
            value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(PathBuf::from)
        })
        .map(|path| path.join("formulas"))
}

fn resolve_direct_formula(root: &Path, operand: &str) -> Option<PathBuf> {
    let path = Path::new(operand);
    let explicit = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    if explicit.is_file() {
        return Some(explicit);
    }
    if operand.contains(['/', '\\']) {
        return None;
    }
    let active = active_formulas(root);
    let mut directories = Vec::new();
    if let Some(active) = active {
        directories.push(active);
    }
    directories.push(root.join(".beads/formulas"));
    if let Some(home) = platform_home_directory() {
        directories.push(home.join(".beads/formulas"));
    }
    if let Some(shared) = std::env::var_os("GT_ROOT") {
        directories.push(PathBuf::from(shared).join(".beads/formulas"));
    }
    let native = native_formula_source(root, operand);
    for directory in directories {
        for suffix in ["formula.toml.j2", "formula.json.j2"] {
            let path = directory.join(format!("{operand}.{suffix}"));
            if path.is_file() {
                return Some(path);
            }
        }
        if let Some(path) = &native
            && path
                .parent()
                .and_then(|parent| fs::canonicalize(parent).ok())
                == fs::canonicalize(&directory).ok()
        {
            return Some(path.clone());
        }
    }
    native
}

fn platform_home_directory() -> Option<PathBuf> {
    std::env::home_dir()
}

fn native_formula_source(root: &Path, operand: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("bd")
        .args(["formula", "list", "--json"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let values: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    values
        .as_array()?
        .iter()
        .find(|entry| entry.get("name").and_then(serde_json::Value::as_str) == Some(operand))
        .and_then(|entry| entry.get("source").and_then(serde_json::Value::as_str))
        .map(PathBuf::from)
}

fn print_refused_receipt(
    receipt: &RefusedBeadComposeReceipt,
    json: bool,
    ephemeral: &[&str],
) -> Result<i32, CommandError> {
    if json {
        print_json(with_ephemeral_paths(receipt, ephemeral)?, Vec::new())
            .map_err(CommandError::usage)?;
    } else {
        let diagnostic = serde_json::to_value(&receipt.error)
            .map_err(|error| CommandError::usage(error.into()))?;
        print_human_receipt(&receipt.receipt, &[diagnostic], ephemeral);
    }
    Ok(exit_codes::VALIDATION_OR_RENDER_FAIL)
}

/// The JSON receipt plus an additive `ephemeral_paths` list naming the
/// receipt fields whose files were removed when the command ended. The field
/// is absent when every reported path is kept.
fn with_ephemeral_paths(
    receipt: &impl serde::Serialize,
    ephemeral: &[&str],
) -> Result<serde_json::Value, CommandError> {
    let mut value =
        serde_json::to_value(receipt).map_err(|error| CommandError::usage(error.into()))?;
    if !ephemeral.is_empty()
        && let Some(fields) = value.as_object_mut()
    {
        fields.insert("ephemeral_paths".into(), serde_json::json!(ephemeral));
    }
    Ok(value)
}

/// Human label for a receipt path, flagged when the command removed it.
fn human_path(path: &Path, field: &str, ephemeral: &[&str]) -> String {
    let text = escape_human_text(&path.display().to_string());
    if ephemeral.contains(&field) {
        format!("{text} (ephemeral: removed when this command exited)")
    } else {
        text
    }
}

fn print_receipt(
    receipt: &BeadComposeReceipt,
    json: bool,
    diagnostics: &[serde_json::Value],
    ephemeral: &[&str],
) -> Result<i32, CommandError> {
    let exit_code = match &receipt.outcome {
        BeadOutcome::Succeeded => exit_codes::SUCCESS,
        BeadOutcome::Refused { .. } | BeadOutcome::Failed { .. } => {
            exit_codes::VALIDATION_OR_RENDER_FAIL
        }
    };
    if json {
        print_json(with_ephemeral_paths(receipt, ephemeral)?, Vec::new())
            .map_err(CommandError::usage)?;
    } else {
        print_human_receipt(receipt, diagnostics, ephemeral);
    }
    Ok(exit_code)
}

fn print_bead_error(
    error: &BeadComposeError,
    operation: BeadOperation,
    json: bool,
) -> Result<i32, CommandError> {
    let exit_code = bead_error_exit_code(error);
    print_bead_error_with_exit(error, operation, json, exit_code)
}

fn bead_error_exit_code(error: &BeadComposeError) -> i32 {
    match error.class() {
        BeadErrorClass::Request => exit_codes::USAGE_FAIL,
        // An execution failure, or a class added by a newer library.
        _ => exit_codes::VALIDATION_OR_RENDER_FAIL,
    }
}

fn print_bead_error_with_exit(
    error: &BeadComposeError,
    operation: BeadOperation,
    json: bool,
    exit_code: i32,
) -> Result<i32, CommandError> {
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
    let mut output = format!(
        "{}: {}",
        error.code(),
        escape_human_text(&error.to_string())
    );
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
            let text = value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned);
            output.push_str(&escape_human_text(&text));
        }
    }
    output
}

fn print_human_receipt(
    receipt: &BeadComposeReceipt,
    diagnostics: &[serde_json::Value],
    ephemeral: &[&str],
) {
    println!(
        "rendered_formula: {}",
        human_path(&receipt.rendered_formula, RENDERED_FORMULA_FIELD, ephemeral)
    );
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
            println!(
                "stage {:?}: {state}: {}",
                stage.stage,
                escape_human_text(&stage.stderr_excerpt)
            );
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
            println!(
                "plan_path: {}",
                human_path(plan_path, PLAN_PATH_FIELD, ephemeral)
            );
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
    fn missing_edge_recovery_commands_escape_controls_and_bidi_for_posix_sh() {
        let from = "source'\\\u{7}\u{7f}\u{80}\u{202e}$(literal)$HOME`literal`";
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
            !command.chars().any(|c| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}')),
            "{command:?}"
        );
        let arguments = command
            .strip_prefix("bd dep add ")
            .unwrap()
            .strip_suffix(" --type 'blocks'")
            .unwrap();
        let separators = "line\u{2028}paragraph\u{2029}end";
        let escaped = super::shell_quote(separators);
        assert!(!escaped.contains(['\u{2028}', '\u{2029}']));
        assert!(!arguments.contains("$'"), "{arguments}");
        #[cfg(unix)]
        {
            // The word is POSIX: the system sh must read it back exactly.
            let output = std::process::Command::new("/bin/sh")
                .args(["-c", &format!("printf '%s\\0' {arguments}")])
                .env("LC_ALL", "C.UTF-8")
                .output()
                .expect("POSIX sh printf");
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, format!("{from}\0{to}\0").into_bytes());
        }
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

    #[test]
    fn human_receipt_paths_escape_terminal_controls() {
        let hostile = "out\u{1b}[2Jforged: line\u{202e}.toml";
        let escaped = super::escape_human_text(hostile);
        assert!(!escaped.chars().any(char::is_control), "{escaped:?}");
        assert!(escaped.contains("\\u{001B}"), "{escaped:?}");
    }

    #[test]
    fn human_refusal_message_escapes_terminal_controls_and_format_characters() {
        let error = BeadComposeError::GraphScopeMismatch {
            field: GraphIdField::Parent,
            value: "item\u{7}\u{202e}\u{200b}\u{feff}\u{2028}\u{2029}\u{00ad}".into(),
        };

        let first_line = human_bead_error(&error).unwrap();
        let first_line = first_line.lines().next().unwrap();
        assert!(!first_line.chars().any(char::is_control), "{first_line:?}");
        assert!(
            !first_line.chars().any(|character| matches!(
                character,
                '\u{2028}' | '\u{2029}' | '\u{00ad}' | '\u{200b}' | '\u{feff}' | '\u{202e}'
            )),
            "{first_line:?}"
        );
        for escaped in [
            "\\u{0007}",
            "\\u{202E}",
            "\\u{200B}",
            "\\u{FEFF}",
            "\\u{2028}",
            "\\u{2029}",
            "\\u{00AD}",
        ] {
            assert!(first_line.contains(escaped), "{first_line:?}");
        }
    }
}
