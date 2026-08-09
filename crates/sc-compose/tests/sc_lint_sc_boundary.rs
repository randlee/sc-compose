use std::fs;
use std::process::Command;

use serde_json::Value;

mod support;
use support::TempFixture;

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
    let fixture = TempFixture::from_checked_in_fixture("sc-boundary", "pass", "sc-boundary");
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
    let fixture =
        TempFixture::from_checked_in_fixture("sc-boundary", "dependency-violation", "sc-boundary");
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
