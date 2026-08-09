use std::fs;
use std::process::Command;

use serde_json::Value;

mod support;
use support::{TempFixture, materialize_sc_lint_runtime, normalize_path_str};

fn line_counts_fixture(name: &str) -> TempFixture {
    let fixture = TempFixture::from_checked_in_fixture("line-counts", name, "line-counts");

    // CI materializes the pinned sc-lint Python utilities in the consumer
    // checkout. Copying them into the ephemeral fixture exercises that
    // supported adapter contract without vendoring scripts in sc-compose.
    materialize_sc_lint_runtime(
        &fixture.path,
        &[
            "lint_line_counts.py",
            "python_adapter.py",
            "lint_common.py",
            "view_common.py",
        ],
    );
    fixture
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
    let fixture = line_counts_fixture("pass");
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
    let fixture = line_counts_fixture("over-limit");
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
    let normalized_finding = normalize_path_str(finding);
    assert!(normalized_finding.contains("line-counts-over-limit/src/lib.rs"));
    assert!(finding.contains("prod="));
    assert!(finding.contains("exceeds limit 5"));

    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("finding report");
    assert!(report_text.contains("findings"));
    assert!(report_text.contains("exceeds limit 5"));
}
