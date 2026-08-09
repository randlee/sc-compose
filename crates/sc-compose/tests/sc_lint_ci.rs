//! End-to-end coverage for the top-level `ci` sc-lint target.
//!
//! These tests invoke the installed sc-lint binary through sc-compose's real
//! runner. They shim only external cargo/Python tools so the profile contract
//! is deterministic on a developer host and on Windows CI. The shared test
//! support module owns repository-root and path normalization helpers.

mod support;

use std::fs;
use std::path::Path;
use std::process::Output;

use support::{
    CheckedInFixture, FakeCargoOptions, SC_LINT_PYTHON_TOOLS, TempFixture, parse_stdout,
    sc_compose, write_fake_cargo,
};

const TARGET: &str = "ci-all";
const COMMAND_ID: &str = "ci";
const PYTHON_TOOL_OUTPUT: &str = r#"import json
print(json.dumps({"adapter_schema": "sc-lint-python-v1", "ok": True, "summary": "fixture utility passed", "data": {"findings": []}, "diagnostics": []}))
"#;

#[derive(Clone, Copy)]
enum ToolMode {
    Pass,
    TestFailure,
    MissingUtilities,
}

#[test]
fn ci_pass_preserves_composite_envelope_and_excludes_xwin() {
    let (root, output) = run_ci("pass", ToolMode::Pass);
    assert_eq!(
        output.status.code(),
        Some(0),
        "unexpected ci failure: {output:?}"
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
    assert_eq!(payload["raw_payload"]["data"]["lint_profile"], "ci");
    assert_eq!(payload["raw_payload"]["data"]["tests_included"], true);
    assert_eq!(payload["raw_payload"]["data"]["xwin"]["available"], true);
    assert_eq!(payload["raw_payload"]["data"]["xwin"]["included"], false);
    assert_eq!(payload["raw_payload"]["data"]["step_count"], 11);
    assert_eq!(payload["raw_payload"]["data"]["steps"][0]["name"], "fmt");
    assert_eq!(payload["raw_payload"]["data"]["steps"][10]["name"], "test");
    assert!(
        payload["raw_payload"]["data"]["steps"]
            .as_array()
            .expect("ci steps")
            .iter()
            .all(|step| !step["name"]
                .as_str()
                .is_some_and(|name| name.contains("xwin")))
    );

    assert_report_materialized(&root.path, &[COMMAND_ID, "pass", "tests_included"]);
}

#[test]
fn ci_test_failure_stays_non_pass_with_structured_diagnostics() {
    let (root, output) = run_ci("test-failure", ToolMode::TestFailure);
    assert_eq!(
        output.status.code(),
        Some(2),
        "ci test failure must retain sc-lint's failure status: {output:?}"
    );

    let payload = &parse_stdout(&output)["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["ok"], false);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(payload["raw_payload"]["error"]["details"]["step"], "test");
    assert!(
        payload["raw_payload"]["error"]["details"]["stderr"]
            .as_str()
            .expect("structured test finding")
            .contains("CI-TEST-FINDING-001")
    );
    assert!(
        payload["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    assert_report_materialized(&root.path, &[COMMAND_ID, "failed", "CI-TEST-FINDING-001"]);
}

#[test]
fn ci_without_materialized_utilities_is_explicit_config_error() {
    let (root, output) = run_ci("config-negative", ToolMode::MissingUtilities);
    assert_eq!(
        output.status.code(),
        Some(3),
        "missing utility must retain sc-lint's subprocess status: {output:?}"
    );

    let payload = &parse_stdout(&output)["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["outcome"], "config_error");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["ok"], false);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert!(
        payload["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert_report_materialized(&root.path, &[COMMAND_ID, "config_error", ".just/"]);
}

fn run_ci(fixture: &str, mode: ToolMode) -> (TempFixture, Output) {
    let root = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "ci",
        name: fixture,
        target: "ci",
    });
    if matches!(mode, ToolMode::MissingUtilities) {
        write_fake_cargo(
            &root.path,
            FakeCargoOptions {
                xwin_available: false,
                test_failure: false,
                fail_closed: false,
            },
        );
    } else {
        materialize_python_tools(&root.path);
        write_fake_tools(&root.path, mode);
    }

    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            TARGET,
            "--json",
        ])
        .env("PATH", root.path_with_fake_tools())
        .output()
        .expect("run sc-compose ci");
    (root, output)
}

fn materialize_python_tools(root: &Path) {
    let just = root.join(".just");
    fs::create_dir_all(&just).expect("fixture just directory");
    for tool in SC_LINT_PYTHON_TOOLS {
        fs::write(just.join(tool), PYTHON_TOOL_OUTPUT).expect("fixture Python utility");
    }
}

fn assert_report_materialized(root: &Path, expected_fragments: &[&str]) {
    let raw = root.join("reports/latest/sc-lint/raw/ci.json");
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
            test_failure: matches!(mode, ToolMode::TestFailure),
            fail_closed: false,
        },
    );
    let bin = root.join("fake-bin");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let python = bin.join("python3");
        fs::write(
            &python,
            "#!/bin/sh\nprintf '%s\\n' '{\"adapter_schema\":\"sc-lint-python-v1\",\"ok\":true,\"summary\":\"fixture utility passed\",\"data\":{\"findings\":[]},\"diagnostics\":[]}'\nexit 0\n",
        )
        .expect("fake python");
        let mut permissions = fs::metadata(&python)
            .expect("fake python metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(python, permissions).expect("fake python permissions");
    }

    #[cfg(windows)]
    {
        fs::write(
            bin.join("python.cmd"),
            "@echo off\r\necho {\"adapter_schema\":\"sc-lint-python-v1\",\"ok\":true,\"summary\":\"fixture utility passed\",\"data\":{\"findings\":[]},\"diagnostics\":[]}\r\nexit /b 0\r\n",
        )
        .expect("fake python");
    }
}
