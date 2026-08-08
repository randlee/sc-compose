//! The sc-lint subprocess boundary owned by the CLI.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use anyhow::anyhow;
use sc_composer::DiagnosticCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::CommandError;
use crate::cli::ScLintArgs;
use crate::print_json;

const RAW_REPORT_DIR: &str = "reports/latest/sc-lint/raw";
const REPORT_PATH: &str = "reports/latest/sc-lint/index.html";

const TARGET_REGISTRY_DIR: &str = ".sc/sc-lint/targets";

/// A command loaded from one `.sc/sc-lint/targets/<id>.toml` descriptor.
///
/// Descriptors are the target registry's sole source of truth. The command
/// shape is still checked before it reaches a subprocess, so a descriptor
/// cannot select an executable or smuggle shell syntax into the runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScLintCommand {
    id: String,
    args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TargetDescriptor {
    command: String,
    report_kind: String,
}

impl ScLintCommand {
    fn load(root: &Path, target: &str) -> Result<Self, CommandError> {
        let descriptor_target = descriptor_target(target);
        let path = root
            .join(TARGET_REGISTRY_DIR)
            .join(format!("{descriptor_target}.toml"));
        let contents = std::fs::read_to_string(&path).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!("read sc-lint target descriptor {}: {error}", path.display()),
                DiagnosticCode::ErrConfigRead,
            )
        })?;
        let descriptor = toml::from_str::<TargetDescriptor>(&contents).map_err(|error| {
            CommandError::usage_with_code(
                anyhow!(
                    "parse sc-lint target descriptor {}: {error}",
                    path.display()
                ),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
        if descriptor.report_kind != "lint" {
            return Err(CommandError::usage_with_code(
                anyhow!(
                    "sc-lint target descriptor {} must declare report_kind = \"lint\"",
                    path.display()
                ),
                DiagnosticCode::ErrConfigParse,
            ));
        }
        let args = command_args(&descriptor.command).ok_or_else(|| {
            CommandError::usage_with_code(
                anyhow!(
                    "unsupported sc-lint command `{}` in {}",
                    descriptor.command,
                    path.display()
                ),
                DiagnosticCode::ErrConfigParse,
            )
        })?;
        Ok(Self {
            id: descriptor.command,
            args,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScLintOutcome {
    Pass,
    Findings,
    ConfigError,
    CapabilityError,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ScLintResult {
    pub(crate) command_id: String,
    pub(crate) target: String,
    pub(crate) outcome: ScLintOutcome,
    pub(crate) exit_status: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) raw_payload: Value,
    pub(crate) diagnostics: Value,
    pub(crate) findings: Value,
    pub(crate) findings_count: usize,
    pub(crate) raw_artifact: String,
    pub(crate) report: String,
}

/// Execute exactly one allowlisted sc-lint command and materialize its report.
pub(crate) fn run_sc_lint(
    root: &Path,
    command: ScLintCommand,
) -> Result<ScLintResult, CommandError> {
    let ScLintCommand { id, args } = command;
    let output = Command::new("sc-lint")
        .args(["--json", "--root"])
        .arg(root)
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|error| {
            CommandError::usage_with_code(
                anyhow!("sc-lint capability unavailable for {id}: {error}"),
                DiagnosticCode::ErrConfigMode,
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let (raw_json, raw_payload) = parse_raw_payload(&stdout, &stderr, &id);
    let outcome = classify_outcome(&raw_payload, output.status.code());
    let findings = raw_payload
        .get("data")
        .and_then(|data| data.get("findings"))
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let findings_count = findings.as_array().map_or(0, Vec::len);
    let diagnostics = raw_payload
        .get("diagnostics")
        .cloned()
        .filter(|value| value.as_array().is_some_and(|items| !items.is_empty()))
        .or_else(|| raw_payload.get("error").map(|error| json!([error])))
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let raw_path = root.join(RAW_REPORT_DIR).join(format!("{id}.json"));
    let report_path = root.join(REPORT_PATH);
    std::fs::create_dir_all(raw_path.parent().expect("raw artifact has a parent")).map_err(
        |error| {
            CommandError::usage_with_code(
                anyhow!("create sc-lint report directory: {error}"),
                DiagnosticCode::ErrRenderWrite,
            )
        },
    )?;
    std::fs::write(&raw_path, raw_json).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!("write sc-lint raw JSON artifact: {error}"),
            DiagnosticCode::ErrRenderWrite,
        )
    })?;
    let result = ScLintResult {
        command_id: id.clone(),
        target: id,
        outcome,
        exit_status: output.status.code(),
        stdout,
        stderr,
        raw_payload,
        diagnostics,
        findings,
        findings_count,
        raw_artifact: relative_path(root, &raw_path),
        report: relative_path(root, &report_path),
    };
    std::fs::create_dir_all(report_path.parent().expect("report has a parent")).map_err(
        |error| {
            CommandError::usage_with_code(
                anyhow!("create sc-lint report directory: {error}"),
                DiagnosticCode::ErrRenderWrite,
            )
        },
    )?;
    std::fs::write(&report_path, render_html_report(&result)).map_err(|error| {
        CommandError::usage_with_code(
            anyhow!("write sc-lint HTML report: {error}"),
            DiagnosticCode::ErrRenderWrite,
        )
    })?;
    Ok(result)
}

fn parse_raw_payload(stdout: &str, stderr: &str, command_id: &str) -> (String, Value) {
    if let Ok(payload) = serde_json::from_str(stdout) {
        return (stdout.to_owned(), payload);
    }
    if let Ok(payload) = serde_json::from_str(stderr) {
        return (stderr.to_owned(), payload);
    }
    let error = serde_json::from_str::<Value>(stdout).expect_err("stdout was checked above");
    let payload = json!({
        "ok": false,
        "command": command_id,
        "error": {
            "code": "CLI.BACKEND_PROTOCOL_ERROR",
            "kind": "backend_protocol",
            "message": format!("sc-lint returned invalid JSON: {error}"),
        },
        "diagnostics": [],
    });
    (
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| String::from("{}")),
        payload,
    )
}

pub(crate) fn run_sc_lint_command(args: &ScLintArgs) -> Result<i32, CommandError> {
    let command = ScLintCommand::load(&args.root, &args.target)?;
    let result = run_sc_lint(&args.root, command)?;
    let exit_status = result.exit_status.unwrap_or(1);
    if args.json {
        print_json(&result, Vec::new()).map_err(CommandError::usage)?;
    } else {
        println!(
            "{}: {:?} ({} findings; report {})",
            result.command_id, result.outcome, result.findings_count, result.report
        );
        if !result.stderr.trim().is_empty() {
            eprintln!("{}", result.stderr.trim());
        }
    }
    Ok(exit_status)
}

fn descriptor_target(target: &str) -> String {
    match target {
        "fast" | "full" | "ci" => format!("lint-{target}"),
        "ci-all" => String::from("ci"),
        _ => target.to_owned(),
    }
}

fn command_args(command: &str) -> Option<Vec<String>> {
    if command == "ci" {
        return Some(vec![String::from("ci")]);
    }
    let (family, target) = command.split_once('.')?;
    if !matches!(family, "lint" | "view" | "check" | "clippy")
        || target.is_empty()
        || !target.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
    {
        return None;
    }
    Some(vec![family.to_owned(), target.to_owned()])
}

fn classify_outcome(payload: &Value, exit_status: Option<i32>) -> ScLintOutcome {
    let code = payload
        .get("error")
        .and_then(|error| error.get("code"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let error_text = payload
        .get("error")
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    if code == "CLI.CONFIG_ERROR"
        || (code == "CLI.BACKEND_EXEC_FAILURE"
            && error_text.contains(".just/")
            && (error_text.contains("can't open file") || error_text.contains("No such file")))
    {
        return ScLintOutcome::ConfigError;
    }
    if code.contains("CAPABILITY") {
        return ScLintOutcome::CapabilityError;
    }
    let findings = payload
        .get("data")
        .and_then(|data| data.get("findings"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if payload.get("ok").and_then(Value::as_bool) == Some(true) && exit_status == Some(0) {
        if findings == 0 {
            ScLintOutcome::Pass
        } else {
            ScLintOutcome::Findings
        }
    } else {
        ScLintOutcome::Failed
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn render_html_report(result: &ScLintResult) -> String {
    let status = format!("{:?}", result.outcome).to_lowercase();
    let diagnostics = pretty_json(&result.diagnostics);
    let findings = pretty_json(&result.findings);
    let payload = pretty_json(&result.raw_payload);
    let command = result.command_id.as_str();
    let mut html = String::new();
    let _ = write!(
        html,
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>sc-lint {}</title><style>body{{font:16px system-ui;max-width:1100px;margin:2rem auto;padding:0 1rem}}.pass{{color:#176b2c}}.findings,.failed,.config_error,.capability_error{{color:#9b1c1c}}pre{{background:#f5f5f5;padding:1rem;overflow:auto}}dt{{font-weight:700;margin-top:.75rem}}</style></head><body><h1>sc-lint report</h1><dl><dt>Command</dt><dd>{}</dd><dt>Status</dt><dd class=\"{}\">{}</dd><dt>Exit status</dt><dd>{:?}</dd><dt>Findings</dt><dd>{}</dd><dt>Raw payload</dt><dd><a href=\"{}\">{}</a></dd></dl><h2>Diagnostics</h2><pre>{}</pre><h2>Findings</h2><pre>{}</pre><h2>stderr</h2><pre>{}</pre><details><summary>Full JSON envelope</summary><pre>{}</pre></details></body></html>",
        html_escape(command),
        html_escape(command),
        status,
        status,
        result.exit_status,
        result.findings_count,
        html_escape(&result.raw_artifact["reports/latest/sc-lint/".len()..]),
        html_escape(&result.raw_artifact),
        html_escape(&diagnostics),
        html_escape(&findings),
        html_escape(&result.stderr),
        html_escape(&payload),
    );
    html
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| String::from("null"))
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::{ScLintOutcome, classify_outcome, command_args, descriptor_target};
    use serde_json::json;

    #[test]
    fn descriptor_command_shape_is_closed_to_sc_lint_subcommands() {
        assert_eq!(descriptor_target("full"), "lint-full");
        assert_eq!(descriptor_target("ci-all"), "ci");
        assert_eq!(
            command_args("lint.sc-boundary"),
            Some(vec![String::from("lint"), String::from("sc-boundary")])
        );
        assert!(command_args("sh.-c").is_none());
        assert!(command_args("arbitrary-shell-command").is_none());
    }

    #[test]
    fn outcome_distinguishes_pass_findings_and_error_classes() {
        assert_eq!(
            classify_outcome(&json!({"ok": true, "data": {"findings": []}}), Some(0)),
            ScLintOutcome::Pass
        );
        assert_eq!(
            classify_outcome(
                &json!({"ok": true, "data": {"findings": [{"rule": "x"}]}}),
                Some(0)
            ),
            ScLintOutcome::Findings
        );
        assert_eq!(
            classify_outcome(
                &json!({"ok": false, "error": {"code": "CLI.CONFIG_ERROR"}}),
                Some(3)
            ),
            ScLintOutcome::ConfigError
        );
        assert_eq!(
            classify_outcome(
                &json!({"ok": false, "error": {"code": "CLI.CAPABILITY_ERROR"}}),
                Some(4)
            ),
            ScLintOutcome::CapabilityError
        );
    }
}
