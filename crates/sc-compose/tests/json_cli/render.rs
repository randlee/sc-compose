//! Capability-oriented integration tests. Shared mechanics live in `tests/support`.
#![allow(unused_imports)]
use crate::support::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[test]
fn render_json_uses_diagnostic_envelope() {
    let root = temp_root("render-json");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["output_path"], "stdout");
}

#[test]
fn render_dry_run_json_uses_diagnostic_envelope() {
    let root = temp_root("render-dry-run-json");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
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
    assert!(value["payload"]["would_write"].is_string());
    assert_eq!(
        value["payload"]["template"],
        normalize_path_str(fs::canonicalize(root.join("template.md.j2")).unwrap())
    );
    assert_eq!(value["payload"]["would_change"], true);
}

#[test]
fn render_dry_run_json_injects_builtin_variables() {
    let root = temp_root("render-dry-run-builtins-json");
    write_file(
        &root.join("report.md.j2"),
        "{{ TEMPLATE_NAME }}|{{ HOSTNAME }}|{{ USERNAME }}|{{ RENDER_DATE }}|{{ RENDER_TIMESTAMP }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("report.md.j2")
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
    let preview = value["payload"]["rendered_preview"].as_str().unwrap();
    let parts = preview.trim().split('|').collect::<Vec<_>>();
    assert_eq!(parts[0], "report.md.j2");
    assert!(!parts[1].is_empty());
    assert!(!parts[2].is_empty());
    assert_eq!(parts[3].len(), 10);
    assert!(parts[4].contains('T'));
}

#[test]
fn render_dry_run_json_builtin_override_precedence_is_stable() {
    let root = temp_root("render-dry-run-builtins-priority-json");
    write_file(
        &root.join("report.md.j2"),
        "---\ndefaults:\n  TEMPLATE_NAME: default-template\n  HOSTNAME: default-host\n  USERNAME: default-user\n  RENDER_DATE: 1999-12-31\n  RENDER_TIMESTAMP: 1999-12-31T23:59:59Z\n---\n{{ TEMPLATE_NAME }}|{{ HOSTNAME }}|{{ USERNAME }}|{{ RENDER_DATE }}|{{ RENDER_TIMESTAMP }}\n",
    );

    let output = sc_compose()
        .env("SC_TEMPLATE_NAME", "env-template")
        .env("SC_HOSTNAME", "env-host")
        .env("SC_USERNAME", "env-user")
        .env("SC_RENDER_DATE", "2001-02-03")
        .env("SC_RENDER_TIMESTAMP", "2001-02-03T04:05:06Z")
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("report.md.j2")
        .arg("--env-prefix")
        .arg("SC_")
        .arg("--var")
        .arg("HOSTNAME=cli-host")
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
    let preview = value["payload"]["rendered_preview"].as_str().unwrap();
    let parts = preview.trim().split('|').collect::<Vec<_>>();
    assert_eq!(
        parts,
        vec![
            "env-template",
            "cli-host",
            "env-user",
            "2001-02-03",
            "2001-02-03T04:05:06Z",
        ]
    );
}

#[test]
fn render_dry_run_json_reports_no_change_when_output_matches() {
    let root = temp_root("render-dry-run-json-no-change");
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
    assert_eq!(value["payload"]["would_change"], false);
}

#[test]
fn resolve_json_uses_diagnostic_envelope() {
    let root = temp_root("resolve-json");
    write_file(
        &root.join(".claude").join("agents").join("example.md"),
        "agent",
    );

    let output = sc_compose()
        .arg("resolve")
        .arg("--mode")
        .arg("profile")
        .arg("--root")
        .arg(&root)
        .arg("--kind")
        .arg("agent")
        .arg("--agent")
        .arg("example")
        .arg("--runtime")
        .arg("claude")
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
    assert_eq!(value["payload"]["found"], true);
}

#[test]
fn validate_json_uses_diagnostic_envelope() {
    let root = temp_root("validate-json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["valid"], false);
    assert_eq!(value["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_first_code(&value, "ERR_VAL_MISSING_REQUIRED");
    assert_eq!(value["diagnostics"][0]["line"], 3);
    assert_eq!(value["diagnostics"][0]["column"], 5);
}

#[test]
fn validate_json_reports_missing_frontmatter_for_included_file() {
    let root = temp_root("validate-json-included-missing-frontmatter");
    write_file(
        &root.join("_includes").join("snippet.md"),
        "hello {{ name }}\n",
    );
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\n@<_includes/snippet.md>\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "ERR_VAL_MISSING_FRONTMATTER"
            && normalize_path_str(diagnostic["path"].as_str().unwrap_or(""))
                == normalize_path_str(
                    fs::canonicalize(root.join("_includes").join("snippet.md")).unwrap(),
                )
    }));
}

#[test]
fn stdin_double_read_reports_structured_error_code() {
    let root = temp_root("stdin-double-read");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--guidance-file")
        .arg("-")
        .arg("--prompt-file")
        .arg("-")
        .stdin(Stdio::piped())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ERR_RENDER_STDIN_DOUBLE_READ"));
}

#[test]
fn render_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("render-failure-json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_VAL_MISSING_REQUIRED");
}

#[test]
fn render_failure_json_preserves_all_validation_diagnostics() {
    let root = temp_root("render-failure-multi-json");
    write_file(
        &root.join("template.md.j2"),
        concat!(
            "---\nrequired_variables:\n  - first\n  - second\n---\n",
            "{{ first }} {{ second }}\n"
        ),
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0]["code"], "ERR_VAL_MISSING_REQUIRED");
    assert_eq!(diagnostics[1]["code"], "ERR_VAL_MISSING_REQUIRED");
}

#[test]
fn validate_json_reports_default_usage_info_for_frontmatter_input_defaults() {
    let root = temp_root("validate-default-usage-json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - task_id\ninput_defaults:\n  assignee: teammate\n---\nhello {{ task_id }} {{ assignee }}\n",
    );
    let vars_file = root.join("vars.json");
    write_file(&vars_file, r#"{ "task_id": "SC-1" }"#);

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
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "INFO_VAL_DEFAULT_USED"
            && diagnostic["message"]
                == r#"variable assignee not provided, using default: "teammate""#
    }));
}

#[test]
fn render_dry_run_json_reports_default_usage_info_for_template_json_defaults() {
    let root = temp_root("render-dry-run-template-default-json");
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
        .arg("--json")
        .env("SC_COMPOSE_TEMPLATE_DIR", &templates_root)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "INFO_VAL_DEFAULT_USED"
            && diagnostic["message"] == r#"variable name not provided, using default: "world""#
    }));
}

#[test]
fn render_json_reports_actual_bytes_written_for_output_file() {
    let root = temp_root("render-bytes-written-json");
    let output_path = root.join("out.txt");
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: café\n---\nhello {{ name }}\n",
    );

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
        value["payload"]["bytes_written"].as_u64().unwrap(),
        fs::metadata(&output_path).unwrap().len()
    );
}

#[test]
fn resolve_failure_json_uses_diagnostic_envelope() {
    let root = temp_root("resolve-failure-json");

    let output = sc_compose()
        .arg("resolve")
        .arg("--mode")
        .arg("profile")
        .arg("--root")
        .arg(&root)
        .arg("--kind")
        .arg("agent")
        .arg("--agent")
        .arg("missing")
        .arg("--runtime")
        .arg("claude")
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
    assert_first_code(&value, "ERR_RESOLVE_NOT_FOUND");
}

#[test]
fn render_write_failure_json_reports_render_write_code() {
    let root = temp_root("render-write-failure-json");
    let out_dir = root.join("out");
    fs::create_dir_all(&out_dir).unwrap();
    write_file(
        &root.join("template.md.j2"),
        "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--output")
        .arg(&out_dir)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "--json must not emit console log noise"
    );
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_first_code(&value, "ERR_RENDER_WRITE");
}

#[test]
fn invalid_var_file_json_reports_config_varfile() {
    let root = temp_root("var-file-invalid-json");
    let vars_file = root.join("vars.json");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");
    write_file(&vars_file, "[1, 2, 3]\n");

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
    assert_first_code(&value, "ERR_CONFIG_VARFILE");
}

#[test]
fn f4_json_cli_regression_covers_var_file_shapes() {
    let root = temp_root("f4-json-var-file-shapes");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - groups\n---\n{% for group in groups %}{% for item in group.items %}{{ group.name }}:{{ item.id }}={{ item.tags | join(',') }}\n{% endfor %}{% endfor %}",
    );

    let nested_vars = root.join("nested.json");
    write_file(
        &nested_vars,
        r#"{"groups":[{"name":"api","items":[{"id":"one","tags":["read","write"]}]}]}"#,
    );
    let output_path = root.join("nested.out");
    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var-file")
        .arg(&nested_vars)
        .arg("--output")
        .arg(&output_path)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(fs::read_to_string(output_path).unwrap(), "api:one=read,write");

    for (filename, contents, expected_code) in [
        (
            "duplicate.json",
            r#"{"config":{"name":"first","name":"second"}}"#,
            "ERR_CONFIG_PARSE",
        ),
        (
            "duplicate.yaml",
            "config:\n  name: first\n  name: second\n",
            "ERR_CONFIG_PARSE",
        ),
        (
            "nested-key.yaml",
            "items:\n  - metadata:\n      7: invalid\n",
            "ERR_VAL_OBJECT_SHAPE",
        ),
        (
            "top-level.json",
            "[\"not\", \"an object\"]\n",
            "ERR_CONFIG_VARFILE",
        ),
        (
            "malformed.json",
            "{ \"name\": \"unterminated\"\n",
            "ERR_CONFIG_PARSE",
        ),
    ] {
        let vars_file = root.join(filename);
        write_file(&vars_file, contents);
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
            .arg("--json")
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(3), "{filename}");
        assert!(output.stderr.is_empty(), "{filename}: {:?}", output.stderr);
        let value = parse_stdout(&output);
        assert_envelope(&value);
        assert_first_code(&value, expected_code);
    }
}

#[test]
fn resolve_mode_mismatch_json_reports_config_mode() {
    let root = temp_root("resolve-mode-mismatch-json");
    write_file(&root.join("template.md.j2"), "hello\n");

    let output = sc_compose()
        .arg("resolve")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
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
    assert_first_code(&value, "ERR_CONFIG_MODE");
}
