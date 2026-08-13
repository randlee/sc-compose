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
fn validate_json_reports_legacy_json_migration_warning() {
    let root = temp_root("validate-json-legacy-warning");
    write_file(&root.join("payload.json.j2"), r#"{"value": "{{ value }}"}"#);

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
fn validate_lint_json_reports_the_same_legacy_json_warning() {
    let root = temp_root("validate-lint-json-legacy-warning");
    write_file(&root.join("payload.json.j2"), r#"{"value": "{{ value }}"}"#);

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
            .any(|diagnostic| { diagnostic["code"] == "WARN_JSON_LEGACY_ESCAPE_MODE" })
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
