mod support;

use std::fs;
use std::process::Output;

use support::{
    CheckedInFixture, TempFixture, installed_sc_lint_path, parse_stdout, sc_compose,
    write_fake_cargo_deny,
};

#[test]
fn lint_ci_pass_executes_real_boundary_and_materializes_report() {
    let (root, output) = run_target("pass");
    assert_eq!(output.status.code(), Some(0), "lint ci failed: {output:?}");
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.ci");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    let steps = payload["raw_payload"]["data"]["steps"]
        .as_array()
        .expect("steps");
    assert!(
        steps
            .iter()
            .any(|step| step["name"] == "sc-boundary" && step["status"] == "pass")
    );
    assert!(steps.iter().all(|step| step["status"] == "pass"));
    assert!(
        root.path
            .join("reports/latest/sc-lint/raw/lint.ci.json")
            .is_file()
    );
    let report =
        fs::read_to_string(root.path.join("reports/latest/sc-lint/index.html")).expect("report");
    assert!(report.contains("lint.ci"));
    assert!(report.contains("pass"));
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
        "profile.steps_failed"
    );
    let failures = payload["raw_payload"]["error"]["details"]["failed_steps"]
        .as_array()
        .expect("profile failures");
    let manifest = failures
        .iter()
        .find(|step| step["name"] == "manifests")
        .expect("manifest step must fail");
    let message = manifest["message"].as_str().expect("manifest finding");
    assert!(message.contains("manifests failed"));
    assert!(message.contains("homepage"));
    assert!(payload["raw_payload"]["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(payload["diagnostics"][0]["code"], "profile.steps_failed");

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
    write_fake_cargo_deny(&root.path);
    let path = installed_sc_lint_path(Some(&root.path.join("fake-bin")));

    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "lint-ci",
            "--json",
        ])
        .env("PATH", path)
        .env_remove("PYTHONPATH")
        .env("SC_LOG_ROOT", root.path.join("logs"))
        .output()
        .expect("run sc-compose lint ci");
    (root, output)
}
