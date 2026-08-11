mod support;

use std::fs;
use std::process::Output;

use support::{
    CheckedInFixture, TempFixture, materialize_sc_lint_runtime_with_config, parse_stdout,
    sc_compose,
};

#[test]
fn lint_ci_preserves_known_sc_lint_boundary_packaging_defect() {
    let (root, output) = run_target("pass");
    assert_eq!(
        output.status.code(),
        Some(2),
        "lint ci should preserve the known backend failure; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.ci");
    assert_eq!(payload["target"], "lint.ci");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["command"], "lint.ci");
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(
        payload["raw_payload"]["error"]["details"]["step"],
        "sc-boundary"
    );
    assert!(
        payload["raw_payload"]["error"]["details"]["command"]
            .as_str()
            .is_some_and(|command| command.contains("lint_sc_boundary.py"))
    );
    let stdout = payload["raw_payload"]["error"]["details"]["stdout"]
        .as_str()
        .expect("boundary utility stdout");
    let stderr = payload["raw_payload"]["error"]["details"]["stderr"]
        .as_str()
        .expect("boundary utility stderr");
    let combined = format!("{stdout}\n{stderr}");
    assert!(combined.contains("sc-lint-boundary"));
    assert!(combined.contains("not found") || combined.contains("could not find"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(
        payload["diagnostics"][0]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );

    let report = root.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("known defect report");
    assert!(report_text.contains("lint.ci"));
    assert!(report_text.contains("failed"));
    assert!(report_text.contains("sc-boundary"));
    assert!(report_text.contains("sc-lint-boundary"));
    assert!(
        root.path
            .join("reports/latest/sc-lint/raw/lint.ci.json")
            .is_file()
    );
}

#[test]
fn lint_ci_manifest_failure_remains_non_pass_with_structured_diagnostics() {
    let (root, output) = run_target("failing-manifest");
    assert_eq!(
        output.status.code(),
        Some(2),
        "lint ci should retain profile failure status; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.ci");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["command"], "lint.ci");
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
        .expect("manifest utility stdout");
    assert!(stdout.contains("manifests failed"));
    assert!(stdout.contains("homepage"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(
        payload["diagnostics"][0]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );

    let report = root.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("manifest failure report");
    assert!(report_text.contains("lint.ci"));
    assert!(report_text.contains("failed"));
    assert!(report_text.contains("homepage"));
    assert!(
        root.path
            .join("reports/latest/sc-lint/raw/lint.ci.json")
            .is_file()
    );
}

fn run_target(fixture: &str) -> (TempFixture, Output) {
    let root = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "lint-ci",
        name: fixture,
        target: "lint-ci",
    });
    materialize_sc_lint_runtime_with_config(
        &root.path,
        &[
            "lint_cargo_deny.py",
            "lint_cargo_shear.py",
            "check_version_sync.py",
            "lint_manifests.py",
            "lint_codespell.py",
            "run_pytests.py",
            "lint_sc_boundary.py",
            "lint_sc_portability.py",
            "lint_common.py",
        ],
    );

    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "lint-ci",
            "--json",
        ])
        .env("SC_LOG_ROOT", root.path.join("logs"))
        .output()
        .expect("run sc-compose lint ci");
    (root, output)
}
