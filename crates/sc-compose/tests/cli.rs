use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sc-compose-cli-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn sc_compose() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-compose"));
    command.env("SC_LOG_ROOT", test_log_root());
    command
}

fn test_log_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("sc-compose-cli-logs-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn parse_stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

#[test]
fn render_dry_run_does_not_create_output_file() {
    let root = temp_root("dry-run");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    let output = root.join("out.md");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&output)
        .arg("--dry-run")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(!output.exists());
}

#[test]
fn exit_code_zero_for_valid_render() {
    let root = temp_root("exit-ok");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(0));
}

#[test]
fn exit_code_two_for_validation_failure() {
    let root = temp_root("exit-validation");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let status = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));
}

#[test]
fn exit_code_three_for_resolve_failure() {
    let root = temp_root("exit-resolve");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("missing.md.j2")
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(3));
}

#[test]
fn render_uses_json_var_file_inputs() {
    let root = temp_root("var-file-json");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.json");
    write_file(&vars_file, "{ \"name\": \"json-world\" }\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello json-world"
    );
}

#[test]
fn render_uses_yaml_var_file_inputs() {
    let root = temp_root("var-file-yaml");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    let vars_file = root.join("vars.yaml");
    write_file(&vars_file, "name: yaml-world\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello yaml-world"
    );
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
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-GENERAL-TASK-REVIEW-001", "assignee": "architect", "description": "review", "worktree_path": "/tmp/wt", "branch": "feat/x", "pr_target": "develop", "deliverables": "pass review", "acceptance_criteria": "passes", "references": "template + dev-template" }"#,
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
    assert!(stdout.contains("<worktree>/tmp/wt</worktree>"));
    assert!(stdout.contains("<branch>feat/x</branch>"));
    assert!(stdout.contains("<pr-target>develop</pr-target>"));
}

#[test]
fn render_treats_required_variable_as_satisfied_by_input_defaults_alias() {
    let root = temp_root("required-input-defaults");
    let vars_file = root.join("vars.json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\ninput_defaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    write_file(&vars_file, "{}");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello world"
    );
}

#[test]
fn validate_still_errors_for_variables_not_in_required_or_input_defaults() {
    let root = temp_root("unknown-variable-with-input-defaults");
    let vars_file = root.join("vars.json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - task_id\ninput_defaults:\n  assignee: teammate\n---\nhello {{ task_id }} {{ assignee }}\n",
    );
    write_file(
        &vars_file,
        r#"{ "task_id": "SC-1", "unexpected": "value" }"#,
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--unknown-var-mode")
        .arg("error")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ERR_VAL_EXTRA_INPUT"));
}

#[test]
fn validate_warns_when_defaults_and_input_defaults_both_exist() {
    let root = temp_root("input-defaults-alias-warning");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: old\ninput_defaults:\n  name: new\nrequired_variables:\n  - task_id\n---\nhello {{ task_id }} {{ name }}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("task_id=SC-1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("WARN_VAL_CONFLICTING_DEFAULT_SECTIONS"));
    assert!(stdout.contains("input_defaults"));
    assert!(stdout.contains("defaults"));
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
fn render_accepts_object_values_in_json_var_file() {
    let root = temp_root("object-json-var-file");
    let vars_file = root.join("vars.json");
    write_file(
        &root.join("template.md.j2"),
        "PR #{{ pr.number }} -> {{ pr.url }}\n",
    );
    write_file(
        &vars_file,
        r#"{ "pr": { "number": 43, "url": "https://example.test/pr/43" } }"#,
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "PR #43 -> https://example.test/pr/43"
    );
}

#[test]
fn render_accepts_object_values_in_yaml_var_file() {
    let root = temp_root("object-yaml-var-file");
    let vars_file = root.join("vars.yaml");
    write_file(
        &root.join("template.md.j2"),
        "PR #{{ pr.number }} -> {{ pr.url }}\n",
    );
    write_file(
        &vars_file,
        "pr:\n  number: 43\n  url: https://example.test/pr/43\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "PR #43 -> https://example.test/pr/43"
    );
}

#[test]
fn render_uses_env_prefix_inputs() {
    let root = temp_root("env-prefix");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--env-prefix")
        .arg("SC_")
        .env("SC_NAME", "env-world")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello env-world"
    );
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
    let value = parse_stdout_json(&output);
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
    let value = parse_stdout_json(&output);
    assert_eq!(value["payload"]["action"], "init");
    assert!(
        !value["payload"]["would_affect"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn render_dry_run_text_reports_would_change() {
    let root = temp_root("render-dry-run-text");
    let output_path = root.join("out.md");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );
    write_file(&output_path, "hello world");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&output_path)
        .arg("--dry-run")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("would_change: false"));
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

#[test]
fn render_reports_include_escape_for_path_confinement_violations() {
    let root = temp_root("render-include-escape-cli");
    let outside = root.parent().unwrap().join("outside-include.md");
    write_file(&outside, "outside\n");
    write_file(&root.join("template.md.j2"), "@<../outside-include.md>\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_INCLUDE_ESCAPE"));
}

#[test]
fn render_reports_include_escape_for_symlink_escape_at_cli_layer() {
    let root = temp_root("render-symlink-escape-cli");
    let outside = root.parent().unwrap().join("outside-symlink-include.md");
    write_file(&outside, "outside\n");
    let symlink_path = root.join("linked-outside.md");
    if !create_symlink_if_supported(&outside, &symlink_path) {
        return;
    }
    write_file(&root.join("template.md.j2"), "@<linked-outside.md>\n");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_INCLUDE_ESCAPE"));
}

#[cfg(windows)]
#[test]
fn windows_backslash_escape_requires_cli_confinement_coverage() {
    let root = temp_root("render-backslash-escape-cli");
    let outside = root.parent().unwrap().join("outside-backslash-include.md");
    write_file(&outside, "outside\n");
    write_file(
        &root.join("template.md.j2"),
        "@<..\\outside-backslash-include.md>\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_INCLUDE_ESCAPE"));
}

#[test]
fn render_smoke_pipeline_handles_includes_vars_var_file_env_and_output() {
    let root = temp_root("render-smoke");
    let output = root.join("out.md");
    let vars_file = root.join("vars.yaml");
    write_file(
        &root.join("template.md.j2"),
        concat!(
            "---\nrequired_variables:\n  - name\n  - title\n  - mood\n---\n",
            "@<partials/body.md>\n"
        ),
    );
    write_file(
        &root.join("partials").join("body.md"),
        "Name: {{ name }}\nTitle: {{ title }}\nMood: {{ mood }}\n",
    );
    write_file(&vars_file, "title: Engineer\n");

    let status = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=Casey")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--env-prefix")
        .arg("SC_")
        .arg("--output")
        .arg(&output)
        .env("SC_MOOD", "focused")
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "Name: Casey\nTitle: Engineer\nMood: focused"
    );
}

#[test]
fn observability_health_text_reports_process_local_status() {
    let root = temp_root("observability-health-text");
    let output = sc_compose()
        .arg("observability-health")
        .env("SC_LOG_ROOT", &root)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("state: Healthy"));
    assert!(stdout.contains("query_state: Healthy"));
    assert!(stdout.contains("sink jsonl-file: Healthy"));
    #[cfg(not(windows))]
    assert!(stdout.contains(&format!(
        "active_log_path: {}",
        root.join("logs").join("sc-compose.log.jsonl").display()
    )));
    #[cfg(windows)]
    assert!(stdout.contains("active_log_path:") && stdout.contains("sc-compose.log.jsonl"));
}

#[test]
fn release_smoke_covers_render_pipeline_and_observability_health() {
    let root = temp_root("release-smoke-observability");
    let logs_root = root.join("telemetry");
    let output = root.join("out.md");
    let vars_file = root.join("vars.yaml");
    write_file(
        &root.join("template.md.j2"),
        concat!(
            "---\nrequired_variables:\n  - name\n  - title\n  - mood\n---\n",
            "@<partials/body.md>\n"
        ),
    );
    write_file(
        &root.join("partials").join("body.md"),
        "Name: {{ name }}\nTitle: {{ title }}\nMood: {{ mood }}\n",
    );
    write_file(&vars_file, "title: Engineer\n");

    let render = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=Casey")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--env-prefix")
        .arg("SC_")
        .arg("--output")
        .arg(&output)
        .env("SC_MOOD", "focused")
        .env("SC_LOG_ROOT", &logs_root)
        .output()
        .unwrap();

    assert!(render.status.success());
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "Name: Casey\nTitle: Engineer\nMood: focused"
    );
    assert!(logs_root.join("logs").join("sc-compose.log.jsonl").exists());

    let health = sc_compose()
        .arg("observability-health")
        .arg("--json")
        .env("SC_LOG_ROOT", &logs_root)
        .output()
        .unwrap();

    assert!(health.status.success());
    let value = parse_stdout_json(&health);
    assert_eq!(value["payload"]["logging"]["state"], "Healthy");
    assert_eq!(
        value["payload"]["logging"]["active_log_path"],
        logs_root
            .join("logs")
            .join("sc-compose.log.jsonl")
            .display()
            .to_string()
    );
}

#[cfg(unix)]
fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
    use std::os::windows::fs::symlink_file;

    match symlink_file(target, link) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(_) => false,
    }
}
