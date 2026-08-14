use crate::support::*;

#[test]
fn validate_lint_json_includes_structured_location_and_recommendation() {
    let root = temp_root("validate-lint-json");
    write_file(
        &root.join("template.md.j2"),
        "---\nrequired_variables:\n  - value\n---\nvalue: {{ value | frontmatter_safe | yaml_safe }}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--lint")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("template.md.j2")
        .arg("--var")
        .arg("value=hello")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["valid"], true);
    assert_eq!(
        value["diagnostics"][0]["code"],
        "WARN_LINT_REDUNDANT_FILTER_CHAIN"
    );
    assert_eq!(value["diagnostics"][0]["line"], 5);
    assert!(
        value["diagnostics"][0]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("template.md.j2"))
    );
    assert!(
        value["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("use `yaml_safe` alone"))
    );
}

#[test]
fn validate_lint_json_rejects_conflicting_included_mode() {
    let root = temp_root("validate-json-include-mode-conflict");
    write_file(
        &root.join("payload.json.j2"),
        "---\nrequired_variables:\n  - value\n---\n{\n  \"value\": {{ value }},\n@<fragment.json.j2>\n}\n",
    );
    write_file(
        &root.join("fragment.json.j2"),
        "---\njson_escape_mode: legacy\n---\n\"fragment\": \"static\"\n",
    );

    let output = sc_compose()
        .args([
            "validate",
            "--lint",
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
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let diagnostic = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "ERR_JSON_MODE_INCLUDE_CONFLICT")
        .expect("include mode conflict diagnostic");
    let message = diagnostic["message"].as_str().unwrap();
    assert!(message.contains("payload.json.j2"), "{message}");
    assert!(message.contains("fragment.json.j2"), "{message}");
    assert!(message.contains("auto"), "{message}");
    assert!(message.contains("legacy"), "{message}");
}

#[test]
fn validate_json_check_render_rejects_conflicting_included_mode_without_body() {
    let root = temp_root("validate-json-check-render-include-mode-conflict");
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
            "validate",
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

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["state"], "contract_invalid");
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
fn validate_lint_json_allows_matching_or_undeclared_include_mode() {
    let root = temp_root("validate-json-include-mode-compatible");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: auto\n---\n{\n  \"value\": {{ value }},\n@<fragment.json.j2>\n}\n",
    );
    write_file(
        &root.join("fragment.json.j2"),
        "---\njson_escape_mode: auto\n---\n\"fragment\": \"static\"\n",
    );

    let output = sc_compose()
        .args([
            "validate",
            "--lint",
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
    assert!(
        !value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "ERR_JSON_MODE_INCLUDE_CONFLICT")
    );

    write_file(&root.join("fragment.json.j2"), "\"fragment\": \"static\"\n");
    let output = sc_compose()
        .args([
            "validate",
            "--lint",
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
    assert!(
        !value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "ERR_JSON_MODE_INCLUDE_CONFLICT")
    );
}

#[test]
fn validate_json_reports_legacy_json_migration_warning() {
    let root = temp_root("validate-json-legacy-warning");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: legacy\n---\n{\"value\": \"{{ value }}\"}",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var")
        .arg("value=hello")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["valid"], true);
    let diagnostics = value["diagnostics"].as_array().unwrap();
    let warning = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE")
        .expect("JSON migration warning");
    assert_eq!(
        warning["message"],
        "Template uses legacy JSON escape mode. Migrate to bare placeholders (auto mode) to avoid double-quoting issues. See docs/migration/json-escape-mode.md"
    );
}

#[test]
fn validate_json_reports_one_legacy_warning_for_multiple_quoted_placeholders() {
    let root = temp_root("validate-json-legacy-warning-once");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: legacy\n---\n{\"first\": \"{{ first }}\", \"second\": \"{{ second }}\"}",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var")
        .arg("first=one")
        .arg("--var")
        .arg("second=two")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let value = parse_stdout(&output);
    assert_envelope(&value);
    let warning_count = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE")
        .count();
    assert_eq!(warning_count, 1);
}

#[test]
fn validate_json_reports_legacy_warning_for_dotted_and_filtered_placeholders() {
    let root = temp_root("validate-json-legacy-expression-warning");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: legacy\n---\n{\"name\": \"{{ user.name | default('unknown') }}\"}",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE")
    );
}

#[test]
fn validate_lint_json_reports_the_same_legacy_json_warning() {
    let root = temp_root("validate-lint-json-legacy-warning");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: legacy\n---\n{\"value\": \"{{ value }}\"}",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--lint")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var")
        .arg("value=hello")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| {
                diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE"
                    && diagnostic["severity"] == "warning"
            })
    );
}

#[test]
fn validate_lint_json_cli_auto_override_supersedes_legacy_frontmatter() {
    let root = temp_root("validate-lint-json-cli-auto-overrides-legacy");
    write_file(
        &root.join("payload.json.j2"),
        "---\njson_escape_mode: legacy\n---\n{\"value\": \"{{ value }}\"}",
    );

    let output = sc_compose()
        .args([
            "validate",
            "--lint",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--var",
            "value=hello",
            "--json-escape-mode",
            "auto",
            "--json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "stderr: {:?}", output.stderr);
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| {
                diagnostic["code"] == "ERR_JSON_MODE_CONTRACT" && diagnostic["severity"] == "error"
            })
    );
    assert!(
        !value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE")
    );
}

#[test]
fn validate_lint_json_reports_ambiguous_quoted_placeholder_conservatively() {
    let root = temp_root("validate-lint-json-ambiguous-quoted-placeholder");
    write_file(
        &root.join("payload.json.j2"),
        r#"{"value": "{{ value.foo["key"] }}"}"#,
    );

    let output = sc_compose()
        .args([
            "validate",
            "--lint",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| {
                diagnostic["code"] == "WARN_JSON_QUOTED_PLACEHOLDER"
                    && diagnostic["severity"] == "warning"
            })
    );
}

#[test]
fn validate_json_rejects_non_string_legacy_value_in_quoted_slot() {
    let root = temp_root("validate-json-legacy-non-string");
    let vars_file = root.join("vars.json");
    write_file(&root.join("payload.json.j2"), r#"{"value": "{{ value }}"}"#);
    write_file(&vars_file, r#"{"value": 3}"#);

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.json.j2")
        .arg("--var-file")
        .arg(&vars_file)
        .arg("--json-escape-mode")
        .arg("legacy")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["valid"], false);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| { diagnostic["code"] == "ERR_JSON_LEGACY_NON_STRING" })
    );
}

#[test]
fn validate_json_rejects_json_escape_mode_on_non_json_templates() {
    let root = temp_root("validate-json-mode-non-json");
    write_file(
        &root.join("payload.md.j2"),
        "---\njson_escape_mode: legacy\n---\nhello {{ value }}\n",
    );

    let output = sc_compose()
        .arg("validate")
        .arg("--mode")
        .arg("file")
        .arg("--root")
        .arg(&root)
        .arg("--file")
        .arg("payload.md.j2")
        .arg("--var")
        .arg("value=hello")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["valid"], false);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "ERR_JSON_ESCAPE_MODE_NON_JSON")
    );
}

#[test]
fn validate_json_reports_static_only_state_without_rendering() {
    let root = temp_root("validate-json-static-only");
    write_file(&root.join("payload.json.j2"), "{\"value\": {{ value }}}");

    let output = sc_compose()
        .args([
            "validate",
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
    assert_eq!(value["payload"]["state"], "static_only");
    assert_eq!(value["payload"]["valid"], true);
    assert!(value["payload"].get("checked_context").is_none());
}

#[test]
fn validate_json_check_render_reports_exact_context_and_no_body() {
    let root = temp_root("validate-json-check-render");
    write_file(&root.join("payload.json.j2"), "{\"value\": {{ value }}}");

    let output = sc_compose()
        .args([
            "validate",
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
    assert_eq!(value["payload"]["state"], "render_checked");
    assert_eq!(value["payload"]["output_format"], "json");
    assert_eq!(value["payload"]["json_escape_mode"], "auto");
    assert!(value["payload"]["checked_context"].is_string());
    assert!(!value["payload"].to_string().contains("hello"));
}

#[test]
fn validate_json_check_render_attributes_template_defaults_and_caller_overrides() {
    let root = temp_root("validate-json-check-render-context-sources");
    write_file(
        &root.join("payload.json.j2"),
        "---\ndefaults:\n  fallback: from-template\n  overridden: from-template\n---\n{\"fallback\": {{ fallback }}, \"overridden\": {{ overridden }}}\n",
    );

    let output = sc_compose()
        .args([
            "validate",
            "--check-render",
            "--mode",
            "file",
            "--root",
            root.to_str().unwrap(),
            "--file",
            "payload.json.j2",
            "--var",
            "overridden=from-caller",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["state"], "render_checked");
    let checked_context = value["payload"]["checked_context"].as_str().unwrap();
    assert!(checked_context.contains("1 explicit caller values (overridden)"));
    assert!(checked_context.contains("1 root frontmatter defaults (fallback)"));
    assert!(checked_context.contains("0 template-pack defaults"));
    assert!(!checked_context.contains("overridden, fallback"));

    let diagnostics = value["diagnostics"].as_array().unwrap();
    let fallback_default = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "INFO_VAL_DEFAULT_USED")
        .expect("template default diagnostic");
    assert!(
        fallback_default["message"]
            .as_str()
            .is_some_and(|message| message.contains("fallback"))
    );
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == "INFO_VAL_DEFAULT_USED"
            && diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("overridden"))
    }));
}

#[test]
fn validate_json_lint_check_render_combines_diagnostics() {
    let root = temp_root("validate-json-lint-check-render");
    write_file(
        &root.join("payload.json.j2"),
        "{\"value\": {{ value | frontmatter_safe | yaml_safe }}}",
    );

    let output = sc_compose()
        .args([
            "validate",
            "--lint",
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
    assert_eq!(value["payload"]["state"], "render_checked");
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "WARN_LINT_REDUNDANT_FILTER_CHAIN")
    );
}
