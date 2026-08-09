//! End-to-end coverage for the top-level `ci` sc-lint target.
//!
//! These tests invoke the installed sc-lint binary through sc-compose's real
//! runner. They shim only external cargo/Python tools so the profile contract
//! is deterministic on a developer host and on Windows CI. The shared test
//! support module owns repository-root and path normalization helpers.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use support::{parse_stdout, repo_root, sc_compose, temp_root};

const TARGET: &str = "ci-all";
const COMMAND_ID: &str = "ci";

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

    assert_report_materialized(&root, &[COMMAND_ID, "pass", "tests_included"]);
}

#[test]
fn ci_test_failure_stays_non_pass_with_structured_diagnostics() {
    let (root, output) = run_ci("test-failure", ToolMode::TestFailure);
    assert_eq!(
        output.status.code(),
        Some(5),
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

    assert_report_materialized(&root, &[COMMAND_ID, "failed", "CI-TEST-FINDING-001"]);
}

#[test]
fn ci_without_materialized_utilities_is_explicit_config_error() {
    let (root, output) = run_ci("config-negative", ToolMode::MissingUtilities);
    assert_eq!(
        output.status.code(),
        Some(5),
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
    assert_report_materialized(&root, &[COMMAND_ID, "config_error", ".just/"]);
}

fn run_ci(fixture: &str, mode: ToolMode) -> (PathBuf, Output) {
    let root = temp_root(&format!("sc-lint-ci-{fixture}"));
    copy_directory(
        &repo_root().join("tests/fixtures/sc-lint/ci").join(fixture),
        &root,
    );
    fs::create_dir_all(root.join(".sc/sc-lint/targets")).expect("target registry");
    fs::copy(
        repo_root().join(".sc/sc-lint/targets/ci.toml"),
        root.join(".sc/sc-lint/targets/ci.toml"),
    )
    .expect("ci descriptor");
    if matches!(mode, ToolMode::MissingUtilities) {
        write_fake_cargo(&root, false, false);
    } else {
        write_fake_tools(&root, mode);
    }

    let output = sc_compose()
        .args([
            "lint",
            "--root",
            root.to_str().expect("UTF-8 fixture root"),
            "--target",
            TARGET,
            "--json",
        ])
        .env("PATH", path_with_fake_tools(&root))
        .output()
        .expect("run sc-compose ci");
    (root, output)
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
        matches!(mode, ToolMode::Pass),
        matches!(mode, ToolMode::TestFailure),
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

fn write_fake_cargo(root: &Path, xwin_available: bool, test_failure: bool) {
    let bin = root.join("fake-bin");
    fs::create_dir_all(&bin).expect("fake tools directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let xwin_code = if xwin_available { "0" } else { "1" };
        let test_branch = if test_failure {
            "if [ \"$1\" = \"test\" ]; then\n  printf '%s\\n' '{\"findings\":[{\"rule_id\":\"CI-TEST-FINDING-001\",\"path\":\"tests/fixture\",\"message\":\"workspace test failed\"}]}' >&2\n  exit 1\nfi\n"
        } else {
            ""
        };
        let cargo = bin.join("cargo");
        fs::write(
            &cargo,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"xwin\" ] && [ \"$2\" = \"--version\" ]; then\n  exit {xwin_code}\nfi\n{test_branch}exit 0\n"
            ),
        )
        .expect("fake cargo");
        let mut permissions = fs::metadata(&cargo)
            .expect("fake cargo metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(cargo, permissions).expect("fake cargo permissions");
    }

    #[cfg(windows)]
    {
        let xwin_code = if xwin_available { "0" } else { "1" };
        let test_branch = if test_failure {
            "if \"%1\"==\"test\" (echo {\"findings\":[{\"rule_id\":\"CI-TEST-FINDING-001\",\"path\":\"tests/fixture\",\"message\":\"workspace test failed\"}] 1>&2 & exit /b 1)\r\n"
        } else {
            ""
        };
        fs::write(
            bin.join("cargo.cmd"),
            format!(
                "@echo off\r\nif \"%1\"==\"xwin\" if \"%2\"==\"--version\" exit /b {xwin_code}\r\n{test_branch}exit /b 0\r\n"
            ),
        )
        .expect("fake cargo");
    }
}

fn path_with_fake_tools(root: &Path) -> String {
    let mut paths = vec![root.join("fake-bin")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH entries")
        .to_string_lossy()
        .into_owned()
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("fixture destination");
    for entry in fs::read_dir(source).expect("fixture source") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path);
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).expect("fixture parent");
            }
            fs::copy(source_path, destination_path).expect("fixture file");
        }
    }
}
