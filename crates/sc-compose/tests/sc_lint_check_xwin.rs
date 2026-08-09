//! End-to-end coverage for the check.xwin sc-lint target.
//!
//! The temporary cargo shim exercises sc-lint's real workflow contract without
//! requiring a Windows target toolchain during this host-side integration test.
//! The unavailable branch is asserted as a capability result, never as pass.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

mod support;
use support::{TempFixture, write_fake_cargo};

const TARGET: &str = "check-xwin";
const COMMAND_ID: &str = "check.xwin";
const WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";

#[test]
fn check_xwin_pass_preserves_workflow_envelope_and_report() {
    let fixture = check_xwin_fixture("pass", true);
    let output = run_check_xwin(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "unexpected check failure: {output:?}"
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["target"], COMMAND_ID);
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(payload["raw_payload"]["ok"], true);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(payload["raw_payload"]["data"]["mode"], "xwin");
    assert_eq!(payload["raw_payload"]["data"]["tool"], "cargo");
    assert_eq!(payload["raw_payload"]["data"]["xwin"]["available"], true);
    assert_eq!(
        payload["raw_payload"]["data"]["xwin"]["target"],
        WINDOWS_TARGET
    );
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][0]["name"],
        COMMAND_ID
    );
    assert_eq!(payload["raw_payload"]["data"]["steps"][0]["status"], "pass");
    assert!(
        payload["raw_payload"]["data"]["steps"][0]["command"]
            .as_str()
            .expect("step command")
            .contains(WINDOWS_TARGET)
    );

    assert_report_materialized(&fixture.path, &[COMMAND_ID, "pass"]);
}

#[test]
fn check_xwin_unavailable_remains_explicit_capability_failure() {
    let fixture = check_xwin_fixture("capability-negative", false);
    let output = run_check_xwin(&fixture);
    assert_eq!(
        output.status.code(),
        Some(3),
        "capability failure must use sc-compose's normalized exit code: {output:?}"
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["outcome"], "capability_error");
    assert_eq!(payload["exit_status"], 4);
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(payload["raw_payload"]["ok"], false);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.CAPABILITY_ERROR"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["command"],
        COMMAND_ID
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["tool"],
        "cargo xwin"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["target"],
        WINDOWS_TARGET
    );
    assert!(
        payload["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    assert_report_materialized(&fixture.path, &[COMMAND_ID, "capability", "cargo xwin"]);
}

fn run_check_xwin(fixture: &TempFixture) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            TARGET,
            "--json",
        ])
        .env("PATH", fixture.path_with_fake_tools())
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose lint check-xwin")
}

fn assert_report_materialized(root: &Path, expected_fragments: &[&str]) {
    let raw = root.join("reports/latest/sc-lint/raw/check.xwin.json");
    assert!(raw.is_file(), "missing raw report: {}", raw.display());
    let report = root.join("reports/latest/sc-lint/index.html");
    assert!(
        report.is_file(),
        "missing rendered report: {}",
        report.display()
    );
    let report_text = fs::read_to_string(report).expect("rendered report");
    for fragment in expected_fragments {
        assert!(
            report_text.contains(fragment),
            "report missing {fragment:?}: {report_text}"
        );
    }
}

fn result_payload(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "sc-compose did not emit JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn check_xwin_fixture(name: &str, xwin_available: bool) -> TempFixture {
    let fixture = TempFixture::from_checked_in_fixture("check-xwin", name, "check-xwin");
    write_fake_cargo(&fixture.path, xwin_available, false);
    fixture
}
