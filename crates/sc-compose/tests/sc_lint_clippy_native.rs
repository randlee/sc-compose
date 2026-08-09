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
            "sc-compose-clippy-native-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = repo_root()
            .join("tests")
            .join("fixtures")
            .join("sc-lint")
            .join("clippy-native")
            .join(name);
        copy_directory(&source, &path);
        let target_dir = path.join(".sc").join("sc-lint").join("targets");
        fs::create_dir_all(&target_dir).expect("target registry");
        fs::copy(
            repo_root()
                .join(".sc")
                .join("sc-lint")
                .join("targets")
                .join("clippy-native.toml"),
            target_dir.join("clippy-native.toml"),
        )
        .expect("clippy-native target descriptor");
        Self { path }
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn repo_root() -> PathBuf {
    let canonical = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root");
    let Some(path) = canonical.to_str() else {
        return canonical;
    };
    PathBuf::from(path.strip_prefix(r"\\?\").unwrap_or(path))
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

fn run_clippy_native(fixture: &TempFixture) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "clippy-native",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose clippy native")
}

fn result_payload(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sc-compose JSON envelope")
}

#[test]
fn clippy_native_pass_preserves_workflow_envelope_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture("pass");
    let output = run_clippy_native(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "clippy native failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "clippy.native");
    assert_eq!(payload["target"], "clippy.native");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "clippy.native");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(payload["raw_payload"]["data"]["mode"], "native");
    assert_eq!(payload["raw_payload"]["data"]["step_count"], 1);
    assert_eq!(payload["raw_payload"]["data"]["tool"], "cargo");
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][0]["name"],
        "clippy.native"
    );
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][0]["command"],
        "cargo clippy --workspace --all-targets -- -D warnings"
    );
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][0]["kind"],
        "clippy"
    );
    assert_eq!(payload["raw_payload"]["data"]["steps"][0]["status"], "pass");
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports")
            .join("latest")
            .join("sc-lint")
            .join("raw")
            .join("clippy.native.json")
            .is_file()
    );
    let report = fixture
        .path
        .join("reports")
        .join("latest")
        .join("sc-lint")
        .join("index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("pass report");
    assert!(report_text.contains("clippy.native"));
    assert!(report_text.contains("pass"));
    assert!(report_text.contains("cargo clippy --workspace --all-targets -- -D warnings"));
}

#[test]
fn clippy_native_warning_remains_non_pass_with_structured_diagnostics() {
    let fixture = TempFixture::from_checked_in_fixture("warning");
    let output = run_clippy_native(&fixture);
    assert_eq!(
        output.status.code(),
        Some(5),
        "clippy native should retain the workflow failure exit status; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "clippy.native");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["command"], "clippy.native");
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["step"],
        "clippy.native"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["command"],
        "cargo clippy --workspace --all-targets -- -D warnings"
    );
    let stderr = payload["raw_payload"]["error"]["details"]["stderr"]
        .as_str()
        .expect("cargo clippy stderr");
    assert!(stderr.contains("clippy::len-zero"));
    assert!(stderr.contains("-D warnings"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(
        payload["diagnostics"][0]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );

    let report = fixture
        .path
        .join("reports")
        .join("latest")
        .join("sc-lint")
        .join("index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("failure report");
    assert!(report_text.contains("clippy.native"));
    assert!(report_text.contains("failed"));
    assert!(report_text.contains("CLI.BACKEND_EXEC_FAILURE"));
    assert!(report_text.contains("clippy::len-zero"));
}
