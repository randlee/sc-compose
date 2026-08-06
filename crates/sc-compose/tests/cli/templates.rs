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
use std::time::{SystemTime, UNIX_EPOCH};

fn sprint_report_html_sample_vars() -> PathBuf {
    repo_root()
        .join("examples")
        .join("sprint-report-html.sample-vars.json")
}

#[test]
fn template_init_builds_multi_pass_template_from_pass_groups() {
    let root = temp_root("template-init-multi-pass");
    let file = root.join("agent.md");
    write_file(&file, "deploy test for wyvern");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team=wyvern")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pass: 2"), "{stdout}");
    assert!(stdout.contains("pass: 1"), "{stdout}");
    assert!(stdout.contains("{{{ team }}}"), "{stdout}");
    assert!(stdout.contains("{{ task }}"), "{stdout}");
}

#[test]
fn template_init_single_pass_omits_pass_one_marker() {
    let root = temp_root("template-init-single-pass");
    let file = root.join("agent.md");
    write_file(&file, "deploy test");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "---\nrequired_variables:\n  - task\ndefaults: {}\nmetadata: {}\n---\ndeploy {{ task }}\n"
    );
}

#[test]
fn template_init_round_trip_verifies_clean() {
    let root = temp_root("template-init-round-trip");
    let file = root.join("agent.md");
    let deployed = root.join("deployed.md");
    write_file(&file, "deploy test for wyvern");
    write_file(&deployed, "deploy test for wyvern");

    let init = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--force")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team=wyvern")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");

    let verify = sc_compose()
        .arg("verify")
        .arg("--all")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--against")
        .arg("agent.md")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team=wyvern")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("task=test")
        .arg(&deployed)
        .output()
        .unwrap();

    assert!(verify.status.success(), "{verify:?}");
    assert!(String::from_utf8_lossy(&verify.stdout).contains("OK"));
}

#[test]
fn template_init_reports_values_not_found() {
    let root = temp_root("template-init-value-not-found");
    let file = root.join("agent.md");
    write_file(&file, "deploy test");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("team=wyvern")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("values not found in file"), "{stderr}");
}

#[test]
fn template_init_rejects_duplicate_literal_assignments() {
    let root = temp_root("template-init-duplicate-literal");
    let file = root.join("agent.md");
    write_file(&file, "alpha alpha");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("first=alpha")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("second=alpha")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("values could not be substituted without overlap"),
        "{stderr}"
    );
}

#[test]
fn template_init_avoids_rewriting_inside_inserted_tokens() {
    let root = temp_root("template-init-substring-overlap");
    let file = root.join("agent.md");
    write_file(&file, "team me");

    let output = sc_compose()
        .arg("template-init")
        .arg(&file)
        .arg("--dry-run")
        .arg("--pass")
        .arg("2")
        .arg("--var")
        .arg("team_name=team")
        .arg("--pass")
        .arg("1")
        .arg("--var")
        .arg("suffix=me")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("{{{ team_name }}} {{ suffix }}"),
        "{stdout}"
    );
    assert!(!stdout.contains("{{{ tea{{ suffix }}_name }}}"), "{stdout}");
}

#[test]
fn examples_list_uses_data_dir_override() {
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hello"));
    assert!(stdout.contains("pytest-fixture"));
}

#[test]
fn examples_list_with_nonexistent_data_dir_exits_zero_and_prints_nothing() {
    let root = temp_root("examples-list-missing-data-dir");
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .env("SC_COMPOSE_DATA_DIR", root.join("missing-data-root"))
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "");
}

#[test]
fn examples_named_render_uses_data_dir_override() {
    let output = sc_compose()
        .arg("examples")
        .arg("hello")
        .arg("--var")
        .arg("name=Casey")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Hello Casey!"
    );
}

#[test]
fn examples_named_render_accepts_array_values_from_var_file() {
    let root = temp_root("examples-array");
    let vars_file = root.join("vars.json");
    write_file(
        &vars_file,
        r#"{ "module_name": "auth", "fixture_name": "fixture_state", "test_names": ["login", "logout"] }"#,
    );

    let output = sc_compose()
        .arg("examples")
        .arg("pytest-fixture")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("def test_login(fixture_state):"));
    assert!(stdout.contains("def test_logout(fixture_state):"));
}

#[test]
fn examples_named_render_sprint_report_html_renders_browser_viewable_html() {
    let vars_file = sprint_report_html_sample_vars();

    let output = sc_compose()
        .arg("examples")
        .arg("sprint-report-html")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("<title>HTML Sprint Report</title>"));
    assert!(
        stdout.contains(
            "https:&#x2f;&#x2f;github.com&#x2f;randlee&#x2f;sc-compose&#x2f;pull&#x2f;47"
        )
    );
    assert!(stdout.contains(
        "https:&#x2f;&#x2f;github.com&#x2f;randlee&#x2f;sc-compose&#x2f;actions&#x2f;runs&#x2f;118"
    ));
    assert!(stdout.contains("Plan Doc"));
    assert!(stdout.contains("Findings Doc"));
    assert!(stdout.contains("Structured object inputs"));
    assert!(stdout.contains("Arrays of objects"));
}

#[test]
fn examples_named_render_dry_run_derives_html_output_path_from_example_name() {
    let vars_file = sprint_report_html_sample_vars();

    let output = sc_compose()
        .arg("examples")
        .arg("sprint-report-html")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--dry-run")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("would_write: sprint-report-html.html"));
    assert!(stdout.contains("<!DOCTYPE html>"));
}

#[test]
fn examples_list_includes_report_evidence_summary() {
    let output = sc_compose()
        .arg("examples")
        .arg("list")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("report-evidence-summary"));
}

#[test]
fn examples_named_render_report_evidence_summary_renders_browser_viewable_html() {
    let vars_file = repo_root()
        .join("examples")
        .join("report-evidence-summary.sample-vars.json");

    let output = sc_compose()
        .arg("examples")
        .arg("report-evidence-summary")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("Report Evidence Summary"));
    assert!(stdout.contains("sc-lint style evidence family"));
    assert!(stdout.contains("reports&#x2f;latest&#x2f;publish-manifest.json"));
}

#[test]
fn general_task_template_validate_accepts_optional_input_defaults_without_explicit_values() {
    let vars_file = temp_root("general-task-validate").join("vars.json");
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-GENERAL-TASK-REVIEW-001", "description": "review", "deliverables": "pass review", "acceptance_criteria": "passes", "references": "template + dev-template" }"#,
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn general_task_template_render_uses_optional_input_defaults_when_absent() {
    let vars_file = temp_root("general-task-defaults").join("vars.json");
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-GENERAL-TASK-REVIEW-001", "description": "review", "deliverables": "pass review", "acceptance_criteria": "passes", "references": "template + dev-template" }"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"assignee="teammate""#));
    assert!(!stdout.contains("<worktree>"));
    assert!(!stdout.contains("<branch>"));
    assert!(!stdout.contains("<pr-target>"));
}

#[test]
fn general_task_template_render_dry_run_reports_default_usage_info() {
    let vars_file = temp_root("general-task-dry-run-defaults").join("vars.json");
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-GENERAL-TASK-REVIEW-001", "description": "review", "deliverables": "pass review", "acceptance_criteria": "passes", "references": "template + dev-template" }"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"variable assignee not provided, using default: "teammate""#));
}

#[test]
fn general_task_template_render_allows_overriding_optional_input_defaults() {
    let vars_file = temp_root("general-task-override").join("vars.json");
    let worktree_path = std::env::temp_dir().join(format!(
        "wt-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    write_file(
        &vars_file,
        &serde_json::json!({
            "task_id": "SC-GENERAL-TASK-REVIEW-001",
            "assignee": "architect",
            "description": "review",
            "worktree_path": normalize_path_str(&worktree_path),
            "branch": "feat/x",
            "pr_target": "develop",
            "deliverables": "pass review",
            "acceptance_criteria": "passes",
            "references": "template + dev-template"
        })
        .to_string(),
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(repo_root())
        .arg("--file")
        .arg(".claude/skills/team-lead/general-task-template.xml.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"assignee="architect""#));
    let escaped_worktree_path = normalize_path_str(&worktree_path).replace('/', "&#x2f;");
    assert!(stdout.contains(&format!("<worktree>{escaped_worktree_path}</worktree>")));
    assert!(stdout.contains("<branch>feat&#x2f;x</branch>"));
    assert!(stdout.contains("<pr-target>develop</pr-target>"));
}

#[test]
fn templates_named_render_dry_run_reports_template_json_default_usage() {
    let root = temp_root("template-json-default-usage");
    let templates_root = root.join("templates");
    let pack_root = templates_root.join("report");
    write_file(
        &pack_root.join("template.json"),
        r#"{ "description": "Report template", "version": "1.0.0", "input_defaults": { "name": "world" } }"#,
    );
    write_file(&pack_root.join("report.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("templates")
        .arg("report")
        .arg("--dry-run")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"variable name not provided, using default: "world""#));
}

#[test]
fn examples_named_render_missing_pack_reports_list_recovery_hint() {
    let output = sc_compose()
        .arg("examples")
        .arg("missing-pack")
        .env("SC_COMPOSE_DATA_DIR", repo_root())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_FOUND"));
    assert!(stderr.contains("sc-compose examples list"));
}

#[test]
fn examples_named_render_reports_not_renderable_when_example_names_collide() {
    let root = temp_root("examples-collision");
    let examples_root = root.join("examples");
    write_file(&examples_root.join("hello.md.j2"), "hello");
    write_file(&examples_root.join("hello.yaml.j2"), "hello");

    let output = sc_compose()
        .arg("examples")
        .arg("hello")
        .env("SC_COMPOSE_DATA_DIR", &root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_RENDERABLE"));
    assert!(stderr.contains("ambiguous"));
}

#[test]
fn templates_named_render_uses_array_input_defaults_from_template_json() {
    let root = temp_root("templates-array-defaults");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("pytest-defaults");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Pytest defaults", "version": "1.0.0", "input_defaults": { "fixture_name": "fixture_state", "test_names": ["login", "logout"] } }"#,
    );
    write_file(
        &pack.join("pytest-tests.py.j2"),
        "{% for test_name in test_names %}def test_{{ test_name }}({{ fixture_name }}):\n    pytest.fail(\"Fail: Not implemented\")\n\n{% endfor %}",
    );

    let output = sc_compose()
        .arg("templates")
        .arg("pytest-defaults")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("def test_login(fixture_state):"));
    assert!(stdout.contains("def test_logout(fixture_state):"));
}

#[test]
fn template_json_object_input_defaults_obey_precedence() {
    let root = temp_root("templates-object-default-precedence");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("report");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Report defaults", "version": "1.0.0", "input_defaults": { "pr": { "number": 43, "url": "https://example.test/pr/43" } } }"#,
    );
    write_file(
        &pack.join("report.md.j2"),
        "---\ndefaults:\n  pr:\n    number: 7\n    url: https://frontmatter.test/pr/7\n---\nPR #{{ pr.number }} -> {{ pr.url }}\n",
    );

    let default_output = sc_compose()
        .arg("templates")
        .arg("report")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(default_output.status.success());
    assert_eq!(
        String::from_utf8(default_output.stdout).unwrap().trim(),
        "PR #43 -> https://example.test/pr/43"
    );

    let vars_file = root.join("vars.json");
    write_file(
        &vars_file,
        r#"{ "pr": { "number": 99, "url": "https://input.test/pr/99" } }"#,
    );
    let explicit_output = sc_compose()
        .arg("templates")
        .arg("report")
        .arg("--var-file")
        .arg(&vars_file)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(explicit_output.status.success());
    assert_eq!(
        String::from_utf8(explicit_output.stdout).unwrap().trim(),
        "PR #99 -> https://input.test/pr/99"
    );
}

#[test]
fn templates_named_render_accepts_recursive_values_in_template_json_defaults() {
    let root = temp_root("templates-recursive-defaults");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("nested-report");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Nested report", "version": "1.0.0", "input_defaults": { "categories": [ { "name": "Added", "items": [ { "summary": "template manifest item" } ] } ] } }"#,
    );
    write_file(
        &pack.join("report.md.j2"),
        "{% for category in categories %}{% for item in category.items %}{{ category.name }}:{{ item.summary }}\n{% endfor %}{% endfor %}",
    );

    let output = sc_compose()
        .arg("templates")
        .arg("nested-report")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Added:template manifest item"));
}

#[test]
fn templates_named_render_uses_array_of_objects_input_defaults_from_template_json() {
    let root = temp_root("templates-array-object-defaults");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("sprint-summary");
    write_file(
        &pack.join("template.json"),
        r#"{ "description": "Sprint summary", "version": "1.0.0", "input_defaults": { "sprints": [ { "id": "S1", "stage": "qa" }, { "id": "S2", "stage": "merged" } ] } }"#,
    );
    write_file(
        &pack.join("report.md.j2"),
        "{% for sprint in sprints %}{{ sprint.id }}:{{ sprint.stage }}\n{% endfor %}",
    );

    let output = sc_compose()
        .arg("templates")
        .arg("sprint-summary")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("S1:qa"));
    assert!(stdout.contains("S2:merged"));
}

#[test]
fn templates_add_directory_creates_pack_and_readme_and_named_render_uses_input_defaults() {
    let root = temp_root("templates-add-dir");
    let templates_root = root.join("user-templates");
    let source_dir = root.join("report-pack");
    write_file(
        &source_dir.join("template.json"),
        r#"{ "description": "Report template", "version": "1.0.0", "input_defaults": { "name": "world" } }"#,
    );
    write_file(&source_dir.join("report.md.j2"), "Hello {{ name }}!\n");
    write_file(&source_dir.join("README.txt"), "asset");

    let add_output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source_dir)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(add_output.status.success());
    assert!(templates_root.join("README.md").exists());
    assert!(
        templates_root
            .join("report-pack")
            .join("README.txt")
            .exists()
    );

    let list_output = sc_compose()
        .arg("templates")
        .arg("list")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(list_output.status.success());
    assert!(
        String::from_utf8(list_output.stdout)
            .unwrap()
            .contains("report-pack")
    );

    let render_output = sc_compose()
        .arg("templates")
        .arg("report-pack")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(render_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&render_output.stdout).trim(),
        "Hello world!"
    );
}

#[test]
fn templates_list_with_nonexistent_template_dir_exits_zero_and_prints_nothing() {
    let root = temp_root("templates-list-missing-root");
    let output = sc_compose()
        .arg("templates")
        .arg("list")
        .env(
            "SC_COMPOSE_TEMPLATE_DIR",
            root.join("missing-templates-root"),
        )
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "");
}

#[test]
fn templates_add_duplicate_name_reports_template_exists() {
    let root = temp_root("templates-add-duplicate");
    let templates_root = root.join("user-templates");
    let source = root.join("hello.md.j2");
    write_file(&source, "Hello {{ name }}!\n");

    let first = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();
    assert!(first.status.success());

    let duplicate = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(duplicate.status.code(), Some(3));
    let stderr = String::from_utf8(duplicate.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_TEMPLATE_EXISTS"));
    assert!(stderr.contains("delete the existing template or use a different name"));
}

#[test]
fn templates_add_file_creates_pack_named_from_template_file() {
    let root = temp_root("templates-add-file");
    let templates_root = root.join("user-templates");
    let source = root.join("service-config.yaml.j2");
    write_file(&source, "name: {{ service_name }}\n");

    let output = sc_compose()
        .arg("templates")
        .arg("add")
        .arg(&source)
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(templates_root.join("service-config").is_dir());
    assert!(
        templates_root
            .join("service-config")
            .join("service-config.yaml.j2")
            .exists()
    );
}

#[test]
fn templates_named_render_reports_not_renderable_when_multiple_root_templates_exist() {
    let root = temp_root("templates-not-renderable");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("ambiguous");
    write_file(&pack.join("one.md.j2"), "one");
    write_file(&pack.join("two.md.j2"), "two");

    let output = sc_compose()
        .arg("templates")
        .arg("ambiguous")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_RENDERABLE"));
    assert!(stderr.contains("add a .j2 file to the template pack directory"));
}

#[test]
fn templates_named_render_missing_pack_reports_list_recovery_hint() {
    let root = temp_root("templates-missing-pack");
    let templates_root = root.join("user-templates");

    let output = sc_compose()
        .arg("templates")
        .arg("missing-pack")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_FOUND"));
    assert!(stderr.contains("sc-compose templates list"));
}

#[test]
fn templates_named_render_reports_parse_errors_for_invalid_template_manifest() {
    let root = temp_root("templates-invalid-manifest");
    let templates_root = root.join("user-templates");
    let pack = templates_root.join("broken");
    write_file(&pack.join("template.json"), "{ invalid json");
    write_file(&pack.join("broken.md.j2"), "hello");

    let output = sc_compose()
        .arg("templates")
        .arg("broken")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_CONFIG_PARSE"));
    assert!(!stderr.contains("ERR_CONFIG_PACK_NOT_RENDERABLE"));
}

#[test]
fn frontmatter_init_dry_run_reports_changed_and_would_change_without_writing() {
    let root = temp_root("frontmatter-dry-run-cli");
    let template = root.join("template.md.j2");
    write_file(&template, "hello {{ name }}\n");

    let output = sc_compose()
        .arg("frontmatter-init")
        .arg("--file")
        .arg(&template)
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let value = parse_stdout(&output);
    assert_eq!(value["payload"]["changed"], false);
    assert_eq!(value["payload"]["would_change"], true);
    assert_eq!(fs::read_to_string(&template).unwrap(), "hello {{ name }}\n");
}

#[test]
fn init_dry_run_does_not_create_workspace_and_reports_would_create_files() {
    let root = temp_root("init-dry-run-cli");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--dry-run")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!root.join(".prompts").exists());
    let value = parse_stdout(&output);
    assert_eq!(value["payload"]["action"], "init");
    assert!(
        !value["payload"]["would_affect"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn init_text_reports_recommendations() {
    let root = temp_root("init-text-recommendations");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("workspace_root:"));
    assert!(stdout.contains("root template has no frontmatter"));
}

#[test]
fn init_dry_run_text_reports_recommendations() {
    let root = temp_root("init-dry-run-text-recommendations");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("init")
        .arg("--root")
        .arg(&root)
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("would_affect:"));
    assert!(stdout.contains("root template has no frontmatter"));
}
