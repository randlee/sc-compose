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
            "sc-compose-line-counts-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = repo_root()
            .join("tests/fixtures/sc-lint/line-counts")
            .join(name);
        copy_directory(&source, &path);

        // CI materializes the pinned sc-lint Python utilities in the consumer
        // checkout. Copying them into the ephemeral fixture exercises that
        // supported adapter contract without vendoring scripts in sc-compose.
        let utilities = sc_lint_utilities();
        assert!(
            utilities.is_some(),
            "sc-lint Python utilities are unavailable; run the Phase L setup action first"
        );
        copy_directory(
            &utilities.expect("checked sc-lint utility directory"),
            &path.join(".just"),
        );

        let target_dir = path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).expect("target registry");
        fs::copy(
            repo_root().join(".sc/sc-lint/targets/line-counts.toml"),
            target_dir.join("line-counts.toml"),
        )
        .expect("line-counts target descriptor");
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

fn sc_lint_utilities() -> Option<PathBuf> {
    let root = repo_root();
    let adjacent_sibling = root.parent().map(|parent| parent.join("sc-lint/.just"));
    let worktree_sibling = root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(|parent| parent.join("sc-lint/.just"));
    [Some(root.join(".just")), adjacent_sibling, worktree_sibling]
        .into_iter()
        .flatten()
        .find(|path| path.join("lint_line_counts.py").is_file())
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

fn run_line_counts(fixture: &TempFixture) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "line-counts",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose lint line-counts")
}

fn result_payload(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sc-compose JSON envelope")
}

#[test]
fn line_counts_pass_preserves_adapter_envelope_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture("pass");
    let output = run_line_counts(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "lint failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.line-counts");
    assert_eq!(payload["target"], "lint.line-counts");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "lint.line-counts");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(
        payload["raw_payload"]["data"]["adapter"],
        "sc-lint-python-v1"
    );
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/lint.line-counts.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("pass report");
    assert!(report_text.contains("lint.line-counts"));
    assert!(report_text.contains("pass"));
    assert!(report_text.contains("source file size limits satisfied"));
}

#[test]
fn line_counts_over_limit_remains_failed_with_structured_finding() {
    let fixture = TempFixture::from_checked_in_fixture("over-limit");
    let output = run_line_counts(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "sc-lint should preserve adapter findings in its successful JSON envelope; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.line-counts");
    assert_eq!(payload["outcome"], "findings");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "lint.line-counts");
    assert_eq!(payload["raw_payload"]["data"]["status"], "fail");
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 1);
    let finding = payload["findings"][0].as_str().expect("line-count finding");
    assert!(finding.contains("line-counts-over-limit/src/lib.rs"));
    assert!(finding.contains("prod="));
    assert!(finding.contains("exceeds limit 5"));

    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("finding report");
    assert!(report_text.contains("findings"));
    assert!(report_text.contains("exceeds limit 5"));
}
