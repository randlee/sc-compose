use crate::support::*;

fn fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.xml.j2")),
        root.join(format!("{name}.xml")),
    )
}

fn json_fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = repo_root().join("crates/sc-composer/tests/fixtures/reverse-extract");
    (
        root.join(format!("{name}.json.j2")),
        root.join(format!("{name}.json")),
    )
}

#[test]
fn extract_json_is_a_clean_envelope_with_values_and_provenance() {
    let (template, rendered) = fixture("attributes");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "JSON must remain stdout-clean");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["format"], "xml");
    assert_eq!(value["payload"]["values"]["id"], "42");
    assert_eq!(value["payload"]["values"]["name"], "Ada");
    assert!(value["payload"]["confidence"].is_f64());
    assert_eq!(
        value["payload"]["warnings"].as_array().map(Vec::len),
        Some(0)
    );
    assert_eq!(value["diagnostics"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        value["payload"]["occurrences"][0]["source"]["kind"],
        "attribute"
    );
}

#[test]
fn extract_json_preserves_filters_empty_values_and_warnings() {
    let (template, rendered) = fixture("entities-whitespace-empty");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .arg("--include")
        .arg("empty")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_eq!(value["payload"]["values"]["empty"], "");
    assert!(value["payload"]["values"].get("value").is_none());

    let (template, rendered) = fixture("missing-occurrence");
    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_eq!(
        value["payload"]["warnings"][0]["code"],
        "WARN_EXTRACT_NOT_OBSERVED"
    );
    assert_eq!(value["diagnostics"][0]["code"], "WARN_EXTRACT_NOT_OBSERVED");
}

#[test]
fn extract_json_maps_expected_failures_without_logs_or_backtraces() {
    let cases = [
        ("malformed", "ERR_EXTRACT_MALFORMED"),
        ("unsupported-filter", "ERR_EXTRACT_UNSUPPORTED"),
    ];
    for (name, code) in cases {
        let (template, rendered) = fixture(name);
        let output = sc_compose()
            .arg("extract")
            .arg(&template)
            .arg(&rendered)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(
            output.stderr.is_empty(),
            "expected failure must be JSON-clean"
        );
        let value = parse_stdout(&output);
        assert_envelope(&value);
        assert_first_code(&value, code);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("backtrace"));
    }

    let root = temp_root("extract-json-missing");
    let output = sc_compose()
        .arg("extract")
        .arg(root.join("missing-template.xml.j2"))
        .arg(root.join("missing-rendered.xml"))
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_first_code(&value, "ERR_CONFIG_READ");

    let (ambiguous_template, ambiguous_rendered) = fixture("ambiguous-adjacent");
    let output = sc_compose()
        .arg("extract")
        .arg(ambiguous_template)
        .arg(ambiguous_rendered)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_first_code(&value, "ERR_EXTRACT_AMBIGUOUS");
}

#[test]
fn extract_json_accepts_xml_declaration_comments_and_static_text_fixture() {
    let (template, rendered) = fixture("declaration-comments");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_eq!(value["payload"]["values"]["value"], "Ada");
    assert_eq!(
        value["payload"]["warnings"].as_array().map(Vec::len),
        Some(0)
    );
}

#[test]
fn extract_json_format_emits_json_paths_and_sources() {
    let (template, rendered) = json_fixture("json-atm-payload");

    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .arg("--format")
        .arg("json")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "JSON must remain stdout-clean");
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["format"], "json");
    assert_eq!(
        value["payload"]["values"]["action_name"],
        "execute the assigned task"
    );
    assert_eq!(
        value["payload"]["occurrences"][0]["source"]["kind"],
        "string_value"
    );
    let action_occurrence = value["payload"]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|occurrence| occurrence["variable"] == "action_name")
        .unwrap();
    assert_eq!(action_occurrence["path"][0]["kind"], "object_key");
    assert_eq!(action_occurrence["path"][1]["kind"], "array_index");
    assert_eq!(action_occurrence["path"][2]["kind"], "object_key");
}

#[test]
fn extract_json_format_maps_malformed_input_to_json_diagnostic() {
    let (template, rendered) = json_fixture("json-malformed");

    let output = sc_compose()
        .arg("extract")
        .arg(&template)
        .arg(&rendered)
        .arg("--format")
        .arg("json")
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "JSON failures must remain stdout-clean"
    );
    let value = parse_stdout(&output);
    assert_first_code(&value, "ERR_EXTRACT_JSON_MALFORMED");
}
