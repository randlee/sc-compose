use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn runner_preserves_sc_lint_envelope_and_writes_both_artifacts() {
    let root = workspace_root();
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            root.to_str().expect("UTF-8 root"),
            "--target",
            "sc-boundary",
            "--json",
        ])
        .output()
        .expect("run sc-compose lint");
    assert_eq!(output.status.code(), Some(0));

    let envelope: Value = serde_json::from_slice(&output.stdout).expect("JSON envelope");
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-boundary");
    assert_eq!(payload["exit_status"], 0);
    assert!(
        root.join("reports/latest/sc-lint/raw/lint.sc-boundary.json")
            .is_file()
    );
    assert!(root.join("reports/latest/sc-lint/index.html").is_file());
}

#[test]
fn runner_rejects_commands_outside_the_allowlist() {
    let root = workspace_root();
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            root.to_str().expect("UTF-8 root"),
            "--target",
            "sh -c touch /tmp/not-allowed",
            "--json",
        ])
        .output()
        .expect("run sc-compose lint");
    assert_eq!(output.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("JSON error envelope");
    assert!(envelope["diagnostics"][0]["message"].as_str().is_some());
}
