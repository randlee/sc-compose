use std::fs;
mod support;
use support::{CheckedInFixture, TempFixture, normalize_path_str, parse_stdout, sc_compose};

fn run_sc_runtime(fixture: &TempFixture) -> std::process::Output {
    sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "sc-runtime",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose lint sc-runtime")
}

#[test]
fn runtime_pass_preserves_envelope_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-runtime",
        name: "pass",
        target: "sc-runtime",
    });
    let output = run_sc_runtime(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "lint failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = parse_stdout(&output);
    assert_eq!(envelope["schema_version"], "1");
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-runtime");
    assert_eq!(payload["target"], "lint.sc-runtime");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-runtime");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/lint.sc-runtime.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("pass report");
    assert!(report_text.contains("lint.sc-runtime"));
    assert!(report_text.contains("pass"));
}

#[test]
fn runtime_unsafe_wait_stays_non_pass_with_structured_finding() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-runtime",
        name: "unsafe-wait",
        target: "sc-runtime",
    });
    let output = run_sc_runtime(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "sc-lint findings should remain a successful subprocess with a fail payload; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let envelope = parse_stdout(&output);
    assert_eq!(envelope["schema_version"], "1");
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-runtime");
    assert_eq!(payload["target"], "lint.sc-runtime");
    assert_eq!(payload["outcome"], "findings");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-runtime");
    assert_eq!(payload["raw_payload"]["data"]["status"], "fail");
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 1);

    let finding = &payload["findings"][0];
    assert_eq!(finding["rule_id"], "SCB-RUNTIME-001");
    assert_eq!(finding["kind"], "condvar_wait_without_timeout");
    assert_eq!(
        finding["owner_ids"][0],
        "crate::runtime-unsafe::runtime_unsafe"
    );
    assert_eq!(
        finding["node_ids"][0],
        "crate::runtime-unsafe::runtime_unsafe::block_until_ready"
    );
    let finding_message = finding["message"].as_str().expect("finding message");
    let normalized_finding_message = normalize_path_str(finding_message);
    assert!(normalized_finding_message.contains("crates/runtime-unsafe/src/lib.rs:7:"));
    assert!(finding_message.contains("SCB-RUNTIME-001"));

    let raw_finding = &payload["raw_payload"]["data"]["findings"][0];
    assert_eq!(raw_finding["rule_id"], "SCB-RUNTIME-001");
    assert_eq!(
        raw_finding["owner_ids"][0],
        "crate::runtime-unsafe::runtime_unsafe"
    );

    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/lint.sc-runtime.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("finding report");
    assert!(report_text.contains("lint.sc-runtime"));
    assert!(report_text.contains("findings"));
    assert!(report_text.contains("SCB-RUNTIME-001"));
    assert!(report_text.contains("runtime-unsafe"));
}
