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
fn report_catalog_loads_valid_catalog_from_repo_root() {
    let root = temp_root("report-catalog-valid");
    write_report_catalog(&root, valid_report_catalog());

    let output = sc_compose()
        .arg("report-catalog")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("catalog:"));
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("sc-lint kind=lint producer=just lint required=true"));
}

#[test]
fn report_catalog_rejects_duplicate_ids_before_generation() {
    let root = temp_root("report-catalog-duplicate");
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

[[report]]
id = "sc-lint"
kind = "smoke"
producer = "just smoke"
required = false
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PARSE"));
    assert!(stderr.contains("duplicate report id 'sc-lint'"));
}

#[test]
fn report_catalog_rejects_missing_required_fields() {
    let root = temp_root("report-catalog-missing-field");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing required field 'producer'"));
}

#[test]
fn report_catalog_rejects_unknown_kind() {
    let root = temp_root("report-catalog-kind");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "mystery"
producer = "just lint"
required = true
entrypoint = "reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unsupported kind 'mystery'"));
}

#[test]
fn report_catalog_rejects_non_normalized_relative_paths() {
    let root = temp_root("report-catalog-path");
    write_report_catalog(
        &root,
        r#"
[[report]]
id = "sc-lint"
kind = "lint"
producer = "just lint"
required = true
entrypoint = "../reports/latest/sc-lint/index.html"
metadata = "reports/latest/sc-lint/report.json"
"#,
    );

    let output = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("field 'entrypoint' must be a normalized relative path"));
}

#[test]
fn reports_init_creates_scaffold_and_catalog_passes_validator() {
    let root = temp_root("reports-init");

    let init = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(init.status.success());
    assert!(
        root.join("reports")
            .join("catalog")
            .join("reports.toml")
            .exists()
    );
    assert!(root.join("reports").join("latest").exists());
    assert!(root.join("reports").join("archive").exists());
    assert!(root.join("reports").join("templates").exists());
    assert!(
        root.join("reports")
            .join("smoke")
            .join("reference-template.html.j2")
            .exists()
    );
    assert!(
        root.join("reports")
            .join("smoke")
            .join("sample-vars.json")
            .exists()
    );

    let validate = sc_compose()
        .arg("report-catalog")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(validate.status.success());
    let stdout = String::from_utf8(validate.stdout).unwrap();
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("smoke kind=smoke producer=just smoke required=true"));
}

#[test]
fn reports_smoke_writes_latest_smoke_artifact_set() {
    let root = temp_root("reports-smoke");
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
        .output()
        .unwrap();

    assert!(output.status.success());
    let entrypoint = root
        .join("reports")
        .join("latest")
        .join("smoke")
        .join("index.html");
    let metadata = root
        .join("reports")
        .join("latest")
        .join("smoke")
        .join("report.json");
    assert!(entrypoint.exists());
    assert!(metadata.exists());
    assert!(
        fs::read_to_string(&entrypoint)
            .unwrap()
            .contains("Smoke Report")
    );
    let metadata_text = fs::read_to_string(&metadata).unwrap();
    assert!(metadata_text.contains("\"report_id\": \"smoke\""));
    assert!(metadata_text.contains("\"entrypoint\": \"reports/latest/smoke/index.html\""));
    assert!(metadata_text.contains("\"produced_at\":"));
    assert!(metadata_text.contains("\"status\": \"pass\""));
}

#[test]
fn reports_smoke_archive_writes_timestamped_archive_copy() {
    let root = temp_root("reports-smoke-archive");
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
        .output()
        .unwrap();

    assert!(output.status.success());
    let archive_root = root.join("reports").join("archive");
    let archive_entries = fs::read_dir(&archive_root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(archive_entries.len(), 1);
    let archived_smoke_dir = archive_entries[0].join("smoke");
    assert!(archived_smoke_dir.join("index.html").exists());
    assert!(archived_smoke_dir.join("report.json").exists());
}

#[test]
fn reports_index_summarizes_latest_report_status() {
    let root = temp_root("reports-index");
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

    let index = sc_compose()
        .arg("reports")
        .arg("index")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(index.status.success());
    let stdout = String::from_utf8(index.stdout).unwrap();
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("smoke kind=smoke required=true status=pass"));
    assert!(stdout.contains("entrypoint=reports/latest/smoke/index.html"));
}

#[test]
fn reports_verify_fails_when_required_evidence_is_missing() {
    let root = temp_root("reports-verify-missing");
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

    let verify = sc_compose()
        .arg("reports")
        .arg("verify")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(verify.status.code(), Some(3));
    let stderr = String::from_utf8(verify.stderr).unwrap();
    assert!(stderr.contains("missing required report evidence for smoke"));
    assert!(stderr.contains("reports/latest/smoke/index.html"));
}

#[test]
fn reports_publish_manifest_writes_machine_readable_handoff() {
    let root = temp_root("reports-publish-manifest");
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

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(publish_manifest.status.success());
    let stdout = String::from_utf8(publish_manifest.stdout).unwrap();
    assert!(stdout.contains("manifest: reports/latest/publish-manifest.json"));
    assert!(stdout.contains("reports: 1"));
    assert!(stdout.contains("smoke kind=smoke entrypoint=reports/latest/smoke/index.html"));
    assert!(stdout.contains("publish_to=reports/smoke/index.html"));
    assert!(stdout.contains("archive_root: reports/archive/"));

    let manifest_path = root
        .join("reports")
        .join("latest")
        .join("publish-manifest.json");
    assert!(manifest_path.exists());
    let manifest_text = fs::read_to_string(manifest_path).unwrap();
    assert!(manifest_text.contains("\"report_id\": \"smoke\""));
    assert!(manifest_text.contains("\"publish_to\": \"reports/smoke/index.html\""));
    assert!(manifest_text.contains("\"archive_root\": \"reports/archive/"));
}

#[test]
fn reports_publish_manifest_rejects_parent_dir_artifact_from_metadata() {
    let root = temp_root("reports-publish-manifest-parent-dir");
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
    assert!(smoke.status.success(), "{smoke:?}");

    write_file(
        &root.join("reports").join("latest").join("escaped.html"),
        "<!DOCTYPE html><html><body>escaped</body></html>\n",
    );
    write_file(
        &root
            .join("reports")
            .join("latest")
            .join("smoke")
            .join("report.json"),
        r#"{
  "report_id": "smoke",
  "kind": "smoke",
  "produced_at": "2026-05-27T03:40:00Z",
  "status": "pass",
  "entrypoint": "reports/latest/smoke/index.html",
  "artifacts": [
    "reports/latest/smoke/index.html",
    "reports/latest/smoke/report.json",
    "reports/latest/smoke/../escaped.html"
  ]
}
"#,
    );

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert_eq!(publish_manifest.status.code(), Some(3));
    let stderr = String::from_utf8(publish_manifest.stderr).unwrap();
    assert!(stderr.contains("invalid publish-manifest artifact path for smoke"));
    assert!(stderr.contains("path must not contain '..' segments"));
}

#[test]
fn reports_finalize_writes_shared_sidecar_for_generic_producer_outputs() {
    let root = temp_root("reports-finalize");
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

    finalize_report(
        &root,
        "sc-lint",
        "lint",
        "reports/latest/sc-lint/index.html",
        &[
            "reports/latest/sc-lint/index.html",
            "reports/latest/sc-lint/panels/manifest.json",
        ],
    );

    let metadata = root
        .join("reports")
        .join("latest")
        .join("sc-lint")
        .join("report.json");
    assert!(metadata.exists());
    let metadata_text = fs::read_to_string(&metadata).unwrap();
    assert!(metadata_text.contains("\"report_id\": \"sc-lint\""));
    assert!(metadata_text.contains("\"kind\": \"lint\""));
    assert!(metadata_text.contains("\"entrypoint\": \"reports/latest/sc-lint/index.html\""));

    let archive_root = root.join("reports").join("archive");
    assert!(archive_root.exists());
}

#[test]
fn reports_render_spec_writes_mermaid_latest_artifact_set() {
    let root = temp_root("reports-render-spec");
    write_state_machine_spec(&root, "reports/specs/state-diagrams.toml");

    let output = sc_compose()
        .arg("reports")
        .arg("render-spec")
        .arg("--root")
        .arg(&root)
        .arg("--spec")
        .arg("reports/specs/state-diagrams.toml")
        .arg("--archive")
        .output()
        .unwrap();

    assert!(output.status.success());
    let entrypoint = root
        .join("reports")
        .join("latest")
        .join("state-diagrams")
        .join("index.mmd");
    let metadata = root
        .join("reports")
        .join("latest")
        .join("state-diagrams")
        .join("report.json");
    assert!(entrypoint.exists());
    assert!(metadata.exists());
    let rendered = fs::read_to_string(&entrypoint).unwrap();
    assert!(rendered.contains("stateDiagram-v2"));
    assert!(rendered.contains("accepted --> validated : validate_ok"));
    let metadata_text = fs::read_to_string(&metadata).unwrap();
    assert!(metadata_text.contains("\"report_id\": \"state-diagrams\""));
    assert!(metadata_text.contains("\"kind\": \"state_machine\""));
}

#[test]
fn reports_render_spec_participates_in_publish_manifest_pipeline() {
    let root = temp_root("reports-render-spec-publish-manifest");
    write_state_machine_spec(&root, "reports/specs/state-diagrams.toml");
    write_report_catalog(
        &root,
        r#"[[report]]
id = "state-diagrams"
kind = "state_machine"
producer = "just state-diagrams"
required = true
entrypoint = "reports/latest/state-diagrams/index.mmd"
metadata = "reports/latest/state-diagrams/report.json"
"#,
    );

    let render = sc_compose()
        .arg("reports")
        .arg("render-spec")
        .arg("--root")
        .arg(&root)
        .arg("--spec")
        .arg("reports/specs/state-diagrams.toml")
        .arg("--archive")
        .output()
        .unwrap();
    assert!(render.status.success());

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(publish_manifest.status.success());
    let manifest_text = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("publish-manifest.json"),
    )
    .unwrap();
    assert!(manifest_text.contains("\"report_id\": \"state-diagrams\""));
    assert!(manifest_text.contains("\"path\": \"reports/latest/state-diagrams/index.mmd\""));
    assert!(manifest_text.contains("\"publish_to\": \"reports/state-diagrams/index.mmd\""));
}

#[test]
fn report_render_many_accepts_semantic_spec_inputs_for_diagram_family() {
    let root = temp_root("report-render-many-semantic-spec");
    write_state_machine_spec(&root, "docs/specs/first.toml");
    write_sql_query_spec(&root, "docs/specs/second.toml");

    let output = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-diagrams")
        .arg("--glob")
        .arg("docs/specs/*.toml")
        .arg("--template-family")
        .arg("diagram")
        .arg("--output-dir")
        .arg("reports/latest/state-diagrams")
        .output()
        .unwrap();

    assert!(output.status.success());
    let first = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("state-diagrams")
            .join("docs")
            .join("specs")
            .join("first.html"),
    )
    .unwrap();
    assert!(first.contains("Diagram report family"));
    assert!(first.contains("stateDiagram-v2"));
    let second = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("state-diagrams")
            .join("docs")
            .join("specs")
            .join("second.html"),
    )
    .unwrap();
    assert!(second.contains("flowchart TD"));
    assert!(second.contains("read: orders"));
}

#[test]
fn phase_b_reference_fixtures_produce_publish_manifest_for_distinct_report_families() {
    let root = temp_root("phase-b-b8-reference");
    stage_phase_b_reference_assets(&root);

    let lint = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("sc-lint")
        .arg("--glob")
        .arg("reports/inputs/lint/*.md")
        .arg("--template-family")
        .arg("lint")
        .arg("--output-dir")
        .arg("reports/latest/sc-lint/panels")
        .output()
        .unwrap();
    assert!(lint.status.success(), "{lint:?}");
    render_report_summary(
        &root,
        "reports/vars/sc-lint-summary.json",
        "reports/latest/sc-lint/index.html",
    );
    finalize_report(
        &root,
        "sc-lint",
        "lint",
        "reports/latest/sc-lint/index.html",
        &[
            "reports/latest/sc-lint/index.html",
            "reports/latest/sc-lint/panels/manifest.json",
            "reports/latest/sc-lint/panels/reports/inputs/lint/summary.html",
            "reports/latest/sc-lint/panels/reports/inputs/lint/whitespace.html",
        ],
    );

    let test = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("test-evidence")
        .arg("--glob")
        .arg("reports/inputs/test/*.md")
        .arg("--template-family")
        .arg("test")
        .arg("--output-dir")
        .arg("reports/latest/test-evidence/panels")
        .output()
        .unwrap();
    assert!(test.status.success(), "{test:?}");
    render_report_summary(
        &root,
        "reports/vars/test-evidence-summary.json",
        "reports/latest/test-evidence/index.html",
    );
    finalize_report(
        &root,
        "test-evidence",
        "test",
        "reports/latest/test-evidence/index.html",
        &[
            "reports/latest/test-evidence/index.html",
            "reports/latest/test-evidence/panels/manifest.json",
            "reports/latest/test-evidence/panels/reports/inputs/test/results.html",
            "reports/latest/test-evidence/panels/reports/inputs/test/matrix.html",
        ],
    );

    let init = sc_compose()
        .arg("reports")
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");

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
    assert!(smoke.status.success(), "{smoke:?}");

    let state = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("state-diagrams")
        .arg("--glob")
        .arg("reports/specs/state-diagrams/*.toml")
        .arg("--template-family")
        .arg("diagram")
        .arg("--output-dir")
        .arg("reports/latest/state-diagrams/panels")
        .output()
        .unwrap();
    assert!(state.status.success(), "{state:?}");
    render_report_summary(
        &root,
        "reports/vars/state-diagrams-summary.json",
        "reports/latest/state-diagrams/index.html",
    );
    finalize_report(
        &root,
        "state-diagrams",
        "state_machine",
        "reports/latest/state-diagrams/index.html",
        &[
            "reports/latest/state-diagrams/index.html",
            "reports/latest/state-diagrams/panels/manifest.json",
            "reports/latest/state-diagrams/panels/reports/specs/state-diagrams/approval-flow.html",
            "reports/latest/state-diagrams/panels/reports/specs/state-diagrams/retry-loop.html",
        ],
    );

    let sql = sc_compose()
        .arg("report-render-many")
        .arg("--root")
        .arg(&root)
        .arg("--id")
        .arg("sql-diagrams")
        .arg("--glob")
        .arg("reports/specs/sql-diagrams/*.toml")
        .arg("--template-family")
        .arg("diagram")
        .arg("--output-dir")
        .arg("reports/latest/sql-diagrams/panels")
        .output()
        .unwrap();
    assert!(sql.status.success(), "{sql:?}");
    render_report_summary(
        &root,
        "reports/vars/sql-diagrams-summary.json",
        "reports/latest/sql-diagrams/index.html",
    );
    finalize_report(
        &root,
        "sql-diagrams",
        "sql_query",
        "reports/latest/sql-diagrams/index.html",
        &[
            "reports/latest/sql-diagrams/index.html",
            "reports/latest/sql-diagrams/panels/manifest.json",
            "reports/latest/sql-diagrams/panels/reports/specs/sql-diagrams/publish-manifest.html",
            "reports/latest/sql-diagrams/panels/reports/specs/sql-diagrams/release-summary.html",
        ],
    );

    render_report_summary(
        &root,
        "examples/report-evidence-summary.sample-vars.json",
        "reports/latest/report-evidence-summary/index.html",
    );
    finalize_report(
        &root,
        "report-evidence-summary",
        "custom",
        "reports/latest/report-evidence-summary/index.html",
        &["reports/latest/report-evidence-summary/index.html"],
    );

    let verify = sc_compose()
        .arg("reports")
        .arg("verify")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(verify.status.success(), "{verify:?}");

    let publish_manifest = sc_compose()
        .arg("reports")
        .arg("publish-manifest")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(publish_manifest.status.success(), "{publish_manifest:?}");

    let state_panel = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("state-diagrams")
            .join("panels")
            .join("reports")
            .join("specs")
            .join("state-diagrams")
            .join("approval-flow.html"),
    )
    .unwrap();
    assert!(state_panel.contains("Diagram report family"));
    assert!(state_panel.contains("Copy json"));

    let lint_panel = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("sc-lint")
            .join("panels")
            .join("reports")
            .join("inputs")
            .join("lint")
            .join("whitespace.html"),
    )
    .unwrap();
    assert!(lint_panel.contains("Whitespace edge case"));

    let manifest_text = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("publish-manifest.json"),
    )
    .unwrap();
    assert!(manifest_text.contains("\"report_id\": \"sc-lint\""));
    assert!(manifest_text.contains("\"report_id\": \"test-evidence\""));
    assert!(manifest_text.contains("\"report_id\": \"smoke\""));
    assert!(manifest_text.contains("\"report_id\": \"state-diagrams\""));
    assert!(manifest_text.contains("\"report_id\": \"sql-diagrams\""));
    assert!(manifest_text.contains("\"report_id\": \"report-evidence-summary\""));
}

#[test]
fn report_evidence_summary_renders_large_variable_payload() {
    let root = temp_root("report-evidence-summary-large-payload");
    stage_phase_b_reference_assets(&root);
    let vars_path = root
        .join("examples")
        .join("report-evidence-summary.large-vars.json");
    let output_path = root
        .join("reports")
        .join("latest")
        .join("report-evidence-summary")
        .join("large.html");
    write_large_report_evidence_vars(&vars_path);
    fs::create_dir_all(output_path.parent().unwrap()).unwrap();

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("examples/report-evidence-summary.html.j2")
        .arg("--var-file")
        .arg(&vars_path)
        .arg("--output")
        .arg(&output_path)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let rendered = fs::read_to_string(&output_path).unwrap();
    assert!(rendered.contains("Large Report Evidence Summary"));
    assert!(rendered.contains("Generated item 0"));
    assert!(rendered.contains("Generated item 159"));
}

#[test]
fn report_render_many_renders_one_output_per_source_in_sorted_order() {
    let root = temp_root("report-render-many");
    write_render_many_fixture(&root);
    write_file(
        &root.join("docs").join("diagrams").join("b.txt"),
        "# title: Bravo\n# sets:\n#   - publish\nbravo body\n",
    );
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "# title: Alpha\nalpha body\n",
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
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(
            root.join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("diagrams")
                .join("a.html")
        )
        .unwrap(),
        "<article>Alpha|alpha body|reports/latest/panels/docs/diagrams/a.html</article>"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("diagrams")
                .join("b.html")
        )
        .unwrap(),
        "<article>Bravo|bravo body|reports/latest/panels/docs/diagrams/b.html|publish</article>"
    );

    let manifest = fs::read_to_string(
        root.join("reports")
            .join("latest")
            .join("panels")
            .join("manifest.json"),
    )
    .unwrap();
    let alpha_index = manifest
        .find("\"source_path\": \"docs/diagrams/a.txt\"")
        .unwrap();
    let bravo_index = manifest
        .find("\"source_path\": \"docs/diagrams/b.txt\"")
        .unwrap();
    assert!(alpha_index < bravo_index);
}

#[test]
fn report_render_many_accepts_repo_relative_root_argument() {
    let root = temp_root("report-render-many-relative-root");
    write_render_many_fixture(&root);
    write_file(
        &root.join("docs").join("diagrams").join("a.txt"),
        "# title: Alpha\nalpha body\n",
    );

    let output = sc_compose()
        .current_dir(&root)
        .arg("report-render-many")
        .arg("--root")
        .arg(".")
        .arg("--id")
        .arg("state-machines")
        .arg("--glob")
        .arg("docs/diagrams/*.txt")
        .arg("--template")
        .arg("reports/templates/panel.html.j2")
        .arg("--output-dir")
        .arg("reports/latest/panels")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_to_string(
            root.join("reports")
                .join("latest")
                .join("panels")
                .join("docs")
                .join("diagrams")
                .join("a.html")
        )
        .unwrap(),
        "<article>Alpha|alpha body|reports/latest/panels/docs/diagrams/a.html</article>"
    );
}

#[test]
fn report_render_many_supports_shared_diagram_template_selector() {
    let root = temp_root("report-render-many-shared");
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
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
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
    assert!(rendered.contains("<pre>alpha body</pre>"));
    assert!(rendered.contains("Copy text"));
    assert!(rendered.contains("Open fragment"));
    assert!(!rendered.contains("Copy json"));
}

#[test]
fn report_render_many_uses_repo_local_template_family_override() {
    let root = temp_root("report-render-many-family-override");
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
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
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
    assert!(rendered.contains("Override body marker"));
    assert!(rendered.contains("Copy text"));
    assert!(rendered.contains("Copy json"));
}
