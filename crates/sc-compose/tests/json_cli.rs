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
    let root = std::env::temp_dir().join(format!("sc-compose-json-logs-{}", std::process::id()));
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
