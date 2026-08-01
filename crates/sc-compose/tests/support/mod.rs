#![allow(
    dead_code,
    reason = "shared helpers are selected by separate integration-test binaries"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub fn temp_root(label: &str) -> PathBuf {
    temp_root_with_prefix(label, "sc-compose-test")
}

fn temp_root_with_prefix(label: &str, prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("{prefix}-{label}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

pub fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

pub fn sc_compose() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-compose"));
    command.env("SC_LOG_ROOT", test_log_root("sc-compose-test"));
    command
}

fn test_log_root(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("{prefix}-logs-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

pub fn parse_stdout(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

pub fn assert_envelope(value: &Value) {
    assert_eq!(value["schema_version"], "1");
    assert!(value.get("payload").is_some());
    assert!(!value["payload"].is_null(), "payload must not be null");
    assert!(
        value["diagnostics"].is_array(),
        "diagnostics must be a JSON array, got: {:?}",
        value["diagnostics"]
    );
}

pub fn assert_first_code(value: &Value, code: &str) {
    assert_eq!(value["diagnostics"][0]["code"], code);
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

pub fn normalize_path_str(p: impl AsRef<Path>) -> String {
    let path = p.as_ref().to_string_lossy();
    let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
    path.replace('\\', "/")
}

pub fn write_report_catalog(root: &Path, contents: &str) {
    write_file(
        &root.join("reports").join("catalog").join("reports.toml"),
        contents,
    );
}

#[allow(
    dead_code,
    reason = "shared CLI support is only used by selected branch-local tests"
)]
pub fn valid_report_catalog() -> &'static str {
    r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#
}

pub fn write_smoke_fixture(root: &Path) {
    write_file(
        &root
            .join("reports")
            .join("smoke")
            .join("reference-template.html.j2"),
        "---\nrequired_variables:\n  - title\n  - summary\n---\n<html><body><h1>{{ title }}</h1><p>{{ summary }}</p></body></html>\n",
    );
    write_file(
        &root.join("reports").join("smoke").join("sample-vars.json"),
        "{ \"title\": \"Smoke Report\", \"summary\": \"fixture\" }\n",
    );
}

pub fn write_render_many_fixture(root: &Path) {
    write_file(
        &root.join("reports").join("templates").join("panel.html.j2"),
        "<article>{{ metadata.title }}|{{ body }}|{{ output_path }}{% if sets %}|{{ sets | join(\",\") }}{% endif %}</article>\n",
    );
}

pub fn write_report_family_override(root: &Path) {
    write_file(
        &root
            .join("reports")
            .join("templates")
            .join("lint")
            .join("report.html.j2"),
        "{% extends \"base/report.html.j2\" %}\n{% block report_header %}<header class=\"report-header report-header-lint\"><h1>Lint override</h1><p>Lint override</p></header>{% endblock %}\n{% block panel_body %}<div class=\"panel-body panel-body-lint\">Override body marker</div>{% endblock %}\n",
    );
}

pub fn write_state_machine_spec(root: &Path, relative: &str) {
    write_file(
        &root.join(relative),
        r#"[spec]
kind = "state_machine"
id = "state-diagrams"
title = "State Diagrams"
renderer_targets = ["mermaid"]

[metadata]
sets = ["publish", "diagram"]

[[states]]
id = "accepted"
label = "Accepted"

[[states]]
id = "validated"
label = "Validated"
terminal = true

[[transitions]]
from = "accepted"
to = "validated"
event = "validate_ok"
guard = "input_valid"
effect = "store message"
"#,
    );
}

pub fn write_sql_query_spec(root: &Path, relative: &str) {
    write_file(
        &root.join(relative),
        r#"[spec]
kind = "sql_query"
id = "sql-diagrams"
title = "SQL Diagrams"
renderer_targets = ["mermaid"]

[sql_query]
purpose = "Summarize shipped orders"
tables_read = ["orders", "customers"]
tables_written = ["report_cache"]
filters = ["status = shipped"]
ordering = ["created_at DESC"]
cardinality = "many"
transactional_assumptions = ["read committed"]

[metadata]
sets = ["publish", "diagram"]
"#,
    );
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(&path, &target).unwrap();
        }
    }
}

pub fn stage_phase_b_reference_assets(root: &Path) {
    copy_dir_all(&repo_root().join("examples"), &root.join("examples"));
    copy_dir_all(&repo_root().join("reports"), &root.join("reports"));
}

pub fn render_report_summary(root: &Path, vars_path: &str, output_path: &str) {
    if let Some(parent) = root.join(output_path).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(root)
        .arg("--file")
        .arg("examples/report-evidence-summary.html.j2")
        .arg("--var-file")
        .arg(root.join(vars_path))
        .arg("--output")
        .arg(root.join(output_path))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

pub fn write_large_report_evidence_vars(path: &Path) {
    let items = (0..160)
        .map(|idx| {
            serde_json::json!({
                "section_key": if idx % 2 == 0 { "evidence" } else { "diagrams" },
                "label": format!("Generated item {idx}"),
                "href": format!("reports/latest/generated/{idx}.html"),
                "status": if idx % 3 == 0 { serde_json::Value::String("latest".to_owned()) } else { serde_json::Value::Null },
                "note": format!("Generated note {}", "x".repeat(24)),
            })
        })
        .collect::<Vec<_>>();
    let payload = serde_json::json!({
        "report": {
            "title": "Large Report Evidence Summary",
            "family": "Phase B proof vehicle",
            "generated_at": "2026-05-27T03:40:00Z"
        },
        "summary": {
            "status": "PASS",
            "note": "Large payload compatibility check."
        },
        "sections": [
            { "key": "evidence", "title": "Evidence" },
            { "key": "diagrams", "title": "Diagrams" }
        ],
        "items": items
    });
    write_file(path, &serde_json::to_string_pretty(&payload).unwrap());
}

pub fn finalize_report(
    root: &Path,
    report_id: &str,
    kind: &str,
    entrypoint: &str,
    artifacts: &[&str],
) {
    let mut command = sc_compose();
    command
        .arg("reports")
        .arg("finalize")
        .arg("--root")
        .arg(root)
        .arg("--report-id")
        .arg(report_id)
        .arg("--kind")
        .arg(kind)
        .arg("--entrypoint")
        .arg(entrypoint)
        .arg("--archive");
    for artifact in artifacts {
        command.arg("--artifact").arg(artifact);
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[cfg(unix)]
pub fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
pub fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    use std::os::windows::fs::symlink_file;

    match symlink_file(target, link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(_) => false,
    }
}
