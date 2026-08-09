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
            "sc-compose-sc-boundary-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = repo_root()
            .join("tests/fixtures/sc-lint/sc-boundary")
            .join(name);
        copy_directory(&source, &path);
        let target_dir = path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).expect("target registry");
        fs::copy(
            repo_root().join(".sc/sc-lint/targets/sc-boundary.toml"),
            target_dir.join("sc-boundary.toml"),
        )
        .expect("sc-boundary target descriptor");
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

fn run_sc_boundary(fixture: &TempFixture) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "sc-boundary",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose lint sc-boundary")
}

fn result_payload(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sc-compose JSON envelope")
}

#[test]
fn boundary_pass_uses_shared_runner_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture("pass");
    let output = run_sc_boundary(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "lint failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(
        payload["exit_status"].as_i64(),
        output.status.code().map(i64::from)
    );
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-boundary");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/lint.sc-boundary.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("pass report");
    assert!(report_text.contains("lint.sc-boundary"));
    assert!(report_text.contains("pass"));
}

#[test]
fn boundary_dependency_violation_stays_non_pass_with_structured_finding() {
    let fixture = TempFixture::from_checked_in_fixture("dependency-violation");
    let output = run_sc_boundary(&fixture);
    let envelope = result_payload(&output);
    let payload = &envelope["payload"];

    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["outcome"], "findings");
    assert_eq!(
        payload["exit_status"].as_i64(),
        output.status.code().map(i64::from)
    );
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-boundary");
    assert_eq!(payload["raw_payload"]["data"]["status"], "fail");
    assert_eq!(payload["findings_count"], 1);
    assert_eq!(payload["findings"][0]["rule_id"], "SCB-DEPENDENCY-001");
    assert!(
        payload["findings"][0]["message"]
            .as_str()
            .expect("finding message")
            .contains("boundary-app")
    );
    assert!(
        payload["findings"][0]["message"]
            .as_str()
            .expect("finding message")
            .contains("boundary-api")
    );

    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("finding report");
    assert!(report_text.contains("findings"));
    assert!(report_text.contains("SCB-DEPENDENCY-001"));
    assert!(report_text.contains("boundary-api"));
}
