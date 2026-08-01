//! Capability-oriented integration tests. Shared mechanics live in `tests/support`.
#![allow(
    unused_imports,
    reason = "shared support imports are selected by platform and test configuration"
)]
use crate::support::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[test]
fn report_catalog_json_uses_diagnostic_envelope() {
    let root = temp_root("report-catalog-json");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("report-catalog")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["report_count"], 1);
    assert_eq!(value["payload"]["reports"][0]["id"], "sc-lint");
    assert_eq!(
        value["payload"]["reports"][0]["metadata"],
        "reports/latest/sc-lint/report.json"
    );
}

#[test]
fn report_catalog_invalid_json_reports_config_parse() {
    let root = temp_root("report-catalog-invalid-json");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = "yes"
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert_first_code(&value, "ERR_CONFIG_PARSE");
    assert_eq!(
        value["diagnostics"][0]["message"],
        "report 'sc-lint' field 'required' must be true or false"
    );
}

#[test]
fn reports_init_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-init-json");

    let output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(
        value["payload"]["created_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "reports/latest/")
    );
    assert!(
        value["payload"]["created_paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "reports/catalog/reports.toml")
    );
}

#[test]
fn reports_smoke_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-smoke-json");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);

    let output = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["entrypoint"],
        "reports/latest/smoke/index.html"
    );
    assert_eq!(
        value["payload"]["metadata"],
        "reports/latest/smoke/report.json"
    );
    assert_eq!(value["payload"]["report_id"], "smoke");
    assert_eq!(value["payload"]["status"], "pass");
    assert!(value["payload"]["produced_at"].is_string());
}

#[test]
fn reports_index_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-index-json");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);
    write_report_catalog(
        &root,
        r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#,
    );

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .output()
        .unwrap();
    assert!(smoke.status.success());

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["report_count"], 1);
    assert_eq!(value["payload"]["entries"][0]["report_id"], "smoke");
    assert_eq!(value["payload"]["entries"][0]["status"], "pass");
}

#[test]
fn reports_verify_json_succeeds_when_required_evidence_is_present() {
    let root = temp_root("reports-verify-json-success");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);
    write_report_catalog(
        &root,
        r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#,
    );

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .output()
        .unwrap();
    assert!(smoke.status.success());

    let output = sc_compose()
        .arg("reports")
        .arg("verify")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["required_count"], 1);
    assert_eq!(value["payload"]["verified_count"], 1);
}

#[test]
fn reports_smoke_json_lists_archive_artifacts_when_requested() {
    let root = temp_root("reports-smoke-archive-json");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);

    let output = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--archive")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let archived = value["payload"]["archived_artifacts"].as_array().unwrap();
    assert_eq!(archived.len(), 2);
    for artifact in archived {
        let artifact = artifact.as_str().unwrap();
        assert!(artifact.contains("reports/archive/"));
        assert!(!artifact.contains('\\'));
    }
}

#[test]
fn reports_publish_manifest_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-publish-manifest-json");
    let init_output = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init_output.status.success());
    write_smoke_fixture(&root);
    write_report_catalog(
        &root,
        r#"[[report]]
id = "smoke"
kind = "smoke"
producer = "just smoke"
required = true
entrypoint = "reports/latest/smoke/index.html"
metadata = "reports/latest/smoke/report.json"
"#,
    );

    let smoke = sc_compose()
        .arg("reports")
        .arg("smoke")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg("reports/smoke/reference-template.html.j2")
        .arg("--vars")
        .arg("reports/smoke/sample-vars.json")
        .arg("--archive")
        .output()
        .unwrap();
    assert!(smoke.status.success());

    let output = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["manifest_path"],
        "reports/latest/publish-manifest.json"
    );
    assert_eq!(value["payload"]["report_count"], 1);
    assert_eq!(
        value["payload"]["manifest"]["reports"][0]["report_id"],
        "smoke"
    );
    assert_eq!(
        value["payload"]["manifest"]["reports"][0]["files"][0]["publish_to"],
        "reports/smoke/index.html"
    );
}

#[test]
fn reports_render_spec_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-render-spec-json");
    write_state_machine_spec(&root, "reports/specs/state-diagrams.toml");

    let output = sc_compose()
        .arg("reports")
        .arg("render-spec")
        .arg("--root")
        .arg(&root)
        .arg("--spec")
        .arg("reports/specs/state-diagrams.toml")
        .arg("--archive")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["report_id"], "state-diagrams");
    assert_eq!(value["payload"]["kind"], "state_machine");
    assert_eq!(
        value["payload"]["entrypoint"],
        "reports/latest/state-diagrams/index.mmd"
    );
    assert_eq!(
        value["payload"]["archived_artifacts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn reports_finalize_json_uses_diagnostic_envelope() {
    let root = temp_root("reports-finalize-json");
    write_file(
        &root
            .join("reports")
            .join("latest")
            .join("sc-lint")
            .join("index.html"),
        "<!DOCTYPE html><html><body>lint</body></html>\n",
    );
    write_file(
        &root
            .join("reports")
            .join("latest")
            .join("sc-lint")
            .join("panels")
            .join("manifest.json"),
        "{}\n",
    );

    let output = sc_compose()
        .arg("reports")
        .arg("finalize")
        .arg("--root")
        .arg(&root)
        .arg("--report-id")
        .arg("sc-lint")
        .arg("--kind")
        .arg("lint")
        .arg("--entrypoint")
        .arg("reports/latest/sc-lint/index.html")
        .arg("--artifact")
        .arg("reports/latest/sc-lint/index.html")
        .arg("--artifact")
        .arg("reports/latest/sc-lint/panels/manifest.json")
        .arg("--archive")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["report_id"], "sc-lint");
    assert_eq!(value["payload"]["kind"], "lint");
    assert_eq!(
        value["payload"]["metadata"],
        "reports/latest/sc-lint/report.json"
    );
    assert_eq!(
        value["payload"]["archived_artifacts"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn report_render_many_json_uses_diagnostic_envelope() {
    let root = temp_root("report-render-many-json");
    write_render_many_fixture(&root);
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "/*\ntitle: Alpha\nsets:\n  - publish\n*/\nalpha body\n",
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("reports/templates/panel.html.j2")
        .arg("--output-dir")
        .arg("reports/latest/panels")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["manifest_path"],
        "reports/latest/panels/manifest.json"
    );
    assert_eq!(
        value["payload"]["entries"][0]["source_path"],
        "docs/diagrams/a.txt"
    );
    assert_eq!(
        value["payload"]["entries"][0]["output_path"],
        "reports/latest/panels/docs/diagrams/a.html"
    );
    assert_eq!(
        value["payload"]["entries"][0]["sets"],
        serde_json::json!(["publish"])
    );
}

#[test]
fn report_render_many_json_supports_shared_template_family() {
    let root = temp_root("report-render-many-shared-family-json");
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "# title: Alpha\n# fragment_href: /reports/alpha\nalpha body\n",
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("shared:diagram")
        .arg("--output-dir")
        .arg("reports/latest/diagrams")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["entries"][0]["source_path"],
        "docs/diagrams/a.txt"
    );
    assert_eq!(
        value["payload"]["entries"][0]["output_path"],
        "reports/latest/diagrams/docs/diagrams/a.html"
    );
    let rendered = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("diagrams")
            .join("docs")
            .join("diagrams")
            .join("a.html"),
    )
    .unwrap();
    assert!(rendered.contains("Diagram report family"));
}

#[test]
fn report_render_many_json_supports_template_family_overrides() {
    let root = temp_root("report-render-many-family-json");
    write_file(
        &root.join("docs").join("lint").join("a.txt"),
        "# title: Lint Alpha\n# copy_json:\n#   status: pass\nalpha body\n",
    );
    write_report_family_override(&root);
    write_report_catalog(
        &root,
        r#"[reporting.templates.lint]
path = "reports/templates/lint/report.html.j2"
"#,
    );

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("lint")
        .arg("--glob")
        .arg("docs/lint/*.txt")
        .arg("--template-family")
        .arg("lint")
        .arg("--output-dir")
        .arg("reports/latest/lint")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["entries"][0]["source_path"],
        "docs/lint/a.txt"
    );
    assert_eq!(
        value["payload"]["entries"][0]["output_path"],
        "reports/latest/lint/docs/lint/a.html"
    );
    let rendered = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("lint")
            .join("docs")
            .join("lint")
            .join("a.html"),
    )
    .unwrap();
    assert!(rendered.contains("Lint override"));
}
