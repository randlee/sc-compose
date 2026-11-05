use std::fs;
mod support;
use support::{CheckedInFixture, TempFixture, normalize_path_str, parse_stdout, sc_compose};

fn run_view_findings(fixture: &TempFixture) -> std::process::Output {
    sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "view-findings",
            "--json",
        ])
        .output()
        .expect("run sc-compose view findings")
}

#[test]
fn view_findings_pass_preserves_identity_and_materializes_report() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "view-findings",
        name: "pass",
        target: "view-findings",
    });
    let output = run_view_findings(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];

    assert_eq!(envelope["schema_version"], "1");
    assert_eq!(payload["command_id"], "view.findings");
    assert_eq!(payload["target"], "view.findings");
    assert_eq!(payload["raw_payload"]["command"], "view.findings");
    assert!(payload["raw_payload"]["diagnostics"].is_array());

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
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "view-findings",
        name: "malformed-summary",
        target: "view-findings",
    });
    let output = run_view_findings(&fixture);
    let envelope = parse_stdout(&output);
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
    assert!(report_text.contains("CLI.BACKEND_PROTOCOL_ERROR"));
    assert!(report_text.contains("failed"));
}

#[test]
fn view_findings_missing_runtime_is_explicit_and_non_pass() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "view-findings",
        name: "pass",
        target: "view-findings",
    });
    let bin = fixture.path.join("fake-bin");
    fs::create_dir_all(&bin).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in ["python", "python3"] {
            let path = bin.join(name);
            fs::write(
                &path,
                "#!/bin/sh\necho 'ModuleNotFoundError: No module named sc_lint' >&2\nexit 1\n",
            )
            .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    #[cfg(windows)]
    {
        let source = bin.join("missing-python.rs");
        let executable = bin.join("python.exe");
        fs::write(&source, "fn main() { eprintln!(\"ModuleNotFoundError: No module named sc_lint\"); std::process::exit(1); }").unwrap();
        let status = std::process::Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap();
        assert!(status.success());
        fs::copy(&executable, bin.join("python3.exe")).unwrap();
    }
    let output = sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().unwrap(),
            "--target",
            "view-findings",
            "--json",
        ])
        .env("PATH", fixture.path_with_fake_tools())
        .output()
        .unwrap();
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "view.findings");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(
        payload["raw_payload"]["error"]["code"],
        "CLI.BACKEND_EXEC_FAILURE"
    );
    assert!(
        payload["raw_payload"]["error"]["cause"]
            .as_str()
            .unwrap()
            .contains("ModuleNotFoundError")
    );
    assert!(!payload["diagnostics"].as_array().unwrap().is_empty());
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/view.findings.json")
            .is_file()
    );
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/index.html")
            .is_file()
    );
}
