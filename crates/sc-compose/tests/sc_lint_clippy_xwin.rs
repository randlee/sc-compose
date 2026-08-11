use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

mod support;

use support::{CheckedInFixture, TempFixture, normalize_path_str, parse_stdout, sc_compose};

static CLIPPY_XWIN_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn run_clippy_xwin(fixture: &TempFixture) -> std::process::Output {
    let _guard = CLIPPY_XWIN_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("clippy xwin test lock");
    sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "clippy-xwin",
            "--json",
        ])
        .output()
        .expect("run sc-compose clippy xwin")
}

fn cargo_xwin_available() -> bool {
    Command::new("cargo")
        .args(["xwin", "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn assert_report_materialized(fixture: &TempFixture, report_text: &str) {
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/clippy.xwin.json")
            .is_file()
    );
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/index.html")
            .is_file()
    );
    assert!(report_text.contains("clippy.xwin"));
    let raw_report = normalize_path_str(
        Path::new("reports")
            .join("latest")
            .join("sc-lint")
            .join("raw")
            .join("clippy.xwin.json"),
    );
    assert!(report_text.contains(&raw_report));
}

#[test]
fn clippy_xwin_pass_preserves_identity_and_materializes_report() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "clippy-xwin",
        name: "pass",
        target: "clippy-xwin",
    });
    let output = run_clippy_xwin(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];

    assert_eq!(envelope["schema_version"], "1");
    assert_eq!(payload["command_id"], "clippy.xwin");
    assert_eq!(payload["target"], "clippy.xwin");
    assert_eq!(payload["raw_payload"]["command"], "clippy.xwin");
    assert!(payload["diagnostics"].is_array());
    assert_eq!(payload["findings_count"], 0);

    let report_text = fs::read_to_string(fixture.path.join("reports/latest/sc-lint/index.html"))
        .expect("clippy xwin pass report");
    assert_report_materialized(&fixture, &report_text);

    if cargo_xwin_available() {
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(payload["outcome"], "pass");
        assert_eq!(payload["exit_status"], 0);
        assert_eq!(payload["raw_payload"]["ok"], true);
        assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
        assert_eq!(payload["raw_payload"]["data"]["mode"], "xwin");
        assert_eq!(
            payload["raw_payload"]["data"]["xwin"]["target"],
            "x86_64-pc-windows-msvc"
        );
        assert!(report_text.contains("pass"));
    } else {
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(payload["outcome"], "capability_error");
        assert_eq!(payload["exit_status"], 4);
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.CAPABILITY_ERROR"
        );
        assert!(report_text.contains("capability_error"));
    }
}

#[test]
fn clippy_xwin_failure_stays_non_pass_with_structured_diagnostics() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "clippy-xwin",
        name: "failing-analysis",
        target: "clippy-xwin",
    });
    let output = run_clippy_xwin(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];

    assert_eq!(envelope["schema_version"], "1");
    assert_eq!(payload["command_id"], "clippy.xwin");
    assert_eq!(payload["target"], "clippy.xwin");
    assert_eq!(payload["raw_payload"]["command"], "clippy.xwin");
    assert!(payload["diagnostics"].is_array());
    assert!(
        !payload["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    assert_eq!(payload["findings_count"], 0);

    let report_text = fs::read_to_string(fixture.path.join("reports/latest/sc-lint/index.html"))
        .expect("clippy xwin failure report");
    assert_report_materialized(&fixture, &report_text);

    if cargo_xwin_available() {
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(payload["outcome"], "failed");
        assert_eq!(payload["exit_status"], 5);
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.BACKEND_EXEC_FAILURE"
        );
        assert_eq!(
            payload["raw_payload"]["error"]["details"]["step"],
            "clippy.xwin"
        );
        assert!(
            payload["raw_payload"]["error"]["cause"]
                .as_str()
                .expect("clippy failure cause")
                .contains("unused-mut")
        );
        assert!(report_text.contains("failed"));
        assert!(report_text.contains("clippy-xwin-failing"));
    } else {
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(payload["outcome"], "capability_error");
        assert_eq!(payload["exit_status"], 4);
        assert_eq!(
            payload["raw_payload"]["error"]["code"],
            "CLI.CAPABILITY_ERROR"
        );
        assert!(report_text.contains("capability_error"));
    }
}
