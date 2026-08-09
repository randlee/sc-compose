use std::fs;

mod support;
use support::{CheckedInFixture, TempFixture, parse_stdout, sc_compose};

fn run_clippy_native(fixture: &TempFixture) -> std::process::Output {
    sc_compose()
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

#[test]
fn clippy_native_pass_preserves_workflow_envelope_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "clippy-native",
        name: "pass",
        target: "clippy-native",
    });
    let output = run_clippy_native(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "clippy native failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = parse_stdout(&output);
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
    assert_eq!(payload["raw_payload"]["data"]["steps"][0]["kind"], "clippy");
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
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "clippy-native",
        name: "warning",
        target: "clippy-native",
    });
    let output = run_clippy_native(&fixture);
    assert_eq!(
        output.status.code(),
        Some(2),
        "clippy native should retain the workflow failure exit status; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let envelope = parse_stdout(&output);
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
