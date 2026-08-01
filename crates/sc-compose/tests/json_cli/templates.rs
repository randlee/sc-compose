//! Capability-oriented integration tests. Shared mechanics live in `tests/support`.
#![allow(unused_imports)]
use crate::support::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[test]
fn frontmatter_init_json_uses_diagnostic_envelope() {
    let root = temp_root("frontmatter-init-json");
    let path = root.join("template.md.j2");
    write_file(&path, "hello {{ name }}\n");

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["template_path"],
        normalize_path_str(fs::canonicalize(&path).unwrap())
    );
    assert_eq!(value["payload"]["frontmatter_added"], true);
    assert_eq!(value["payload"]["would_change"], true);
    assert_eq!(value["payload"]["vars"][0], "name");
}

#[test]
fn frontmatter_init_dry_run_json_uses_diagnostic_envelope() {
    let root = temp_root("frontmatter-init-dry-run-json");
    let path = root.join("template.md.j2");
    write_file(&path, "hello {{ name }}\n");

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&path)
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["action"], "frontmatter-init");
    assert_eq!(value["payload"]["changed"], false);
    assert_eq!(value["payload"]["would_change"], true);
    assert_eq!(value["payload"]["skipped"], false);
    assert_eq!(value["payload"]["vars"][0], "name");
}

#[test]
fn template_init_json_uses_documented_payload_fields() {
    let root = temp_root("template-init-json");
    let path = root.join("template.md");
    write_file(&path, "deploy test");

    let output = sc_compose()
        .arg("template-init")
        .arg(&path)
        .arg("--json")
        .arg("--force")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["template_path"],
        normalize_path_str(fs::canonicalize(&path).unwrap())
    );
    assert_eq!(value["payload"]["template_added"], true);
    assert_eq!(value["payload"]["would_change"], true);
    assert_eq!(value["payload"]["vars"][0], "task");
}

#[test]
fn init_json_uses_diagnostic_envelope() {
    let root = temp_root("init-json");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["workspace_root"],
        normalize_path_str(fs::canonicalize(&root).unwrap())
    );
}

#[test]
fn init_json_created_files_reflect_actual_files_written() {
    let root = temp_root("init-json-created-files");
    write_file(&root.join(".gitignore"), "target/\n");

    let output = sc_compose()
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
    assert_eq!(
        value["payload"]["created_files"],
        serde_json::json!([".prompts/", ".gitignore"])
    );
}

#[test]
fn init_dry_run_json_uses_diagnostic_envelope() {
    let root = temp_root("init-dry-run-json");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--json")
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["action"], "init");
}

#[test]
fn frontmatter_init_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("frontmatter-init-failure-json");
    let path = root.join("template.md.j2");
    write_file(
        &path,
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&path)
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
    assert_first_code(&value, "ERR_CONFIG_READONLY");
}

#[test]
fn init_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("init-failure-json");
    fs::create_dir_all(root.join(".prompts")).unwrap();
    write_file(&root.join(".gitignore"), ".prompts/\n");

    let output = sc_compose()
        .arg("init")
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
    assert_first_code(&value, "ERR_CONFIG_READONLY");
}

#[test]
fn examples_list_json_uses_diagnostic_envelope() {
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .arg("--json")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let packs = value["payload"]["packs"].as_array().unwrap();
    assert!(packs.iter().any(|pack| pack["name"] == "hello"));
    assert!(
        packs
            .iter()
            .any(|pack| pack["name"] == "report-evidence-summary")
    );
}

#[test]
fn examples_named_render_json_matches_render_schema() {
    let output = sc_compose()
        .arg("examples")
        .arg("hello")
        .arg("--var")
        .arg("name=Casey")
        .arg("--json")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["output_path"], "stdout");
    assert_eq!(
        value["payload"]["template"],
        normalize_path_str(
            repo_root()
                .join("examples")
                .join("hello.md.j2")
                .canonicalize()
                .unwrap()
        )
    );
}

#[test]
fn examples_named_render_html_dry_run_preserves_html_extension() {
    let vars_file = repo_root()
        .join("examples")
        .join("sprint-report-html.sample-vars.json");

    let output = sc_compose()
        .arg("examples")
        .arg("sprint-report-html")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--json")
        .arg("--dry-run")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["would_write"], "sprint-report-html.html");
    assert_eq!(
        value["payload"]["template"],
        normalize_path_str(
            repo_root()
                .join("examples")
                .join("sprint-report-html.html.j2")
                .canonicalize()
                .unwrap()
        )
    );
}

#[test]
fn examples_named_render_report_evidence_summary_json_matches_render_schema() {
    let vars_file = repo_root()
        .join("examples")
        .join("report-evidence-summary.sample-vars.json");

    let output = sc_compose()
        .arg("examples")
        .arg("report-evidence-summary")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--json")
        .arg("--dry-run")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(
        value["payload"]["would_write"],
        "report-evidence-summary.html"
    );
    assert_eq!(
        value["payload"]["template"],
        normalize_path_str(
            repo_root()
                .join("examples")
                .join("report-evidence-summary.html.j2")
                .canonicalize()
                .unwrap()
        )
    );
}

#[test]
fn templates_list_json_uses_diagnostic_envelope() {
    let root = temp_root("templates-list-json");
    let templates_root = root.join("user-templates");
    write_file(&templates_root.join("hello").join("hello.md.j2"), "hello");

    let output = sc_compose()
        .arg("templates")
        .arg("list")
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["packs"][0]["name"], "hello");
}

#[test]
fn templates_add_json_uses_diagnostic_envelope() {
    let root = temp_root("templates-add-json");
    let templates_root = root.join("user-templates");
    let source = root.join("hello.md.j2");
    write_file(&source, "Hello {{ name }}!");

    let output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["name"], "hello");
    assert_eq!(
        value["payload"]["source"],
        normalize_path_str(fs::canonicalize(&source).unwrap())
    );
    assert_eq!(
        value["payload"]["destination"],
        normalize_path_str(templates_root.join("hello"))
    );
    assert_eq!(value["payload"]["changed"], true);
}

#[test]
fn templates_add_duplicate_json_reports_template_exists_code() {
    let root = temp_root("templates-add-duplicate-json");
    let templates_root = root.join("user-templates");
    let source = root.join("hello.md.j2");
    write_file(&source, "Hello {{ name }}!");

    let first = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();
    assert!(first.status.success());

    let output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_TEMPLATE_EXISTS");
}

#[test]
fn templates_render_json_reports_pack_not_renderable_code() {
    let root = temp_root("templates-render-json-not-renderable");
    let templates_root = root.join("user-templates");
    write_file(&templates_root.join("ambiguous").join("one.md.j2"), "one");
    write_file(&templates_root.join("ambiguous").join("two.md.j2"), "two");

    let output = sc_compose()
        .arg("templates")
        .arg("ambiguous")
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_CONFIG_PACK_NOT_RENDERABLE");
}

#[test]
fn init_missing_root_json_reports_config_parse() {
    let root = temp_root("init-missing-root-json").join("missing");

    let output = sc_compose()
        .arg("init")
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
    assert_first_code(&value, "ERR_CONFIG_PARSE");
}
