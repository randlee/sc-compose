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
fn render_json_legacy_mode_supports_existing_quoted_placeholders() {
    let root = temp_root("render-json-legacy-mode");
    write_file(&root.join("payload.json.j2"), r#"{"value": "{{ value }}"}"#);

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var")
        .arg(r#"value=quote " slash \ newline"#)
        .arg("--json-escape-mode")
        .arg("legacy")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let rendered: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rendered["value"], r#"quote " slash \ newline"#);
}

#[test]
fn render_json_frontmatter_mode_is_used_when_cli_mode_is_absent() {
    let root = temp_root("render-json-frontmatter-mode");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: legacy\n---\n{\"value\": \"{{ value }}\"}",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var")
        .arg("value=hello")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let rendered: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rendered["value"], "hello");
}

#[test]
fn render_json_rejects_conflicting_included_mode_before_output() {
    let root = temp_root("render-json-include-mode-conflict");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: auto\n---\n{\n  \"value\": {{ value }},\n@<fragment.json.j2>\n}\n",
    );
    write_file(
        &root.join("fragment.json.j2"),
        "---\njson_escape_mode: legacy\n---\n\"fragment\": \"static\"\n",
    );

    let output = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--var",
            "value=hello",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "ERR_JSON_MODE_INCLUDE_CONFLICT")
    );
    assert!(!value.to_string().contains("hello"));
}

#[test]
fn render_json_cli_mode_overrides_frontmatter_mode() {
    let root = temp_root("render-json-cli-mode");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: legacy\n---\n{\"value\": {{ value }}}",
    );

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var")
        .arg("value=hello")
        .arg("--json-escape-mode")
        .arg("auto")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let rendered: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rendered["value"], "hello");
}

#[test]
fn render_json_auto_mode_preserves_types_for_bare_placeholders() {
    let root = temp_root("render-json-auto-mode");
    write_file(
        &root.join("payload.json.j2"),
        r#"{"count": {{ count }}, "enabled": {{ enabled }}}"#,
    );
    let vars_file = root.join("vars.json");
    write_file(&vars_file, r#"{"count": 3, "enabled": true}"#);

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let rendered: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rendered["count"], 3);
    assert_eq!(rendered["enabled"], true);
}

#[test]
fn render_json_auto_mode_rejects_quoted_placeholder_before_stdout_emission() {
    let root = temp_root("render-json-malformed-output");
    write_file(&root.join("payload.json.j2"), r#"{"value": "{{ value }}"}"#);

    let output = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--var",
            "value=hello",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "malformed body was emitted: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERR_RENDER_JSON_MALFORMED"), "{stderr}");
    assert!(!stderr.contains("hello"), "context leaked: {stderr}");
}

#[test]
fn render_json_multi_pass_failure_identifies_the_failing_pass() {
    let root = temp_root("render-json-multi-pass-malformed");
    write_file(
        &root.join("payload.2.json.j2"),
        "---\npass: 2\nrequired_variables:\n  - value\n---\n---\npass: 1\n---\n{\"value\": \"{{{ value }}}\"}",
    );

    let output = sc_compose()
        .args([
            "render",
            "--all",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.2.json.j2",
            "--pass",
            "2",
            "--var",
            "value=hello",
            "--pass",
            "1",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert_eq!(value["diagnostics"][0]["code"], "ERR_RENDER_JSON_MALFORMED");
    assert!(
        value["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("after render pass 2")
    );
    assert!(!value.to_string().contains("hello"), "context leaked");
}

#[test]
fn render_json_multi_pass_failure_identifies_an_early_pass_before_later_configured_passes() {
    let root = temp_root("render-json-early-multi-pass-malformed");
    write_file(
        &root.join("payload.2.json.j2"),
        "---\npass: 2\nrequired_variables:\n  - value\n---\n---\npass: 1\n---\n---\npass: 3\n---\n{\"value\": \"{{{ value }}}\"}",
    );

    let output = sc_compose()
        .args([
            "render",
            "--all",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.2.json.j2",
            "--pass",
            "2",
            "--var",
            "value=hello",
            "--pass",
            "1",
            "--pass",
            "3",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert_eq!(value["diagnostics"][0]["code"], "ERR_RENDER_JSON_MALFORMED");
    let message = value["diagnostics"][0]["message"].as_str().unwrap();
    assert!(message.contains("after render pass 2"), "{message}");
    assert!(!message.contains("after render pass 3"), "{message}");
    assert!(!value.to_string().contains("hello"), "context leaked");
}

#[test]
fn render_check_render_preserves_non_json_output_behavior() {
    let root = temp_root("render-text-check-render");
    write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

    let plain = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "template.md.j2",
            "--var",
            "name=world",
        ])
        .output()
        .unwrap();
    let checked = sc_compose()
        .args([
            "render",
            "--check-render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "template.md.j2",
            "--var",
            "name=world",
        ])
        .output()
        .unwrap();

    assert!(plain.status.success(), "{plain:?}");
    assert!(checked.status.success(), "{checked:?}");
    assert_eq!(plain.stdout, checked.stdout);
}

#[test]
fn render_json_parser_failure_preserves_diagnostic_envelope() {
    let root = temp_root("render-json-malformed-envelope");
    write_file(&root.join("payload.json.j2"), r#"{"value": "{{ value }}"}"#);

    let output = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--var",
            "value=hello",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert_eq!(value["diagnostics"][0]["code"], "ERR_RENDER_JSON_MALFORMED");
    assert!(!value.to_string().contains("hello"));
}

#[test]
fn render_json_file_parser_failure_does_not_create_output() {
    let root = temp_root("render-json-malformed-file");
    let output_path = root.join("payload.json");
    write_file(&root.join("payload.json.j2"), r#"{"value": "{{ value }}"}"#);

    let output = sc_compose()
        .args([
            "render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--var",
            "value=hello",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(!output_path.exists());
}

#[test]
fn render_json_check_render_reports_checked_contract() {
    let root = temp_root("render-json-check-render");
    write_file(&root.join("payload.json.j2"), r#"{"value": {{ value }}}"#);

    let output = sc_compose()
        .args([
            "render",
            "--check-render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--var",
            "value=hello",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["render_check"]["state"], "render_checked");
    assert_eq!(value["payload"]["render_check"]["output_format"], "json");
    assert_eq!(value["payload"]["render_check"]["json_escape_mode"], "auto");
    let body: Value = serde_json::from_str(value["payload"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["value"], "hello");
}

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
fn render_json_stdout_includes_body_matching_plain_render() {
    let root = temp_root("render-json-stdout-body");
    let template = "---\ndefaults:\n  name: café\n---\nhello {{ name }}\n";
    write_file(&root.join("template.md.j2"), template);

    let json_output = sc_compose()
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
    let plain_output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .output()
        .unwrap();

    assert!(
        json_output.status.success(),
        "stderr: {:?}",
        json_output.stderr
    );
    assert!(
        plain_output.status.success(),
        "stderr: {:?}",
        plain_output.stderr
    );
    assert!(json_output.stderr.is_empty());
    let value = parse_stdout(&json_output);
    assert_envelope(&value);
    let body = value["payload"]["body"].as_str().expect("JSON stdout body");
    // JSON reports the logical stdout target, which includes plain-mode's
    // trailing newline even though the JSON body itself does not.
    assert_eq!(body.len() as u64 + 1, value["payload"]["bytes_written"]);
    assert_eq!(
        body.as_bytes(),
        plain_output
            .stdout
            .strip_suffix(b"\n")
            .expect("plain render transport newline")
    );
}

#[test]
fn render_json_stdout_bytes_written_includes_trailing_newline() {
    let root = temp_root("render-json-stdout-bytes-written");
    write_file(&root.join("template.md.j2"), "hello {{ name }}");

    let output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=world")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let value = parse_stdout(&output);
    let body = value["payload"]["body"].as_str().unwrap();
    assert_eq!(value["payload"]["bytes_written"], body.len() as u64 + 1);
}

#[test]
fn render_json_stdout_bytes_match_plain_stdout_bytes() {
    let root = temp_root("render-json-stdout-byte-parity");
    write_file(&root.join("template.md.j2"), "hello {{ name }}");

    let plain_output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=world")
        .output()
        .unwrap();
    let json_output = sc_compose()
        .arg("render")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("name=world")
        .arg("--json")
        .output()
        .unwrap();

    assert!(
        plain_output.status.success(),
        "stderr: {:?}",
        plain_output.stderr
    );
    assert!(
        json_output.status.success(),
        "stderr: {:?}",
        json_output.stderr
    );
    let value = parse_stdout(&json_output);
    assert_eq!(
        value["payload"]["bytes_written"],
        plain_output.stdout.len() as u64
    );
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
    assert_eq!(value["payload"]["rendered_preview"], "hello world");
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
    assert!(value["payload"].get("body").is_none());
    assert_eq!(fs::read_to_string(output_path).unwrap(), "hello café");
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
    assert_eq!(
        fs::read_to_string(output_path).unwrap(),
        "api:one=read,write"
    );

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

#[test]
fn render_check_render_rejects_malformed_json_despite_doubled_template_suffix() {
    let root = temp_root("render-check-render-doubled-suffix");
    write_file(
        &root.join("payload.json.j2.j2"),
        "{\"value\": \"{{ value }}\"",
    );

    let output = sc_compose()
        .args([
            "render",
            "--check-render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2.j2",
            "--var",
            "value=hello",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert_first_code(&value, "ERR_RENDER_JSON_MALFORMED");
    assert!(!value.to_string().contains("hello"));
}

#[test]
fn render_check_render_rejects_malformed_json_for_uppercase_json_extension() {
    let root = temp_root("render-check-render-uppercase-extension");
    write_file(&root.join("payload.JSON.j2"), "{\"value\": \"{{ value }}\"");

    let output = sc_compose()
        .args([
            "render",
            "--check-render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.JSON.j2",
            "--var",
            "value=hello",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"], serde_json::json!({}));
    assert_first_code(&value, "ERR_RENDER_JSON_MALFORMED");
    assert!(!value.to_string().contains("hello"));
}
