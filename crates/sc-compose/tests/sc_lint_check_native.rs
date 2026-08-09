use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

struct TempFixture {
    path: PathBuf,
}

impl TempFixture {
    fn from_checked_in_fixture(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sc-compose-check-native-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = repo_root()
            .join("tests/fixtures/sc-lint/check-native")
            .join(name);
        copy_directory(&source, &path);
        let target_dir = path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).expect("target registry");
        fs::copy(
            repo_root().join(".sc/sc-lint/targets/check-native.toml"),
            target_dir.join("check-native.toml"),
        )
        .expect("check-native target descriptor");
        Self { path }
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("fixture destination");
    for entry in fs::read_dir(source).expect("fixture source") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("fixture file");
        }
    }
}

fn run_check_native(fixture: &TempFixture) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "check-native",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose check native")
}

fn result_payload(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sc-compose JSON envelope")
}

#[test]
fn check_native_pass_preserves_workflow_envelope_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture("pass");
    let output = run_check_native(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "check native failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "check.native");
    assert_eq!(payload["target"], "check.native");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "check.native");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(payload["raw_payload"]["data"]["mode"], "native");
    assert_eq!(payload["raw_payload"]["data"]["step_count"], 1);
    assert_eq!(payload["raw_payload"]["data"]["tool"], "cargo");
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][0]["name"],
        "check.native"
    );
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][0]["command"],
        "cargo check --workspace"
    );
    assert_eq!(payload["raw_payload"]["data"]["steps"][0]["status"], "pass");
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/check.native.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("pass report");
    assert!(report_text.contains("check.native"));
    assert!(report_text.contains("pass"));
    assert!(report_text.contains("cargo check --workspace"));
}

#[test]
fn check_native_compile_failure_remains_non_pass_with_structured_diagnostics() {
    let fixture = TempFixture::from_checked_in_fixture("compile-error");
    let output = run_check_native(&fixture);
    assert_eq!(
        output.status.code(),
        Some(5),
        "check native should retain the workflow failure exit status; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "check.native");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["command"], "check.native");
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["step"],
        "check.native"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["command"],
        "cargo check --workspace"
    );
    let stderr = payload["raw_payload"]["error"]["details"]["stderr"]
        .as_str()
        .expect("cargo check stderr");
    assert!(stderr.contains("error[E0425]"));
    assert!(stderr.contains("MissingType"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(
        payload["diagnostics"][0]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );

    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("failure report");
    assert!(report_text.contains("check.native"));
    assert!(report_text.contains("failed"));
    assert!(report_text.contains("CLI.BACKEND_EXEC_FAILURE"));
    assert!(report_text.contains("MissingType"));
}
