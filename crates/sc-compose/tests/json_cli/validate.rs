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
