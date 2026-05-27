use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sc-compose-json-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn sc_compose() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-compose"));
    command.env("SC_LOG_ROOT", test_log_root());
    command
}

fn test_log_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sc-compose-json-logs-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn parse_stdout(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn write_report_catalog(root: &Path, contents: &str) {
    write_file(
        &root.join("reports").join("catalog").join("reports.toml"),
        contents,
    );
}

fn write_smoke_fixture(root: &Path) {
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

fn write_render_many_fixture(root: &Path) {
    write_file(
        &root.join("reports").join("templates").join("panel.html.j2"),
        "<article>{{ metadata.title }}|{{ body }}|{{ output_path }}{% if sets %}|{{ sets | join(\",\") }}{% endif %}</article>\n",
    );
}

fn write_report_family_override(root: &Path) {
    write_file(
        &root
            .join("reports")
            .join("templates")
            .join("lint")
            .join("report.html.j2"),
        "{% extends \"base/report.html.j2\" %}\n{% block report_header %}<header class=\"report-header report-header-lint\"><h1>Lint override</h1><p>Lint override</p></header>{% endblock %}\n{% block panel_body %}<div class=\"panel-body panel-body-lint\">Override body marker</div>{% endblock %}\n",
    );
}

fn assert_envelope(value: &Value) {
    assert_eq!(value["schema_version"], "1");
    assert!(value.get("payload").is_some());
    assert!(!value["payload"].is_null(), "payload must not be null");
    assert!(
        value["diagnostics"].is_array(),
        "diagnostics must be a JSON array, got: {:?}",
        value["diagnostics"]
    );
}

fn assert_first_code(value: &Value, code: &str) {
    assert_eq!(value["diagnostics"][0]["code"], code);
}

#[test]
fn render_json_uses_diagnostic_envelope() {
    let root = temp_root("render-json");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["output_path"], "stdout");
}

#[test]
fn render_dry_run_json_uses_diagnostic_envelope() {
    let root = temp_root("render-dry-run-json");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(value["payload"]["would_write"].is_string());
    assert_eq!(
        value["payload"]["template"],
        fs::canonicalize(root.join("template.md.j2"))
            .unwrap()
            .display()
            .to_string()
    );
    assert_eq!(value["payload"]["would_change"], true);
}

#[test]
fn render_dry_run_json_reports_no_change_when_output_matches() {
    let root = temp_root("render-dry-run-json-no-change");
    let output_path = root.join("out.md");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    write_file(&output_path, "hello world");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&output_path)
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["would_change"], false);
}

#[test]
fn resolve_json_uses_diagnostic_envelope() {
    let root = temp_root("resolve-json");
    write_file(
        &root.join(".claude").join("agents").join("example.md"),
        "agent",
    );

    let output = sc_compose()
        .arg("resolve")
        .arg("--mode")
        .arg("profile")
        .arg("--root")
        .arg(&root)
        .arg("--kind")
        .arg("agent")
        .arg("--agent")
        .arg("example")
        .arg("--runtime")
        .arg("claude")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["found"], true);
}

#[test]
fn validate_json_uses_diagnostic_envelope() {
    let root = temp_root("validate-json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["valid"], false);
    assert_eq!(value["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_first_code(&value, "ERR_VAL_MISSING_REQUIRED");
    assert_eq!(value["diagnostics"][0]["line"], 3);
    assert_eq!(value["diagnostics"][0]["column"], 5);
}

#[test]
fn validate_json_reports_missing_frontmatter_for_included_file() {
    let root = temp_root("validate-json-included-missing-frontmatter");
    write_file(
        &root.join("_includes").join("snippet.md"),
        "hello {{ name }}\n",
    );
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\n@<_includes/snippet.md>\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "ERR_VAL_MISSING_FRONTMATTER"
            && diagnostic["path"]
                == fs::canonicalize(root.join("_includes").join("snippet.md"))
                    .unwrap()
                    .display()
                    .to_string()
    }));
}

#[test]
fn frontmatter_init_json_uses_diagnostic_envelope() {
    let root = temp_root("frontmatter-init-json");
    let path = root.join("template.md.j2");
    write_file(&path, "hello {{ name }}\n");

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["template_path"],
        fs::canonicalize(&path).unwrap().display().to_string()
    );
    assert_eq!(value["payload"]["frontmatter_added"], true);
    assert_eq!(value["payload"]["would_change"], true);
    assert_eq!(value["payload"]["vars"][0], "name");
}

#[test]
fn frontmatter_init_dry_run_json_uses_diagnostic_envelope() {
    let root = temp_root("frontmatter-init-dry-run-json");
    let path = root.join("template.md.j2");
    write_file(&path, "hello {{ name }}\n");

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&path)
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["action"], "frontmatter-init");
    assert_eq!(value["payload"]["changed"], false);
    assert_eq!(value["payload"]["would_change"], true);
    assert_eq!(value["payload"]["skipped"], false);
    assert_eq!(value["payload"]["vars"][0], "name");
}

#[test]
fn init_json_uses_diagnostic_envelope() {
    let root = temp_root("init-json");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["workspace_root"],
        fs::canonicalize(&root).unwrap().display().to_string()
    );
}

#[test]
fn init_json_created_files_reflect_actual_files_written() {
    let root = temp_root("init-json-created-files");
    write_file(&root.join(".gitignore"), "target/\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["created_files"],
        serde_json::json!([".prompts/", ".gitignore"])
    );
}

#[test]
fn init_dry_run_json_uses_diagnostic_envelope() {
    let root = temp_root("init-dry-run-json");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["action"], "init");
}

#[test]
fn stdin_double_read_reports_structured_error_code() {
    let root = temp_root("stdin-double-read");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--guidance-file")
        .arg("-")
        .arg("--prompt-file")
        .arg("-")
        .stdin(Stdio::piped())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_RENDER_STDIN_DOUBLE_READ"));
}

#[test]
fn render_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("render-failure-json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_VAL_MISSING_REQUIRED");
}

#[test]
fn observability_health_json_uses_diagnostic_envelope_and_stays_stdout_clean() {
    let root = temp_root("observability-health-json");

    let output = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["logging"]["state"], "Healthy");
    assert_eq!(value["payload"]["logging"]["query"]["state"], "Healthy");
    assert_eq!(
        value["payload"]["logging"]["maintenance"]["state"],
        "Running"
    );
    assert_eq!(
        value["payload"]["logging"]["active_log_path"],
        root.join("logs")
            .join("sc-compose.log.jsonl")
            .display()
            .to_string()
    );
}

#[test]
fn observability_health_json_nulls_unavailable_query_state() {
    let root = temp_root("observability-health-json-null-query");

    let output = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &root)
        .env("SC_COMPOSE_TEST_FORCE_QUERY_UNAVAILABLE", "1")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(value["payload"]["logging"]["query"].is_null());
    assert_eq!(
        value["payload"]["logging"]["maintenance"]["state"],
        "Stopped"
    );
}

#[test]
fn render_failure_json_preserves_all_validation_diagnostics() {
    let root = temp_root("render-failure-multi-json");
    write_file(
        &root.join("template.md.j2"),
        concat!(
            "---\nrequired_variables:\n  - first\n  - second\n---\n",
            "{{ first }} {{ second }}\n"
        ),
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0]["code"], "ERR_VAL_MISSING_REQUIRED");
    assert_eq!(diagnostics[1]["code"], "ERR_VAL_MISSING_REQUIRED");
}

#[test]
fn validate_json_reports_default_usage_info_for_frontmatter_input_defaults() {
    let root = temp_root("validate-default-usage-json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - task_id\ninput_defaults:\n  assignee: teammate\n---\nhello {{ task_id }} {{ assignee }}\n",
    );
    let vars_file = root.join("vars.json");
    write_file(&vars_file, r#"{ "task_id": "SC-1" }"#);

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "INFO_VAL_DEFAULT_USED"
            && diagnostic["message"]
                == r#"variable assignee not provided, using default: "teammate""#
    }));
}

#[test]
fn render_dry_run_json_reports_default_usage_info_for_template_json_defaults() {
    let root = temp_root("render-dry-run-template-default-json");
    let templates_root = root.join("templates");
    let pack_root = templates_root.join("report");
    write_file(
        &pack_root.join("template.json"),
        r#"{ "description": "Report template", "version": "1.0.0", "input_defaults": { "name": "world" } }"#,
    );
    write_file(&pack_root.join("report.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("templates")
        .arg("report")
        .arg("--dry-run")
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "INFO_VAL_DEFAULT_USED"
            && diagnostic["message"] == r#"variable name not provided, using default: "world""#
    }));
}

#[test]
fn render_json_reports_actual_bytes_written_for_output_file() {
    let root = temp_root("render-bytes-written-json");
    let output_path = root.join("out.txt");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: café\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&output_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["bytes_written"].as_u64().unwrap(),
        fs::metadata(&output_path).unwrap().len()
    );
}

#[test]
fn resolve_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("resolve-failure-json");

    let output = sc_compose()
        .arg("resolve")
        .arg("--mode")
        .arg("profile")
        .arg("--root")
        .arg(&root)
        .arg("--kind")
        .arg("agent")
        .arg("--agent")
        .arg("missing")
        .arg("--runtime")
        .arg("claude")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_RESOLVE_NOT_FOUND");
}

#[test]
fn frontmatter_init_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("frontmatter-init-failure-json");
    let path = root.join("template.md.j2");
    write_file(
        &path,
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_READONLY");
}

#[test]
fn init_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("init-failure-json");
    fs::create_dir_all(root.join(".prompts")).unwrap();
    write_file(&root.join(".gitignore"), ".prompts/\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_READONLY");
}

#[test]
fn render_write_failure_json_reports_render_write_code() {
    let root = temp_root("render-write-failure-json");
    let out_dir = root.join("out");
    fs::create_dir_all(&out_dir).unwrap();
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&out_dir)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_RENDER_WRITE");
}

#[test]
fn invalid_var_file_json_reports_config_varfile() {
    let root = temp_root("var-file-invalid-json");
    let vars_file = root.join("vars.json");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    write_file(&vars_file, "[1, 2, 3]\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_VARFILE");
}

#[test]
fn examples_list_json_uses_diagnostic_envelope() {
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .arg("--json")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let packs = value["payload"]["packs"].as_array().unwrap();
    assert!(packs.iter().any(|pack| pack["name"] == "hello"));
}

#[test]
fn examples_named_render_json_matches_render_schema() {
    let output = sc_compose()
        .arg("examples")
        .arg("hello")
        .arg("--var")
        .arg("name=Casey")
        .arg("--json")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["output_path"], "stdout");
    assert_eq!(
        value["payload"]["template"],
        repo_root()
            .join("examples")
            .join("hello.md.j2")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
}

#[test]
fn examples_named_render_html_dry_run_preserves_html_extension() {
    let vars_file = repo_root()
        .join("examples")
        .join("sprint-report-html.sample-vars.json");

    let output = sc_compose()
        .arg("examples")
        .arg("sprint-report-html")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--json")
        .arg("--dry-run")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["would_write"], "sprint-report-html.html");
    assert_eq!(
        value["payload"]["template"],
        repo_root()
            .join("examples")
            .join("sprint-report-html.html.j2")
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
}

#[test]
fn templates_list_json_uses_diagnostic_envelope() {
    let root = temp_root("templates-list-json");
    let templates_root = root.join("user-templates");
    write_file(&templates_root.join("hello").join("hello.md.j2"), "hello");

    let output = sc_compose()
        .arg("templates")
        .arg("list")
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["packs"][0]["name"], "hello");
}

#[test]
fn templates_add_json_uses_diagnostic_envelope() {
    let root = temp_root("templates-add-json");
    let templates_root = root.join("user-templates");
    let source = root.join("hello.md.j2");
    write_file(&source, "Hello {{ name }}!");

    let output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["name"], "hello");
    assert_eq!(value["payload"]["changed"], true);
}

#[test]
fn templates_add_duplicate_json_reports_template_exists_code() {
    let root = temp_root("templates-add-duplicate-json");
    let templates_root = root.join("user-templates");
    let source = root.join("hello.md.j2");
    write_file(&source, "Hello {{ name }}!");

    let first = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();
    assert!(first.status.success());

    let output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_TEMPLATE_EXISTS");
}

#[test]
fn templates_render_json_reports_pack_not_renderable_code() {
    let root = temp_root("templates-render-json-not-renderable");
    let templates_root = root.join("user-templates");
    write_file(&templates_root.join("ambiguous").join("one.md.j2"), "one");
    write_file(&templates_root.join("ambiguous").join("two.md.j2"), "two");

    let output = sc_compose()
        .arg("templates")
        .arg("ambiguous")
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_PACK_NOT_RENDERABLE");
}

#[test]
fn resolve_mode_mismatch_json_reports_config_mode() {
    let root = temp_root("resolve-mode-mismatch-json");
    write_file(&root.join("template.md.j2"), "hello\n");

    let output = sc_compose()
        .arg("resolve")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_MODE");
}

#[test]
fn init_missing_root_json_reports_config_parse() {
    let root = temp_root("init-missing-root-json").join("missing");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_PARSE");
}

#[test]
fn report_catalog_json_uses_diagnostic_envelope() {
    let root = temp_root("report-catalog-json");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("report-catalog")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["report_count"], 1);
    assert_eq!(value["payload"]["reports"][0]["id"], "sc-lint");
    assert_eq!(
        value["payload"]["reports"][0]["metadata"],
        "reports/latest/sc-lint/report.json"
    );
}

#[test]
fn report_catalog_invalid_json_reports_config_parse() {
    let root = temp_root("report-catalog-invalid-json");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = "yes"
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("report-catalog")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert_first_code(&value, "ERR_CONFIG_PARSE");
    assert_eq!(
        value["diagnostics"][0]["message"],
        "report 'sc-lint' field 'required' must be true or false"
    );
}

#[test]
fn reports_init_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-init-json");

    let output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["created_paths"][0], "reports/latest/");
    assert!(
        value["payload"]["created_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "reports/catalog/reports.toml")
    );
}

#[test]
fn reports_smoke_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-smoke-json");
    write_smoke_fixture(&root);

    let output = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["entrypoint"],
        "reports/latest/smoke/index.html"
    );
    assert_eq!(
        value["payload"]["metadata"],
        "reports/latest/smoke/report.json"
    );
}

#[test]
fn reports_index_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-index-json");

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .arg("--catalog")
        .arg("reports/catalog/reports.toml")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["subcommand"], "reports index");
    assert_eq!(value["payload"]["status"], "reserved");
}

#[test]
fn reports_verify_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-verify-json");

    let output = sc_compose()
        .arg("reports")
        .arg("verify")
        .arg("--root")
        .arg(&root)
        .arg("--catalog")
        .arg("reports/catalog/reports.toml")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["subcommand"], "reports verify");
    assert_eq!(value["payload"]["status"], "reserved");
}

#[test]
fn report_render_many_json_uses_diagnostic_envelope() {
    let root = temp_root("report-render-many-json");
    write_render_many_fixture(&root);
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "/*\ntitle: Alpha\nsets:\n  - publish\n*/\nalpha body\n",
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("reports/templates/panel.html.j2")
        .arg("--output-dir")
        .arg("reports/latest/panels")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["manifest_path"],
        "reports/latest/panels/manifest.json"
    );
    assert_eq!(
        value["payload"]["entries"][0]["source_path"],
        "docs/diagrams/a.txt"
    );
    assert_eq!(
        value["payload"]["entries"][0]["output_path"],
        "reports/latest/panels/docs/diagrams/a.html"
    );
    assert_eq!(
        value["payload"]["entries"][0]["sets"],
        serde_json::json!(["publish"])
    );
}

#[test]
fn report_render_many_json_supports_shared_template_family() {
    let root = temp_root("report-render-many-shared-family-json");
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "# title: Alpha\n# fragment_href: /reports/alpha\nalpha body\n",
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("shared:diagram")
        .arg("--output-dir")
        .arg("reports/latest/diagrams")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["entries"][0]["source_path"],
        "docs/diagrams/a.txt"
    );
    assert_eq!(
        value["payload"]["entries"][0]["output_path"],
        "reports/latest/diagrams/docs/diagrams/a.html"
    );
    let rendered = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("diagrams")
            .join("docs")
            .join("diagrams")
            .join("a.html"),
    )
    .unwrap();
    assert!(rendered.contains("Diagram report family"));
}

#[test]
fn report_render_many_json_supports_template_family_overrides() {
    let root = temp_root("report-render-many-family-json");
    write_file(
        &root.join("docs").join("lint").join("a.txt"),
        "# title: Lint Alpha\n# copy_json:\n#   status: pass\nalpha body\n",
    );
    write_report_family_override(&root);
    write_report_catalog(
        &root,
        r#"[reporting.templates.lint]
path = "reports/templates/lint/report.html.j2"
"#,
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("lint")
        .arg("--glob")
        .arg("docs/lint/*.txt")
        .arg("--template-family")
        .arg("lint")
        .arg("--output-dir")
        .arg("reports/latest/lint")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["entries"][0]["source_path"],
        "docs/lint/a.txt"
    );
    assert_eq!(
        value["payload"]["entries"][0]["output_path"],
        "reports/latest/lint/docs/lint/a.html"
    );
    let rendered = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("lint")
            .join("docs")
            .join("lint")
            .join("a.html"),
    )
    .unwrap();
    assert!(rendered.contains("Lint override"));
}
