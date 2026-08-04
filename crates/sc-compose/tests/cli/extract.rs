use crate::support::*;

#[cfg(unix)]
fn assert_not_signaled(status: std::process::ExitStatus) {
    use std::os::unix::process::ExitStatusExt as _;
    assert_eq!(
        status.signal(),
        None,
        "sc-compose extract must return a controlled error instead of a signal"
    );
}

fn fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.xml.j2")),
        root.join(format!("{name}.xml")),
    )
}

fn yaml_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.yaml.j2")),
        root.join(format!("{name}.yaml")),
    )
}

fn json_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.json.j2")),
        root.join(format!("{name}.json")),
    )
}

fn toml_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.toml.j2")),
        root.join(format!("{name}.toml")),
    )
}

fn raw_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.raw.j2")),
        root.join(format!("{name}.raw")),
    )
}

#[test]
fn extract_text_reports_inputs_values_provenance_and_confidence() {
    let (template, rendered) = fixture("attributes");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("template:"));
    assert!(stdout.contains("rendered:"));
    assert!(stdout.contains("format: xml"));
    assert!(stdout.contains("confidence:"));
    assert!(stdout.contains("id: \"42\""));
    assert!(stdout.contains("name: \"Ada\""));
    assert!(stdout.contains("occurrences:"));
    assert!(stdout.contains("attribute"));
    assert!(!stdout.contains("<doc id=\"42\">"));
}

#[test]
fn extract_text_applies_repeatable_include_and_exclude_filters() {
    let (template, rendered) = fixture("attributes");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .arg("--include")
        .arg("name")
        .arg("--exclude")
        .arg("id")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("name: \"Ada\""));
    assert!(!stdout.contains("id: \"42\""));
}

#[test]
fn extract_text_reports_empty_values_and_missing_occurrences_as_warnings() {
    let (template, rendered) = fixture("entities-whitespace-empty");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("empty: \"\""));

    let (template, rendered) = fixture("missing-occurrence");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("WARN_EXTRACT_NOT_OBSERVED"));
}

#[test]
fn extract_text_maps_failures_to_usage_exit_and_actionable_stderr() {
    let (template, rendered) = fixture("malformed");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERR_EXTRACT_MALFORMED"));
    assert!(stderr.contains("recovery:"));

    let (template, rendered) = fixture("unsupported-filter");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_EXTRACT_TEMPLATE_UNSUPPORTED"));

    let (template, rendered) = fixture("same-variable-conflicting-occurrences");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "conflicting occurrences are a report diagnostic"
    );

    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--include")
        .arg("bad/name")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_EXTRACT_INVALID_REQUEST"));

    let (ambiguous_template, ambiguous_rendered) = fixture("ambiguous-adjacent");
    let output = sc_compose()
        .arg("extract")
        .arg(ambiguous_template)
        .arg(ambiguous_rendered)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_EXTRACT_AMBIGUOUS"));
}

#[test]
fn extract_text_reports_missing_input_paths() {
    let root = temp_root("extract-missing");
    let output = sc_compose()
        .arg("extract")
        .arg(root.join("missing-template.xml.j2"))
        .arg(root.join("missing-rendered.xml"))
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_CONFIG_READ"));
}

#[test]
fn extract_text_accepts_xml_declaration_comments_and_static_text_fixture() {
    let (template, rendered) = fixture("declaration-comments");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("value: \"Ada\""));
    assert!(stdout.contains("format: xml"));
}

#[test]
fn extract_text_xml_block_format_emits_canonical_content_source() {
    let (template, rendered) = fixture("xml-blocks");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format: xml"));
    assert!(stdout.contains("description: \"Fix the XML extractor"));
    assert!(stdout.contains("references: \"<issue number=\\\"193\\\">Gap 1"));
    assert!(stdout.contains("element_content"));
}

#[test]
fn extract_text_uses_committed_repeated_sibling_fixture() {
    let (template, rendered) = fixture("repeated-siblings");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("first: \"A\""));
    assert!(stdout.contains("second: \"B\""));
    assert!(stdout.contains("/root[0]/item[0]"));
    assert!(stdout.contains("/root[0]/item[1]"));
}

#[test]
fn extract_text_uses_committed_static_prefix_suffix_fixture() {
    let (template, rendered) = fixture("static-prefix-suffix");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("name: \"Ada\""));
}

#[test]
fn extract_text_supports_json_format_without_changing_xml_default() {
    let (template, rendered) = json_fixture("json-atm-payload");

    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format: json"));
    assert!(stdout.contains("action_name: \"execute the assigned task\""));
    assert!(stdout.contains(".actions[0].action"));
    assert!(stdout.contains("string_value"));
}

#[test]
fn extract_text_supports_yaml_format_and_skips_template_frontmatter() {
    let (template, rendered) = yaml_fixture("yaml-atm-config");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--format")
        .arg("yaml")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format: yaml"));
    assert!(stdout.contains("action_name: \"execute the assigned task\""));
    assert!(stdout.contains(".actions[0].action"));
    assert!(stdout.contains("string_scalar"));
}

#[test]
fn extract_text_supports_toml_format_and_array_of_table_paths() {
    let (template, rendered) = toml_fixture("toml-cargo-config");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--format")
        .arg("toml")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format: toml"));
    assert!(stdout.contains("package_name: \"example-app\""));
    assert!(stdout.contains(".bin[1].name"));
    assert!(stdout.contains("string_value"));
}

#[test]
fn extract_text_supports_raw_markdown_format_and_text_spans() {
    let (template, rendered) = raw_fixture("markdown");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--format")
        .arg("raw")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format: raw"));
    assert!(stdout.contains("title: \"Launch Plan\""));
    assert!(stdout.contains("text[2..13]@1:3"));
    assert!(stdout.contains("text_span"));
}

#[test]
#[cfg_attr(
    not(unix),
    ignore = "signal-based crash detection is implemented for unix targets only"
)]
fn extract_xml_deeply_nested_input_does_not_abort_the_process() {
    let root = temp_root("extract-xml-deep-nesting");
    let depth = 60_000;
    let mut template = String::from("<root>");
    let mut rendered = String::from("<root>");
    for _ in 0..depth {
        template.push_str("<a>");
        rendered.push_str("<a>");
    }
    template.push_str("{{ v }}");
    rendered.push('X');
    for _ in 0..depth {
        template.push_str("</a>");
        rendered.push_str("</a>");
    }
    template.push_str("</root>");
    rendered.push_str("</root>");

    let template_path = root.join("deep.xml.j2");
    let rendered_path = root.join("deep.xml");
    write_file(&template_path, &template);
    write_file(&rendered_path, &rendered);

    let output = sc_compose()
        .arg("extract")
        .arg(&template_path)
        .arg(&rendered_path)
        .arg("--format")
        .arg("xml")
        .output()
        .unwrap();

    #[cfg(unix)]
    assert_not_signaled(output.status);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_EXTRACT_INPUT_LIMIT"));
}
