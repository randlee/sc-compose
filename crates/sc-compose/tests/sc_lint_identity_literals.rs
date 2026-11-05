//! End-to-end coverage for the identity-literals sc-lint target.
//!
//! Exercises the installed, pinned sc-lint wheel against isolated fixtures.

mod support;

use std::fs;
use std::process::Output;

use support::{CheckedInFixture, TempFixture, installed_sc_lint_path, parse_stdout, sc_compose};

const TARGET: &str = "identity-literals";
const IDENTITY: &str = include_str!("fixtures/identity-literal.txt");

#[test]
fn identity_literals_pass_preserves_adapter_envelope_and_report() {
    let (root, output) = run_target("pass");
    assert_success(&output);

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.identity-literals");
    assert_eq!(payload["target"], "lint.identity-literals");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(payload["raw_payload"]["ok"], true);
    assert_eq!(payload["raw_payload"]["command"], "lint.identity-literals");
    assert_eq!(
        payload["raw_payload"]["data"]["adapter"],
        "sc-lint-python-v1"
    );
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(
        payload["raw_payload"]["data"]["findings"],
        serde_json::json!([])
    );
    assert_eq!(
        payload["raw_payload"]["data"]["forbidden_literals"][0],
        IDENTITY
    );
    assert_eq!(
        payload["raw_payload"]["data"]["production_canonical_literals"][IDENTITY][0],
        "crates/demo/src/constants.rs"
    );
    assert_eq!(
        payload["raw_artifact"],
        "reports/latest/sc-lint/raw/lint.identity-literals.json"
    );

    let report = fs::read_to_string(root.path.join("reports/latest/sc-lint/index.html"))
        .expect("identity-literals report");
    assert!(report.contains("lint.identity-literals"));
    assert!(report.contains("pass"));
    assert!(
        root.path
            .join("reports/latest/sc-lint/raw/lint.identity-literals.json")
            .is_file()
    );
}

#[test]
fn identity_literals_findings_remain_non_pass_with_structured_evidence() {
    let (root, output) = run_target("fail");
    assert_success(&output);

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.identity-literals");
    assert_eq!(payload["outcome"], "findings");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["findings_count"], 3);
    assert_ne!(payload["outcome"], "pass");
    assert_eq!(payload["raw_payload"]["ok"], false);
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(payload["raw_payload"]["error"]["details"]["status"], "fail");
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["findings"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let findings = payload["findings"].as_array().expect("finding array");
    assert!(
        findings
            .iter()
            .any(|finding| finding.as_str().unwrap().contains("crates/demo/src/lib.rs"))
    );
    assert!(findings.iter().any(|finding| {
        finding
            .as_str()
            .unwrap()
            .contains("crates/demo/tests/identity_literals.rs")
    }));
    assert!(
        findings
            .iter()
            .all(|finding| finding.as_str().unwrap().contains(IDENTITY))
    );

    let report = fs::read_to_string(root.path.join("reports/latest/sc-lint/index.html"))
        .expect("identity-literals report");
    assert!(report.contains("findings"));
    assert!(report.contains("crates/demo/src/lib.rs"));
    assert!(report.contains(IDENTITY));
    assert!(
        root.path
            .join("reports/latest/sc-lint/raw/lint.identity-literals.json")
            .is_file()
    );
}

fn run_target(fixture: &str) -> (TempFixture, Output) {
    let root = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "identity-literals",
        name: fixture,
        target: "identity-literals",
    });
    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            TARGET,
            "--json",
        ])
        .env("PATH", installed_sc_lint_path(None))
        .env_remove("PYTHONPATH")
        .output()
        .expect("run sc-compose identity-literals");
    (root, output)
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "sc-compose lint failed: status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
