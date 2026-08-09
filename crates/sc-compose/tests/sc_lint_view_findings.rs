use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

mod support;
use support::{TempFixture, normalize_path_str, repo_root};

const VIEW_UTILITY_FILES: &[&str] = &[
    "view_findings.py",
    "view_common.py",
    "python_adapter.py",
    "lint_common.py",
];

fn find_pinned_utility_directory() -> Option<PathBuf> {
    let root = repo_root();
    let mut candidates = vec![root.join(".just")];
    let mut ancestor = root.as_path();
    while let Some(parent) = ancestor.parent() {
        candidates.push(parent.join("sc-lint/.just"));
        ancestor = parent;
    }
    candidates.into_iter().find(|directory| {
        VIEW_UTILITY_FILES
            .iter()
            .all(|file| directory.join(file).is_file())
    })
}

fn install_pinned_view_utilities(fixture: &TempFixture) -> bool {
    let Some(source_dir) = find_pinned_utility_directory() else {
        return false;
    };
    let destination_dir = fixture.path.join(".just");
    fs::create_dir_all(&destination_dir).expect("utility destination");
    for file_name in VIEW_UTILITY_FILES {
        fs::copy(source_dir.join(file_name), destination_dir.join(file_name))
            .expect("pinned view utility");
    }
    true
}

fn run_view_findings(fixture: &TempFixture) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "view-findings",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose view findings")
}

fn result_envelope(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sc-compose JSON envelope")
}

#[test]
fn view_findings_pass_preserves_identity_and_materializes_report() {
    let fixture = TempFixture::from_checked_in_fixture("view-findings", "pass", "view-findings");
    let utilities_available = install_pinned_view_utilities(&fixture);
    let output = run_view_findings(&fixture);
    let envelope = result_envelope(&output);
    let payload = &envelope["payload"];

    assert_eq!(envelope["schema_version"], "1");
    assert_eq!(payload["command_id"], "view.findings");
    assert_eq!(payload["target"], "view.findings");
    assert_eq!(payload["raw_payload"]["command"], "view.findings");
    assert!(payload["raw_payload"]["diagnostics"].is_array());

    if utilities_available {
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(payload["outcome"], "pass");
        assert_eq!(payload["exit_status"], 0);
        assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
        assert_eq!(
            payload["raw_payload"]["data"]["summary"],
            "collated 2 findings artifact set(s)"
        );
        assert_eq!(
            payload["raw_payload"]["data"]["views"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            payload["raw_payload"]["data"]["views"][1]["tool"],
            "sc-lint-runtime"
        );
        assert_eq!(
            payload["raw_payload"]["data"]["views"][1]["finding_count"],
            1
        );
    } else {
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(payload["outcome"], "config_error");
        assert_eq!(payload["diagnostics"][0]["code"], "CLI.CONFIG_ERROR");
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.BACKEND_PROTOCOL_ERROR"
        );
    }

    let source_summary = fs::read_to_string(
        fixture
            .path
            .join("artifacts/findings/sc-runtime/summary.json"),
    )
    .expect("stored findings payload");
    assert!(source_summary.contains("SCB-RUNTIME-001"));
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/view.findings.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("view findings report");
    assert!(report_text.contains("view.findings"));
    assert!(report_text.contains("sc-lint-runtime"));
    assert!(normalize_path_str(report_text).contains("artifacts/findings/sc-runtime/summary.json"));
}

#[test]
fn view_findings_malformed_payload_stays_non_pass_with_diagnostics() {
    let fixture =
        TempFixture::from_checked_in_fixture("view-findings", "malformed-summary", "view-findings");
    let utilities_available = install_pinned_view_utilities(&fixture);
    let output = run_view_findings(&fixture);
    let envelope = result_envelope(&output);
    let payload = &envelope["payload"];

    assert_eq!(envelope["schema_version"], "1");
    assert_eq!(payload["command_id"], "view.findings");
    assert_eq!(payload["target"], "view.findings");
    assert_eq!(payload["raw_payload"]["command"], "view.findings");
    assert!(
        payload["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert_eq!(payload["findings_count"], 0);
    if utilities_available {
        assert_eq!(payload["outcome"], "failed");
        assert_ne!(payload["outcome"], "pass");
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.BACKEND_PROTOCOL_ERROR"
        );
        assert!(
            payload["raw_payload"]["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("failed to build findings view"))
        );
    } else {
        assert_eq!(payload["outcome"], "config_error");
        assert_ne!(payload["outcome"], "pass");
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(payload["diagnostics"][0]["code"], "CLI.CONFIG_ERROR");
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.BACKEND_PROTOCOL_ERROR"
        );
    }

    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/view.findings.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("failed view findings report");
    assert!(report_text.contains("view.findings"));
    assert!(
        report_text.contains("CLI.CONFIG_ERROR")
            || report_text.contains("CLI.BACKEND_PROTOCOL_ERROR")
    );
    assert!(report_text.contains("failed"));
}
