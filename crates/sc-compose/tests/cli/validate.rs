use crate::support::*;

#[test]
fn validate_default_output_remains_unchanged_without_lint() {
    let root = temp_root("validate-default-output");
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
        .arg("--var")
        .arg("name=world")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "valid (static_only)\n"
    );
}

#[test]
fn validate_check_render_reports_context_valid_without_emitting_body() {
    let root = temp_root("validate-check-render");
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
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "state: render_checked\n"
    );
    assert!(!root.join("payload.json").exists());
}

#[test]
fn validate_check_render_fails_closed_for_malformed_json() {
    let root = temp_root("validate-check-render-invalid");
    write_file(
        &root.join("payload.json.j2"),
        "{\"value\": \"{{ value }}\"}",
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
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("state: render_invalid"), "{stdout}");
    assert!(stdout.contains("ERR_RENDER_JSON_MALFORMED"), "{stdout}");
    assert!(!stdout.contains("hello"), "secret/context leaked: {stdout}");
}

#[test]
fn validate_lint_reports_filter_chain_location_and_recommendation() {
    let root = temp_root("validate-lint");
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
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("WARN_LINT_REDUNDANT_FILTER_CHAIN"));
    assert!(stdout.contains("template.md.j2:5:"), "{stdout}");
    assert!(stdout.contains("recommendation: use `yaml_safe` alone"));
}

#[test]
fn validate_help_documents_lint_flag() {
    let output = sc_compose().arg("validate").arg("--help").output().unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--lint"));
    assert!(stdout.contains("lint findings with source locations"));
}
