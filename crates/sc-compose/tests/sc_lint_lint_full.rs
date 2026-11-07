//! End-to-end coverage for the lint.full sc-lint profile.
//!
//! The tests invoke the installed sc-lint binary through sc-compose's real
//! runner. They shim only external cargo/Python tools so the profile contract
//! is deterministic on a developer host and on Windows CI. No Python utility
//! or report template is copied into this repository.

use std::fs;
use std::path::Path;
use std::process::Output;

mod support;

use support::{
    CheckedInFixture, FakeCargoOptions, SC_LINT_PYTHON_TOOLS, TempFixture, parse_stdout,
    sc_compose, write_fake_cargo,
};

const TARGET: &str = "lint-full";
const COMMAND_ID: &str = "lint.full";
const PYTHON_PASS_OUTPUT: &str = r#"import json
print(json.dumps({"adapter_schema": "sc-lint-python-v1", "ok": True, "summary": "fixture utility passed", "data": {"findings": []}, "diagnostics": []}))
"#;
const PYTHON_FINDING_OUTPUT: &str = r#"import json
print(json.dumps({"adapter_schema": "sc-lint-python-v1", "ok": False, "summary": "fixture utility found a problem", "data": {"findings": [{"rule_id": "LINT-FULL-FINDING-001", "path": "src/lib.rs", "message": "fixture full-profile finding"}]}, "diagnostics": ["fixture finding is intentionally non-pass"]}))
raise SystemExit(1)
"#;

#[derive(Clone, Copy)]
enum ToolMode {
    Pass,
    Finding,
    MissingUtilities,
}

fn lint_full_fixture(name: &str, mode: ToolMode) -> TempFixture {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "lint-full",
        name,
        target: "lint-full",
    });
    if !matches!(mode, ToolMode::MissingUtilities) {
        materialize_python_tools(&fixture.path, mode);
    }
    write_fake_tools(&fixture.path, mode);
    fixture
}

fn materialize_python_tools(root: &Path, mode: ToolMode) {
    let just = root.join(".just");
    fs::create_dir_all(&just).expect("fixture just directory");
    let output = match mode {
        ToolMode::Pass => PYTHON_PASS_OUTPUT,
        ToolMode::Finding => PYTHON_FINDING_OUTPUT,
        ToolMode::MissingUtilities => unreachable!(),
    };
    for tool in SC_LINT_PYTHON_TOOLS {
        fs::write(just.join(tool), output).expect("fixture Python utility");
    }
}

#[test]
fn lint_full_pass_preserves_profile_envelope_and_report() {
    let fixture = lint_full_fixture("pass", ToolMode::Pass);
    let output = run_lint_full(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "unexpected lint.full failure: {output:?}"
    );

    let payload = &parse_stdout(&output)["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["target"], COMMAND_ID);
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(payload["exit_status"], 0);
    assert_eq!(payload["findings_count"], 0);
    assert_eq!(payload["raw_payload"]["ok"], true);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(payload["raw_payload"]["data"]["profile"], "full");
    assert_eq!(payload["raw_payload"]["data"]["xwin"]["available"], true);
    assert_eq!(payload["raw_payload"]["data"]["xwin"]["included"], true);
    assert_eq!(payload["raw_payload"]["data"]["step_count"], 16);
    assert_eq!(payload["raw_payload"]["data"]["steps"][0]["name"], "fmt");
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][15]["name"],
        "clippy.xwin"
    );

    assert!(
        payload["raw_payload"]["data"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|step| step["status"] == "pass")
    );

    assert_report_materialized(&fixture.path, &[COMMAND_ID, "pass", "profile"]);
}

#[test]
fn lint_full_finding_stays_non_pass_with_structured_backend_payload() {
    let fixture = lint_full_fixture("finding-negative", ToolMode::Finding);
    let output = run_lint_full(&fixture);
    assert_eq!(
        output.status.code(),
        Some(2),
        "full-profile finding must return the CLI validation-failure status: {output:?}"
    );

    let payload = &parse_stdout(&output)["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["ok"], false);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "profile.steps_failed"
    );
    let failures = payload["raw_payload"]["error"]["details"]["failed_steps"]
        .as_array()
        .expect("failed steps");
    let deny = failures
        .iter()
        .find(|step| step["name"] == "deny")
        .expect("deny failed");
    assert!(
        deny["message"]
            .as_str()
            .unwrap()
            .contains("LINT-FULL-FINDING-001")
    );
    assert!(
        payload["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    assert_report_materialized(
        &fixture.path,
        &[COMMAND_ID, "failed", "LINT-FULL-FINDING-001"],
    );
}

#[test]
fn lint_full_without_materialized_utilities_is_explicit_config_error() {
    let fixture = lint_full_fixture("config-negative", ToolMode::MissingUtilities);
    let output = run_lint_full(&fixture);
    assert_eq!(
        output.status.code(),
        Some(3),
        "missing sc_lint runtime must return the CLI usage-failure status: {output:?}"
    );

    let payload = &parse_stdout(&output)["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["outcome"], "config_error");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["ok"], false);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "profile.steps_failed"
    );
    let failures = payload["raw_payload"]["error"]["details"]["failed_steps"]
        .as_array()
        .expect("failed steps");
    assert!(failures.iter().any(|step| {
        step["name"] == "deny"
            && step["message"]
                .as_str()
                .unwrap()
                .contains("ModuleNotFoundError")
    }));
    assert!(
        payload["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty()
                && items.iter().all(|item| item["code"] == "CLI.CONFIG_ERROR"))
    );
    assert_report_materialized(
        &fixture.path,
        &[COMMAND_ID, "config_error", "ModuleNotFoundError"],
    );
}

fn run_lint_full(fixture: &TempFixture) -> Output {
    sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            TARGET,
            "--json",
        ])
        .env("PATH", fixture.path_with_fake_tools())
        .output()
        .expect("run sc-compose lint full")
}

fn assert_report_materialized(root: &Path, expected_fragments: &[&str]) {
    let raw = root.join("reports/latest/sc-lint/raw/lint.full.json");
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

fn write_fake_tools(root: &Path, mode: ToolMode) {
    write_fake_cargo(
        root,
        FakeCargoOptions {
            xwin_available: matches!(mode, ToolMode::Pass),
            test_failure: false,
            fail_closed: false,
        },
    );
    #[cfg(unix)]
    {
        let bin = root.join("fake-bin");

        let python = bin.join("python3");
        let (body, exit_code) = match mode {
            ToolMode::Pass => (
                r#"printf '%s\n' '{"adapter_schema":"sc-lint-python-v1","ok":true,"summary":"fixture utility passed","data":{"findings":[]},"diagnostics":[]}'"#,
                "0",
            ),
            ToolMode::Finding => (
                r#"printf '%s\n' '{"adapter_schema":"sc-lint-python-v1","ok":false,"summary":"fixture utility found a problem","data":{"findings":[{"rule_id":"LINT-FULL-FINDING-001","path":"src/lib.rs","message":"fixture full-profile finding"}]},"diagnostics":["fixture finding is intentionally non-pass"]}'"#,
                "1",
            ),
            ToolMode::MissingUtilities => (
                "echo 'ModuleNotFoundError: No module named sc_lint' >&2",
                "1",
            ),
        };
        support::fake_exec::install(&python, &format!("#!/bin/sh\n{body}\nexit {exit_code}\n"));
    }

    #[cfg(windows)]
    {
        let (stdout, stderr, exit_code) = match mode {
            ToolMode::Pass => (
                "{\"adapter_schema\":\"sc-lint-python-v1\",\"ok\":true,\"summary\":\"fixture utility passed\",\"data\":{\"findings\":[]},\"diagnostics\":[]}\n",
                "",
                0,
            ),
            ToolMode::Finding => (
                "{\"adapter_schema\":\"sc-lint-python-v1\",\"ok\":false,\"summary\":\"fixture utility found a problem\",\"data\":{\"findings\":[{\"rule_id\":\"LINT-FULL-FINDING-001\",\"path\":\"src/lib.rs\",\"message\":\"fixture full-profile finding\"}]},\"diagnostics\":[\"fixture finding is intentionally non-pass\"]}\n",
                "",
                1,
            ),
            ToolMode::MissingUtilities => ("", "ModuleNotFoundError: No module named sc_lint\n", 1),
        };
        support::write_fake_windows_python(root, stdout, stderr, exit_code);
    }
}
