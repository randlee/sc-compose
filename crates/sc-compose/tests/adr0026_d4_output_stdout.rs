//! ADR-0026 D4: file output is silent on text stdout.
#[path = "support/mod.rs"]
mod support;

use std::fs;

use serde_json::Value;
use support::{repo_root, sc_compose, temp_root, write_file};

fn copy_case_file(root: &std::path::Path, name: &str) {
    let fixture = repo_root()
        .join("tests/fixtures/render-regressions/adr0026-d4")
        .join(name);
    fs::copy(&fixture, root.join(name)).unwrap();
}

#[test]
fn output_file_suppresses_text_stdout_without_changing_file_bytes() {
    let root = temp_root("adr0026-d4-output");
    copy_case_file(&root, "output.md.j2");
    let output_path = root.join("out.txt");

    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args(["--file", "output.md.j2", "--var", "v=a", "--output"])
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {:?}",
        output.stdout
    );
    assert_eq!(fs::read(output_path).unwrap(), b"out a");
}

#[test]
fn append_file_suppresses_text_stdout_and_preserves_appended_bytes() {
    let root = temp_root("adr0026-d4-append");
    copy_case_file(&root, "append.json.j2");
    let append_path = root.join("records.jsonl");
    write_file(&append_path, "{\"old\":true}\n");

    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args(["--file", "append.json.j2", "--append"])
        .arg(&append_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {:?}",
        output.stdout
    );
    assert_eq!(
        fs::read(append_path).unwrap(),
        b"{\"old\":true}\n{\"new\":true}\n"
    );
}

#[test]
fn default_text_output_still_writes_stdout() {
    let root = temp_root("adr0026-d4-default");
    copy_case_file(&root, "output.md.j2");

    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args(["--file", "output.md.j2", "--var", "v=a"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"out a\n");
}

#[test]
fn json_output_keeps_its_stdout_envelope_when_writing_a_file() {
    let root = temp_root("adr0026-d4-json");
    copy_case_file(&root, "output.md.j2");
    let output_path = root.join("out.txt");

    let output = sc_compose()
        .args(["render", "--mode", "file", "--root"])
        .arg(&root)
        .args(["--file", "output.md.j2", "--var", "v=a", "--output"])
        .arg(&output_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected_output_path = output_path.to_string_lossy().replace('\\', "/");
    assert_eq!(envelope["payload"]["output_path"], expected_output_path);
    assert!(envelope["payload"].get("body").is_none());
    assert_eq!(fs::read(output_path).unwrap(), b"out a");
}
