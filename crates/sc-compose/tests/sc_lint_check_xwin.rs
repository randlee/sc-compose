//! End-to-end coverage for the check.xwin sc-lint target.
//!
//! The temporary cargo shim exercises sc-lint's real workflow contract without
//! requiring a Windows target toolchain during this host-side integration test.
//! The unavailable branch is asserted as a capability result, never as pass.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const TARGET: &str = "check-xwin";
const COMMAND_ID: &str = "check.xwin";
const WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";

struct TempFixture {
    path: PathBuf,
}

impl TempFixture {
    fn from_checked_in_fixture(name: &str, xwin_available: bool) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sc-compose-check-xwin-{name}-{}-{nonce}",
            std::process::id()
        ));
        let source = repo_root()
            .join("tests/fixtures/sc-lint/check-xwin")
            .join(name);
        copy_directory(&source, &path);
        let target_dir = path.join(".sc/sc-lint/targets");
        fs::create_dir_all(&target_dir).expect("target registry");
        fs::copy(
            repo_root().join(".sc/sc-lint/targets/check-xwin.toml"),
            target_dir.join("check-xwin.toml"),
        )
        .expect("check-xwin target descriptor");
        write_fake_cargo(&path, xwin_available);
        Self { path }
    }

    fn path_with_fake_cargo(&self) -> String {
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
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn check_xwin_pass_preserves_workflow_envelope_and_report() {
    let fixture = TempFixture::from_checked_in_fixture("pass", true);
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
    let fixture = TempFixture::from_checked_in_fixture("capability-negative", false);
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
        .env("PATH", fixture.path_with_fake_cargo())
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

fn write_fake_cargo(root: &Path, xwin_available: bool) {
    let bin = root.join("fake-bin");
    fs::create_dir_all(&bin).expect("fake cargo directory");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let exit_code = if xwin_available { "0" } else { "1" };
        let path = bin.join("cargo");
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"xwin\" ] && {{ [ \"$2\" = \"--version\" ] || [ \"$2\" = \"check\" ]; }}; then\n  exit {exit_code}\nfi\nexit 1\n"
            ),
        )
        .expect("fake cargo");
        let mut permissions = fs::metadata(&path)
            .expect("fake cargo metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("fake cargo permissions");
    }

    #[cfg(windows)]
    {
        let exit_code = if xwin_available { "0" } else { "1" };
        let source = bin.join("fake-cargo.rs");
        let executable = bin.join("cargo.exe");
        fs::write(
            &source,
            format!(
                "fn main() {{\n    let mut args = std::env::args().skip(1);\n    let success = args.next().as_deref() == Some(\"xwin\")\n        && matches!(args.next().as_deref(), Some(\"--version\") | Some(\"check\"));\n    std::process::exit(if success {{ {exit_code} }} else {{ 1 }});\n}}\n"
            ),
        )
        .expect("fake cargo source");
        let status = Command::new("rustc")
            .args([
                "--edition",
                "2021",
                source.to_str().expect("fake cargo source path"),
                "-o",
                executable.to_str().expect("fake cargo executable path"),
            ])
            .status()
            .expect("compile fake cargo");
        assert!(status.success(), "fake cargo compilation failed: {status}");
        fs::remove_file(source).expect("remove fake cargo source");
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
