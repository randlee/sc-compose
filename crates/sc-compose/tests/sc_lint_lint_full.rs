//! End-to-end coverage for the lint.full sc-lint profile.
//!
//! The tests invoke the installed sc-lint binary through sc-compose's real
//! runner. They shim only external cargo/Python tools so the profile contract
//! is deterministic on a developer host and on Windows CI. No Python utility
//! or report template is copied into this repository.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const TARGET: &str = "lint-full";
const COMMAND_ID: &str = "lint.full";

struct TempFixture {
    path: PathBuf,
}

#[derive(Clone, Copy)]
enum ToolMode {
    Pass,
    Finding,
    MissingUtilities,
}

impl TempFixture {
    fn from_checked_in_fixture(name: &str, mode: ToolMode) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sc-compose-lint-full-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = repo_root()
            .join("tests/fixtures/sc-lint/lint-full")
            .join(name);
        copy_directory(&source, &path);
        let target_dir = path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).expect("target registry");
        fs::copy(
            repo_root().join(".sc/sc-lint/targets/lint-full.toml"),
            target_dir.join("lint-full.toml"),
        )
        .expect("lint-full target descriptor");
        if matches!(mode, ToolMode::MissingUtilities) {
            write_fake_cargo(&path, false);
        } else {
            write_fake_tools(&path, mode);
        }
        Self { path }
    }

    fn path_with_fake_tools(&self) -> String {
        let bin = self.path.join("fake-bin");
        let mut paths = vec![bin];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths)
            .expect("PATH entries")
            .to_string_lossy()
            .into_owned()
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        if std::env::var_os("SC_LINT_L14_KEEP").is_some() {
            eprintln!("preserved lint.full fixture: {}", self.path.display());
        } else {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[test]
fn lint_full_pass_preserves_profile_envelope_and_report() {
    let fixture = TempFixture::from_checked_in_fixture("pass", ToolMode::Pass);
    let output = run_lint_full(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "unexpected lint.full failure: {output:?}"
    );

    let payload = &result_payload(&output)["payload"];
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
    assert_eq!(payload["raw_payload"]["data"]["step_count"], 14);
    assert_eq!(payload["raw_payload"]["data"]["steps"][0]["name"], "fmt");
    assert_eq!(
        payload["raw_payload"]["data"]["steps"][13]["name"],
        "clippy.xwin"
    );

    assert_report_materialized(&fixture.path, &[COMMAND_ID, "pass", "profile"]);
}

#[test]
fn lint_full_finding_stays_non_pass_with_structured_backend_payload() {
    let fixture = TempFixture::from_checked_in_fixture("finding-negative", ToolMode::Finding);
    let output = run_lint_full(&fixture);
    assert_eq!(
        output.status.code(),
        Some(5),
        "full-profile finding must retain sc-lint's failure status: {output:?}"
    );

    let payload = &result_payload(&output)["payload"];
    assert_eq!(payload["command_id"], COMMAND_ID);
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["exit_status"], 5);
    assert_eq!(payload["raw_payload"]["ok"], false);
    assert_eq!(payload["raw_payload"]["command"], COMMAND_ID);
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert_eq!(payload["raw_payload"]["error"]["details"]["step"], "deny");
    assert!(
        payload["raw_payload"]["error"]["details"]["stdout"]
            .as_str()
            .expect("structured finding output")
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
    let fixture =
        TempFixture::from_checked_in_fixture("config-negative", ToolMode::MissingUtilities);
    let output = run_lint_full(&fixture);
    assert_eq!(
        output.status.code(),
        Some(5),
        "missing utility must retain sc-lint config status: {output:?}"
    );

    let payload = &result_payload(&output)["payload"];
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
    assert_report_materialized(&fixture.path, &[COMMAND_ID, "config_error", ".just/"]);
}

fn run_lint_full(fixture: &TempFixture) -> Output {
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

fn result_payload(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "sc-compose did not emit JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn write_fake_tools(root: &Path, mode: ToolMode) {
    write_fake_cargo(root, matches!(mode, ToolMode::Pass));
    let bin = root.join("fake-bin");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

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
            ToolMode::MissingUtilities => unreachable!(),
        };
        fs::write(&python, format!("#!/bin/sh\n{body}\nexit {exit_code}\n")).expect("fake python");
        let mut permissions = fs::metadata(&python)
            .expect("fake python metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(python, permissions).expect("fake python permissions");
    }

    #[cfg(windows)]
    {
        let python = bin.join("python.cmd");
        let (body, exit_code) = match mode {
            ToolMode::Pass => (
                r#"echo {"adapter_schema":"sc-lint-python-v1","ok":true,"summary":"fixture utility passed","data":{"findings":[]},"diagnostics":[]}"#,
                "0",
            ),
            ToolMode::Finding => (
                r#"echo {"adapter_schema":"sc-lint-python-v1","ok":false,"summary":"fixture utility found a problem","data":{"findings":[{"rule_id":"LINT-FULL-FINDING-001","path":"src/lib.rs","message":"fixture full-profile finding"}]},"diagnostics":["fixture finding is intentionally non-pass"]}"#,
                "1",
            ),
            ToolMode::MissingUtilities => unreachable!(),
        };
        fs::write(
            &python,
            format!("@echo off\r\n{body}\r\nexit /b {exit_code}\r\n"),
        )
        .expect("fake python");
    }
}

fn write_fake_cargo(root: &Path, xwin_available: bool) {
    let bin = root.join("fake-bin");
    fs::create_dir_all(&bin).expect("fake tools directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let exit_code = if xwin_available { "0" } else { "1" };
        let cargo = bin.join("cargo");
        fs::write(
            &cargo,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"xwin\" ] && [ \"$2\" = \"--version\" ]; then\n  exit {exit_code}\nfi\nexit 0\n"
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
        let exit_code = if xwin_available { "0" } else { "1" };
        fs::write(
            bin.join("cargo.cmd"),
            format!(
                "@echo off\r\nif \"%1\"==\"xwin\" if \"%2\"==\"--version\" exit /b {exit_code}\r\nexit /b 0\r\n"
            ),
        )
        .expect("fake cargo");
    }
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

fn repo_root() -> PathBuf {
    let canonical = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root");
    let Some(path) = canonical.to_str() else {
        return canonical;
    };
    PathBuf::from(path.strip_prefix(r"\\?\").unwrap_or(path))
}
