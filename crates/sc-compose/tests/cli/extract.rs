use crate::support::*;

fn fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.xml.j2")),
        root.join(format!("{name}.xml")),
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
    assert_eq!(output.status.code(), Some(3));
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
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERR_EXTRACT_UNSUPPORTED"));

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

    let root = temp_root("extract-ambiguous");
    let ambiguous_template = root.join("ambiguous.xml.j2");
    let ambiguous_rendered = root.join("ambiguous.xml");
    write_file(&ambiguous_template, "<x>{{ first }}{{ second }}</x>\n");
    write_file(&ambiguous_rendered, "<x>AB</x>\n");
    let output = sc_compose()
        .arg("extract")
        .arg(ambiguous_template)
        .arg(ambiguous_rendered)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
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
