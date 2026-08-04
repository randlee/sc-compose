use crate::support::*;

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

#[test]
fn extract_json_yaml_format_emits_paths_sources_and_clean_envelope() {
    let (template, rendered) = yaml_fixture("yaml-atm-config");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--format")
        .arg("yaml")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["format"], "yaml");
    assert_eq!(
        value["payload"]["values"]["action_name"],
        "execute the assigned task"
    );
    assert_eq!(
        value["payload"]["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .find(|occurrence| occurrence["variable"] == "action_name")
            .unwrap()["source"]["kind"],
        "string_scalar"
    );
}

#[test]
fn extract_json_toml_format_emits_paths_sources_and_clean_envelope() {
    let (template, rendered) = toml_fixture("toml-cargo-config");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--format")
        .arg("toml")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["format"], "toml");
    assert_eq!(
        value["payload"]["values"]["second_bin_name"],
        "example-tool"
    );
    let occurrence = value["payload"]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|occurrence| occurrence["variable"] == "second_bin_name")
        .unwrap();
    assert_eq!(occurrence["source"]["kind"], "string_value");
    assert_eq!(occurrence["path"][0]["kind"], "table_key");
    assert_eq!(occurrence["path"][1]["kind"], "array_index");
    assert_eq!(occurrence["path"][2]["kind"], "table_key");
}

#[test]
fn extract_json_raw_format_emits_text_spans_and_clean_envelope() {
    let (template, rendered) = raw_fixture("markdown");
    let output = sc_compose()
        .arg("extract")
        .arg(template)
        .arg(rendered)
        .arg("--format")
        .arg("raw")
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value = parse_stdout(&output);
    assert_envelope(&value);
    assert_eq!(value["payload"]["format"], "raw");
    assert_eq!(value["payload"]["values"]["title"], "Launch Plan");
    let occurrence = value["payload"]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|occurrence| occurrence["variable"] == "title")
        .unwrap();
    assert_eq!(occurrence["source"]["kind"], "text_span");
    assert_eq!(occurrence["path"][0]["byte_start"], 2);
    assert_eq!(occurrence["path"][0]["byte_end"], 13);
    assert_eq!(occurrence["path"][0]["line"], 1);
    assert_eq!(occurrence["path"][0]["column"], 3);
}

#[test]
fn extract_json_xml_block_format_emits_canonical_content_source() {
    let (template, rendered) = fixture("xml-blocks");
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
    assert_envelope(&value);
    assert_eq!(value["payload"]["format"], "xml");
    assert_eq!(
        value["payload"]["values"]["description"],
        "Fix the XML extractor in <code>sc-compose</code> and preserve &amp; review evidence."
    );
    let occurrence = value["payload"]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|occurrence| occurrence["variable"] == "references")
        .unwrap();
    assert_eq!(occurrence["source"]["kind"], "element_content");
    assert_eq!(
        occurrence["rendered_text"],
        value["payload"]["values"]["references"]
    );
}
