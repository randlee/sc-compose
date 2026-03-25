use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sc-compose-cli-{label}-{}-{nanos}",
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

#[test]
fn render_dry_run_does_not_create_output_file() {
    let root = temp_root("dry-run");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    let output = root.join("out.md");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&output)
        .arg("--dry-run")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(!output.exists());
}

#[test]
fn exit_code_zero_for_valid_render() {
    let root = temp_root("exit-ok");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(0));
}

#[test]
fn exit_code_two_for_validation_failure() {
    let root = temp_root("exit-validation");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let status = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));
}

#[test]
fn exit_code_three_for_resolve_failure() {
    let root = temp_root("exit-resolve");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("missing.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(3));
}

#[test]
fn render_uses_json_var_file_inputs() {
    let root = temp_root("var-file-json");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.json");
    write_file(&vars_file, "{ \"name\": \"json-world\" }\n");

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
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello json-world"
    );
}

#[test]
fn render_uses_yaml_var_file_inputs() {
    let root = temp_root("var-file-yaml");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(&vars_file, "name: yaml-world\n");

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
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello yaml-world"
    );
}

#[test]
fn render_uses_env_prefix_inputs() {
    let root = temp_root("env-prefix");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--env-prefix")
        .arg("SC_")
        .env("SC_NAME", "env-world")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello env-world"
    );
}
