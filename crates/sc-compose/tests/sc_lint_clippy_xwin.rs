use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

static CLIPPY_XWIN_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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
            "sc-compose-clippy-xwin-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = repo_root()
            .join("tests/fixtures/sc-lint/clippy-xwin")
            .join(name);
        copy_directory(&source, &path);
        let target_dir = path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).expect("target registry");
        fs::copy(
            repo_root().join(".sc/sc-lint/targets/clippy-xwin.toml"),
            target_dir.join("clippy-xwin.toml"),
        )
        .expect("clippy-xwin target descriptor");
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

fn run_clippy_xwin(fixture: &TempFixture) -> std::process::Output {
    let _guard = CLIPPY_XWIN_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("clippy xwin test lock");
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "clippy-xwin",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose clippy xwin")
}

fn result_payload(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sc-compose JSON envelope")
}

fn cargo_xwin_available() -> bool {
    Command::new("cargo")
        .args(["xwin", "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn assert_report_materialized(fixture: &TempFixture, report_text: &str) {
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/clippy.xwin.json")
            .is_file()
    );
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/index.html")
            .is_file()
    );
    assert!(report_text.contains("clippy.xwin"));
    assert!(report_text.contains("reports/latest/sc-lint/raw/clippy.xwin.json"));
}

#[test]
fn clippy_xwin_pass_preserves_identity_and_materializes_report() {
    let fixture = TempFixture::from_checked_in_fixture("pass");
    let output = run_clippy_xwin(&fixture);
    let envelope = result_payload(&output);
    let payload = &envelope["payload"];

    assert_eq!(envelope["schema_version"], "1");
    assert_eq!(payload["command_id"], "clippy.xwin");
    assert_eq!(payload["target"], "clippy.xwin");
    assert_eq!(payload["raw_payload"]["command"], "clippy.xwin");
    assert!(payload["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);

    let report_text = fs::read_to_string(fixture.path.join("reports/latest/sc-lint/index.html"))
        .expect("clippy xwin pass report");
    assert_report_materialized(&fixture, &report_text);

    if cargo_xwin_available() {
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(payload["outcome"], "pass");
        assert_eq!(payload["exit_status"], 0);
        assert_eq!(payload["raw_payload"]["ok"], true);
        assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
        assert_eq!(payload["raw_payload"]["data"]["mode"], "xwin");
        assert_eq!(
            payload["raw_payload"]["data"]["xwin"]["target"],
            "x86_64-pc-windows-msvc"
        );
        assert!(report_text.contains("pass"));
    } else {
        assert_eq!(output.status.code(), Some(5));
        assert_eq!(payload["outcome"], "capability_error");
        assert_eq!(payload["exit_status"], 5);
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.CAPABILITY_ERROR"
        );
        assert!(report_text.contains("capability_error"));
    }
}

#[test]
fn clippy_xwin_failure_stays_non_pass_with_structured_diagnostics() {
    let fixture = TempFixture::from_checked_in_fixture("failing-analysis");
    let output = run_clippy_xwin(&fixture);
    let envelope = result_payload(&output);
    let payload = &envelope["payload"];

    assert_eq!(envelope["schema_version"], "1");
    assert_eq!(payload["command_id"], "clippy.xwin");
    assert_eq!(payload["target"], "clippy.xwin");
    assert_eq!(payload["raw_payload"]["command"], "clippy.xwin");
    assert!(payload["diagnostics"].is_array());
    assert!(
        !payload["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    assert_eq!(payload["findings_count"], 0);

    let report_text = fs::read_to_string(fixture.path.join("reports/latest/sc-lint/index.html"))
        .expect("clippy xwin failure report");
    assert_report_materialized(&fixture, &report_text);

    if cargo_xwin_available() {
        assert_eq!(output.status.code(), Some(5));
        assert_eq!(payload["outcome"], "failed");
        assert_eq!(payload["exit_status"], 5);
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.BACKEND_EXEC_FAILURE"
        );
        assert_eq!(
            payload["raw_payload"]["error"]["details"]["step"],
            "clippy.xwin"
        );
        assert!(
            payload["raw_payload"]["error"]["cause"]
                .as_str()
                .expect("clippy failure cause")
                .contains("unused-mut")
        );
        assert!(report_text.contains("failed"));
        assert!(report_text.contains("clippy-xwin-failing"));
    } else {
        assert_eq!(output.status.code(), Some(5));
        assert_eq!(payload["outcome"], "capability_error");
        assert_eq!(payload["exit_status"], 5);
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.CAPABILITY_ERROR"
        );
        assert!(report_text.contains("capability_error"));
    }
}
