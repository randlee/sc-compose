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
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
}

fn parse_stdout(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn assert_envelope(value: &Value) {
    assert_eq!(value["schema_version"], "1");
    assert!(value.get("payload").is_some());
    assert!(value["diagnostics"].is_array());
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
}

#[test]
fn resolve_json_uses_diagnostic_envelope() {
    let root = temp_root("resolve-json");
    write_file(&root.join(".claude/agents/example.md"), "agent");

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
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["valid"], false);
    assert_eq!(value["diagnostics"].as_array().map(Vec::len), Some(1));
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
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["workspace_root"],
        fs::canonicalize(&root).unwrap().display().to_string()
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

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_RENDER_STDIN_DOUBLE_READ"));
}
