use std::fs;

mod support;

use support::{CheckedInFixture, TempFixture, parse_stdout, sc_compose};

#[test]
fn template_contracts_uses_shared_scanner_and_materializes_report() {
    let fixture = TempFixture::from_checked_in_fixture(CheckedInFixture {
        group: "template-contracts",
        name: "findings",
        target: "template-contracts",
    });
    let output = sc_compose()
        .args([
            "lint",
            "--root",
            fixture.path.to_str().expect("UTF-8 fixture root"),
            "--target",
            "template-contracts",
            "--json",
        ])
        .output()
        .expect("run template-contracts target");

    assert_eq!(output.status.code(), Some(2));
    let payload = &parse_stdout(&output)["payload"];
    assert_eq!(payload["command_id"], "template-contracts");
    assert_eq!(payload["outcome"], "failed");
    assert_eq!(payload["raw_payload"]["command"], "template-contracts");
    assert_eq!(payload["raw_payload"]["data"]["templates_scanned"], 3);
    assert!(
        payload["findings_count"]
            .as_u64()
            .is_some_and(|count| count >= 2)
    );
    assert!(payload["findings"].as_array().is_some_and(|findings| {
        findings.iter().any(|finding| {
            finding["mode"] == "auto"
                && finding["diagnostic_code"] == "WARN_JSON_LEGACY_ESCAPE_MODE"
                && finding["location"]["line"] == 1
                && finding["migration_recommendation"]
                    .as_str()
                    .is_some_and(|message| message.contains("docs/migration/json-escape-mode.md"))
        })
    }));

    let report = fixture.path.join("reports/latest/sc-lint/index.html");
    assert!(report.is_file());
    let report_text = fs::read_to_string(report).expect("HTML report");
    assert!(report_text.contains("template-contracts"));
    assert!(report_text.contains("WARN_JSON_LEGACY_ESCAPE_MODE"));
    assert!(
        fixture
            .path
            .join("reports/latest/sc-lint/raw/template-contracts.json")
            .is_file()
    );
}
