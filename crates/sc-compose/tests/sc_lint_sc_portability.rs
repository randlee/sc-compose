use std::fs;
mod support;
use support::{CheckedInFixture, TempFixture, parse_stdout, sc_compose};

fn run_sc_portability(fixture: &TempFixture) -> std::process::Output {
    sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "sc-portability",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose lint sc-portability")
}

#[test]
fn portability_pass_preserves_envelope_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-portability",
        name: "pass",
        target: "sc-portability",
    });
    let output = run_sc_portability(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "lint failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-portability");
    assert_eq!(payload["target"], "lint.sc-portability");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-portability");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/lint.sc-portability.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("pass report");
    assert!(report_text.contains("lint.sc-portability"));
    assert!(report_text.contains("pass"));
}

#[test]
fn portability_path_violation_stays_non_pass_with_structured_finding() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-portability",
        name: "failing-path",
        target: "sc-portability",
    });
    let output = run_sc_portability(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "sc-lint findings should remain a successful subprocess with a fail payload; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-portability");
    assert_eq!(payload["outcome"], "findings");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-portability");
    assert_eq!(payload["raw_payload"]["data"]["status"], "fail");
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 1);
    assert_eq!(payload["findings"][0]["rule_id"], "PORT-001");
    let finding_message = payload["findings"][0]["message"]
        .as_str()
        .expect("finding message");
    assert!(finding_message.contains("/tmp/sc-compose-portability"));
    assert!(finding_message.contains("portability-failing"));

    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("finding report");
    assert!(report_text.contains("findings"));
    assert!(report_text.contains("PORT-001"));
    assert!(report_text.contains("/tmp/sc-compose-portability"));
}
