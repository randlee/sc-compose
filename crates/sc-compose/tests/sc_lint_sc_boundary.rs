use std::fs;
mod support;
use support::{CheckedInFixture, TempFixture, parse_stdout, sc_compose};

fn run_sc_boundary(fixture: &TempFixture) -> std::process::Output {
    sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "sc-boundary",
            "--json",
        ])
        .env("SC_LOG_ROOT", fixture.path.join("logs"))
        .output()
        .expect("run sc-compose lint sc-boundary")
}

#[test]
fn boundary_pass_uses_shared_runner_and_materializes_evidence() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-boundary",
        name: "pass",
        target: "sc-boundary",
    });
    let output = run_sc_boundary(&fixture);
    assert_eq!(
        output.status.code(),
        Some(0),
        "lint failed; stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["outcome"], "pass");
    assert_eq!(
        payload["exit_status"].as_i64(),
        output.status.code().map(i64::from)
    );
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-boundary");
    assert_eq!(payload["raw_payload"]["data"]["status"], "pass");
    assert_eq!(payload["findings_count"], 0);
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/lint.sc-boundary.json")
            .is_file()
    );
    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("pass report");
    assert!(report_text.contains("lint.sc-boundary"));
    assert!(report_text.contains("pass"));
}

#[test]
fn boundary_dependency_violation_stays_non_pass_with_structured_finding() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-boundary",
        name: "dependency-violation",
        target: "sc-boundary",
    });
    let output = run_sc_boundary(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];

    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(
        payload["outcome"],
        "findings",
        "lint config failed: {} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        payload["exit_status"].as_i64(),
        output.status.code().map(i64::from)
    );
    assert_eq!(payload["raw_payload"]["command"], "lint.sc-boundary");
    assert_eq!(payload["raw_payload"]["data"]["status"], "fail");
    assert_eq!(payload["findings_count"], 1);
    assert_eq!(payload["findings"][0]["rule_id"], "SCB-DEPENDENCY-001");
    assert!(
        payload["findings"][0]["message"]
            .as_str()
            .expect("finding message")
            .contains("boundary-app")
    );
    assert!(
        payload["findings"][0]["message"]
            .as_str()
            .expect("finding message")
            .contains("boundary-api")
    );

    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("finding report");
    assert!(report_text.contains("findings"));
    assert!(report_text.contains("SCB-DEPENDENCY-001"));
    assert!(report_text.contains("boundary-api"));
}

#[test]
fn sc_sha_python_boundary_rejects_all_forbidden_dependency_classes() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-boundary",
        name: "sc-sha-python-dependency-violation",
        target: "sc-boundary",
    });
    let output = run_sc_boundary(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(
        payload["outcome"],
        "findings",
        "lint config failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let findings = payload["findings"].as_array().expect("findings array");
    let dependency_findings = findings
        .iter()
        .filter(|finding| finding["rule_id"] == "SCB-DEPENDENCY-001")
        .collect::<Vec<_>>();
    assert_eq!(dependency_findings.len(), 4);
    for dependency in ["sc-compose", "sc-composer", "atmcore", "unrelated-runtime"] {
        assert!(
            dependency_findings.iter().any(|finding| {
                finding["message"]
                    .as_str()
                    .is_some_and(|message| message.contains(dependency))
            }),
            "missing forbidden dependency finding for {dependency}: {findings:?}"
        );
    }
}

#[test]
fn sc_sha_boundary_rejects_renderer_dependency() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-boundary",
        name: "sc-sha-dependency-violation",
        target: "sc-boundary",
    });
    let output = run_sc_boundary(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["outcome"], "findings");
    assert_eq!(payload["findings_count"], 6);
    let messages = payload["findings"]
        .as_array()
        .expect("finding array")
        .iter()
        .map(|finding| {
            assert_eq!(finding["rule_id"], "SCB-DEPENDENCY-001");
            finding["message"].as_str().expect("finding message")
        })
        .collect::<Vec<_>>();
    for package in [
        "sc-composer",
        "sc-compose",
        "sc-compose-py",
        "filesystem-support",
        "cache-host",
        "unrelated-runtime",
    ] {
        assert!(
            messages.iter().any(|message| message.contains(package)),
            "missing forbidden dependency finding for {package}: {messages:?}"
        );
    }
}

#[test]
fn sc_sha_go_boundary_rejects_non_adapter_dependencies() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-boundary",
        name: "sc-sha-go-forbidden-edge",
        target: "sc-boundary",
    });
    let output = run_sc_boundary(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["outcome"], "findings");
    let dependency_findings = payload["findings"]
        .as_array()
        .expect("finding array")
        .iter()
        .filter(|finding| finding["rule_id"] == "SCB-DEPENDENCY-001")
        .collect::<Vec<_>>();
    assert_eq!(dependency_findings.len(), 5);
    let messages = dependency_findings
        .into_iter()
        .map(|finding| finding["message"].as_str().expect("finding message"))
        .collect::<Vec<_>>();
    for package in [
        "sc-compose",
        "sc-composer",
        "atmcore",
        "filesystem-support",
        "sc-sha-python",
    ] {
        assert!(
            messages.iter().any(|message| message.contains(package)),
            "missing forbidden dependency finding for {package}: {messages:?}"
        );
    }
}

#[test]
fn sc_composer_beads_boundary_rejects_host_coupling_and_non_contract_dependencies() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "sc-boundary",
        name: "sc-composer-beads-forbidden-edge",
        target: "sc-boundary",
    });
    let output = run_sc_boundary(&fixture);
    let envelope = parse_stdout(&output);
    let payload = &envelope["payload"];
    assert_eq!(payload["command_id"], "lint.sc-boundary");
    assert_eq!(payload["outcome"], "findings");
    let findings = payload["findings"].as_array().expect("finding array");
    let dependency_findings = findings
        .iter()
        .filter(|finding| finding["rule_id"] == "SCB-DEPENDENCY-001")
        .collect::<Vec<_>>();
    assert_eq!(dependency_findings.len(), 5);
    let messages = dependency_findings
        .into_iter()
        .map(|finding| finding["message"].as_str().expect("finding message"))
        .collect::<Vec<_>>();
    for package in [
        "sc-compose",
        "atmcore",
        "beads-source",
        "filesystem-support",
        "sc-composer-beads-python",
    ] {
        assert!(
            messages.iter().any(|message| message.contains(package)),
            "missing forbidden dependency finding for {package}: {messages:?}"
        );
    }
}
