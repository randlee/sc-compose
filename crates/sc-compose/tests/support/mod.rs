use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub fn temp_root(label: &str, prefix: &str) -> PathBuf {
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

pub fn sc_compose(log_prefix: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-compose"));
    command.env("SC_LOG_ROOT", test_log_root(log_prefix));
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

#[allow(dead_code)]
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
