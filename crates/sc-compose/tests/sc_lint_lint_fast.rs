use std::fs;
use std::process::Command;

use serde_json::Value;

mod support;

use support::{TempFixture, materialize_sc_lint_runtime};

const RUNTIME_FILES: &[&str] = &[
    "check_version_sync.py",
    "lint_manifests.py",
    "lint_codespell.py",
    "run_pytests.py",
    "lint_common.py",
];

fn lint_fast_fixture(name: &str) -> TempFixture {
    let fixture = support::TempFixture::from_checked_in_fixture("lint-fast", name, "lint-fast");
    materialize_sc_lint_runtime(&fixture.path, RUNTIME_FILES);
    fixture
}

fn run_lint_fast(fixture: &TempFixture) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "lint-fast",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose lint fast")
}

fn result_payload(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("sc-compose JSON envelope")
}

#[test]
fn lint_fast_pass_preserves_composite_profile_and_materializes_evidence() {
    let fixture = lint_fast_fixture("pass");
    let output = run_lint_fast(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "lint fast failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.fast");
    assert_eq!(payload["target"], "lint.fast");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["raw_payload"]["command"], "lint.fast");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(payload["raw_payload"]["data"]["profile"], "fast");
    assert_eq!(payload["raw_payload"]["data"]["step_count"], 5);
    assert_eq!(payload["raw_payload"]["data"]["xwin"]["included"], false);
    let steps = payload["raw_payload"]["data"]["steps"]
        .as_array()
        .expect("fast profile steps");
    let step_names: Vec<_> = steps
        .iter()
        .map(|step| step["name"].as_str().expect("step name"))
        .collect();
    assert_eq!(
        step_names,
        ["fmt", "version", "manifests", "spell", "pytests"]
    );
    assert_eq!(steps[0]["command"], "cargo fmt --all --check");
    assert!(
        steps[1]["command"]
            .as_str()
            .is_some_and(|command| { command.contains("check_version_sync.py") })
    );
    assert!(
        steps[2]["command"]
            .as_str()
            .is_some_and(|command| { command.contains("lint_manifests.py") })
    );
    assert!(steps.iter().all(|step| step["status"] == "pass"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports")
            .join("latest")
            .join("sc-lint")
            .join("raw")
            .join("lint.fast.json")
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
    assert!(report_text.contains("lint.fast"));
    assert!(report_text.contains("pass"));
    assert!(report_text.contains("pytests"));
}

#[test]
fn lint_fast_manifest_failure_remains_non_pass_with_structured_diagnostics() {
    let fixture = lint_fast_fixture("failing-manifest");
    let output = run_lint_fast(&fixture);
    assert_eq!(
        output.status.code(),
        Some(2),
        "lint fast should return the CLI validation-failure status; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let envelope = result_payload(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.fast");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["command"], "lint.fast");
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["step"],
        "manifests"
    );
    assert!(
        payload["raw_payload"]["error"]["details"]["command"]
            .as_str()
            .is_some_and(|command| command.contains("lint_manifests.py"))
    );
    let stdout = payload["raw_payload"]["error"]["details"]["stdout"]
        .as_str()
        .expect("manifest lint stdout");
    assert!(stdout.contains("manifests failed"));
    assert!(stdout.contains("homepage"));
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
    assert!(report_text.contains("lint.fast"));
    assert!(report_text.contains("failed"));
    assert!(report_text.contains("CLI.BACKEND_EXEC_FAILURE"));
    assert!(report_text.contains("homepage"));
}
