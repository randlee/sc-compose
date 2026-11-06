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
    let (request, staging) = match request_from_args(args, operation, json) {
        Ok(prepared) => prepared,
        Err(response) => return response,
    };
    let mut diagnostics = Vec::new();
    let mut refusal_diagnostic = None;
    let result =
        execute_bead_request_with_options_and_diagnostics(&request, &options, &mut |error| {
            refusal_diagnostic = BeadDiagnostic::refused(error);
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
            if let Some(error) = refusal_diagnostic {
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

fn request_from_args(
    args: &BeadRequestArgs,
    operation: BeadOperation,
    json: bool,
) -> Result<(BeadComposeRequest, Option<DirectStaging>), Result<i32, CommandError>> {
    let mut staging = None;
    let mut request = if let Some(request_path) = &args.request {
        let input = match fs::read_to_string(request_path) {
            Ok(input) => input,
            Err(error) => {
                let error = BeadComposeError::RequestReadFailed {
                    path: request_path.clone(),
                    source: error,
                };
                return Err(print_bead_error(&error, operation, json));
            }
        };
        match parse_request_for_operation(&input, operation) {
            Ok(RequestParseOutcome::Ready(request)) => request,
            Ok(RequestParseOutcome::Refused(mut receipt)) => {
                receipt.receipt.operation = operation;
                return Err(print_refused_receipt(&receipt, json, &[]));
            }
            // An identifier error that survives parsing is outside the attach
            // family (attach identifiers become refused receipts): ADR-0023 calls
            // it a request error, exit 3, keeping the native typed error.
            Err(error @ BeadComposeError::GraphIdInvalid { .. }) => {
                return Err(print_bead_error_with_exit(
                    &error,
                    operation,
                    json,
                    exit_codes::USAGE_FAIL,
                ));
            }
            Err(error) => return Err(print_bead_error(&error, operation, json)),
        }
    } else {
        match direct_pour_request(args, operation) {
            Ok((request, dir)) => {
                staging = Some(dir);
                request
            }
            Err(error) => return Err(print_bead_error(&error, operation, json)),
        }
    };
    request.operation = operation;
    Ok((request, staging))
}

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Receipt fields a direct pour reports inside its private staging directory.
const RENDERED_FORMULA_FIELD: &str = "rendered_formula";
const PLAN_PATH_FIELD: &str = "graph.plan_path";

/// Private staging directory of a direct pour, removed when the command ends.
struct DirectStaging {
    path: PathBuf,
    /// Every spelling a receipt path inside this directory can take: the
    /// requested path and its canonical form, each without a Windows verbatim
    /// prefix. The engine canonicalizes the output parent and prints it through
    /// its public display, so on Windows a receipt names the canonical long path
    /// without `\\?\`, which neither the raw (possibly 8.3) path nor the raw
    /// canonical (`\\?\`-prefixed) path matches.
    spellings: Vec<String>,
}

impl DirectStaging {
    /// Takes ownership of an already created staging directory; it is removed
    /// on drop even when resolving its canonical spelling fails.
    fn new(path: PathBuf) -> std::io::Result<Self> {
        let mut staging = Self {
            path,
            spellings: Vec::new(),
        };
        let canonical = fs::canonicalize(&staging.path)?;
        staging.spellings = [staging.path.as_path(), canonical.as_path()]
            .into_iter()
            .map(|spelling| without_verbatim_prefix(&spelling.to_string_lossy()).into_owned())
            .collect();
        Ok(staging)
    }

    fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.path) || staging_contains(&self.spellings, path)
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

/// Whether `path`, compared without a Windows verbatim prefix, is one of the
/// staging `spellings` or lies beneath one. Pure string comparison so the
/// Windows spellings are exercised identically on every host.
fn staging_contains(spellings: &[String], path: &Path) -> bool {
    let text = path.to_string_lossy();
    let candidate = without_verbatim_prefix(&text);
    spellings.iter().any(|spelling| {
        let dir = spelling.trim_end_matches(['/', '\\']);
        !dir.is_empty()
            && candidate
                .strip_prefix(dir)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '\\']))
    })
}

/// The spelling the engine's public path display gives a canonical Windows
/// path: `\\?\C:\x` becomes `C:\x` and `\\?\UNC\server\share\x` becomes
/// `\\server\share\x`; every other spelling is unchanged.
fn without_verbatim_prefix(text: &str) -> std::borrow::Cow<'_, str> {
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return std::borrow::Cow::Owned(format!(r"\\{rest}"));
    }
    match text.strip_prefix(r"\\?\") {
        Some(rest)
            if rest.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                && rest.as_bytes().get(1) == Some(&b':') =>
        {
            std::borrow::Cow::Borrowed(rest)
        }
        _ => std::borrow::Cow::Borrowed(text),
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
    let staging = DirectStaging::new(dir.clone()).map_err(|error| {
        bad(format!(
            "cannot resolve private formula staging directory: {error}"
        ))
    })?;
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
    use super::{
        DirectStaging, PLAN_PATH_FIELD, RENDERED_FORMULA_FIELD, human_bead_error,
        missing_edge_recovery_commands, staging_contains, without_verbatim_prefix,
    };
    use sc_composer_beads::{
        BeadComposeError, BeadComposeReceipt, BeadId, GraphConflictReason, GraphDependencyType,
        GraphIdField, MissingEdge,
    };
    use serde_json::json;
    use std::path::{Path, PathBuf};

    /// Staging directory spelling of the hosted Windows failure (PR954 run
    /// 42196008975): `fs::canonicalize` returns the verbatim form.
    const WINDOWS_CANONICAL_STAGING: &str = r"\\?\C:\Users\runneradmin\AppData\Local\Temp\sc-compose-fixture-bead-graph-e2e-6748-1793958490840348900-11\.sc-compose-pour-212-0";
    /// The engine's receipt spelling of a file in that directory.
    const WINDOWS_RECEIPT_FORMULA: &str = r"C:\Users\runneradmin\AppData\Local\Temp\sc-compose-fixture-bead-graph-e2e-6748-1793958490840348900-11\.sc-compose-pour-212-0\plan-fix.formula.toml";

    /// Staging spellings as `DirectStaging::new` derives them from a raw path
    /// and its canonical form.
    fn spellings(raw: &str, canonical: &str) -> Vec<String> {
        [raw, canonical]
            .into_iter()
            .map(|spelling| without_verbatim_prefix(spelling).into_owned())
            .collect()
    }

    /// A staging value with controlled spellings whose drop touches nothing:
    /// its own path names a directory this test never creates.
    fn controlled_staging(raw: &str, canonical: &str) -> DirectStaging {
        DirectStaging {
            path: std::env::temp_dir().join(format!(
                "sc-compose-never-created-staging-{}",
                std::process::id()
            )),
            spellings: spellings(raw, canonical),
        }
    }

    #[test]
    fn verbatim_prefix_is_removed_like_the_engine_public_display() {
        for (input, expected) in [
            (r"\\?\C:\Users\a\b", r"C:\Users\a\b"),
            (r"\\?\d:\x", r"d:\x"),
            (r"\\?\UNC\server\share\x", r"\\server\share\x"),
            (r"C:\Users\a", r"C:\Users\a"),
            (r"\\server\share\x", r"\\server\share\x"),
            (r"\\?\Volume{0000}\x", r"\\?\Volume{0000}\x"),
            ("/tmp/x", "/tmp/x"),
        ] {
            assert_eq!(without_verbatim_prefix(input), expected, "{input}");
        }
    }

    #[test]
    fn windows_receipt_spelling_inside_verbatim_canonical_staging_is_contained() {
        // The raw spelling is the 8.3 TEMP form; only the canonical one names
        // the long path the engine prints.
        let staging = spellings(
            r"C:\Users\RUNNER~1\AppData\Local\Temp\sc-compose-fixture-bead-graph-e2e-6748-1793958490840348900-11\.sc-compose-pour-212-0",
            WINDOWS_CANONICAL_STAGING,
        );
        for receipt in [
            WINDOWS_RECEIPT_FORMULA,
            &format!("{WINDOWS_RECEIPT_FORMULA}.graph.json"),
            &format!(r"\\?\{WINDOWS_RECEIPT_FORMULA}"),
            &WINDOWS_CANONICAL_STAGING[4..],
        ] {
            assert!(staging_contains(&staging, Path::new(receipt)), "{receipt}");
        }
    }

    #[test]
    fn verbatim_unc_staging_contains_unc_receipt_spelling() {
        let staging = spellings(
            r"\\server\share\ws\.sc-compose-pour-1-0",
            r"\\?\UNC\server\share\ws\.sc-compose-pour-1-0",
        );
        assert!(staging_contains(
            &staging,
            Path::new(r"\\server\share\ws\.sc-compose-pour-1-0\f.formula.toml")
        ));
    }

    #[test]
    fn paths_outside_staging_are_not_contained() {
        let staging = spellings(WINDOWS_CANONICAL_STAGING, WINDOWS_CANONICAL_STAGING);
        let (outside_root, _) = WINDOWS_CANONICAL_STAGING[4..]
            .rsplit_once('\\')
            .expect("staging parent");
        for path in [
            // Sibling staging directories sharing a name prefix.
            format!(
                "{}0\\plan-fix.formula.toml",
                &WINDOWS_CANONICAL_STAGING[4..]
            ),
            format!(
                "{}-x\\plan-fix.formula.toml",
                &WINDOWS_CANONICAL_STAGING[4..]
            ),
            format!("{outside_root}\\plan-fix.formula.toml"),
            r"D:\Users\runneradmin\plan-fix.formula.toml".to_owned(),
            "plan-fix.formula.toml".to_owned(),
        ] {
            assert!(!staging_contains(&staging, Path::new(&path)), "{path}");
        }
        assert!(!staging_contains(&[String::new()], Path::new("anything")));
    }

    #[test]
    fn hosted_windows_receipt_flags_both_ephemeral_fields() {
        let staging = controlled_staging(
            r"C:\Users\RUNNER~1\AppData\Local\Temp\x\.sc-compose-pour-212-0",
            WINDOWS_CANONICAL_STAGING,
        );
        let receipt = |formula: &str| -> BeadComposeReceipt {
            serde_json::from_value(json!({
                "schema": "sc-compose/beads/v1",
                "operation": "pour",
                "rendered_formula": formula,
                "outcome": "succeeded",
                "graph": {
                    "mode": "pour", "formula": "plan-fix", "revision": "sha256:fb4897d9ac390385c62c7012c02b8c003aeceac7bbdcabbbd944c32865fa44cc",
                    "parent": "cli-73p", "ids": {}, "nodes": [], "edges": [],
                    "plan_path": format!("{formula}.graph.json")
                },
                "stages": []
            }))
            .expect("receipt")
        };
        assert_eq!(
            staging.ephemeral_fields(&receipt(WINDOWS_RECEIPT_FORMULA)),
            [RENDERED_FORMULA_FIELD, PLAN_PATH_FIELD]
        );
        assert!(
            staging
                .ephemeral_fields(&receipt(r"C:\Users\runneradmin\kept.formula.toml"))
                .is_empty()
        );
    }

    #[test]
    fn real_staging_directory_flags_canonical_children_and_is_removed() {
        let dir = std::env::temp_dir().join(format!(
            "sc-compose-staging-unit-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir(&dir).expect("create staging");
        let staging = DirectStaging::new(dir.clone()).expect("staging");
        let canonical: PathBuf = std::fs::canonicalize(&dir).expect("canonical");
        assert!(staging.contains(&dir.join("f.formula.toml")));
        assert!(staging.contains(&canonical.join("f.formula.toml")));
        assert!(!staging.contains(&dir.with_extension("other").join("f.formula.toml")));
        drop(staging);
        assert!(!dir.exists(), "staging must be removed on drop");
    }

    #[test]
    fn unresolvable_staging_directory_is_an_explicit_error() {
        let missing = std::env::temp_dir().join(format!(
            "sc-compose-staging-missing-{}-{}",
            std::process::id(),
            line!()
        ));
        assert!(DirectStaging::new(missing).is_err());
    }

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
        let from = "source'\\\u{202e}$(literal)$HOME`literal`";
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
        let escaped_controls = super::shell_quote("\u{7}\u{7f}\u{80}");
        assert!(!escaped_controls.chars().any(char::is_control));
        assert!(escaped_controls.contains("\\007\\177\\302\\200"));
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
