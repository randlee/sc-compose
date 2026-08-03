use sc_composer::{
    ExtractError, ExtractFormat, ExtractRequest, ExtractionDiagnosticKind, ExtractionPathSegment,
    ExtractionSource, JsonPathSegment, TomlPathSegment, VariableName, XmlPathSegment, extract,
};

fn variable(name: &str) -> VariableName {
    VariableName::new(name).unwrap()
}

const TEAM_LEAD_SENDER_TOKEN: &str = "__TEAM_LEAD_SENDER__";
const TEAM_LEAD_SENDER_FILE: &str = include_str!("fixtures/reverse-extract/team-lead-sender.txt");

fn team_lead_sender() -> &'static str {
    TEAM_LEAD_SENDER_FILE.trim()
}

fn fixture_with_team_lead_sender(source: &str) -> String {
    source.replace(TEAM_LEAD_SENDER_TOKEN, team_lead_sender())
}

fn request(template: &'static str, rendered: &'static str) -> ExtractRequest<'static> {
    ExtractRequest::new(template, rendered, ExtractFormat::Xml, &[], &[])
}

fn json_request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Json, &[], &[])
}

fn toml_request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Toml, &[], &[])
}

#[test]
fn fixture_extracts_attribute_and_text_values() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/attributes.xml.j2"),
        include_str!("fixtures/reverse-extract/attributes.xml"),
    ))
    .unwrap();

    assert_eq!(report.values[&variable("id")], "42");
    assert_eq!(report.values[&variable("name")], "Ada");
}

#[test]
fn fixture_preserves_repeated_sibling_occurrence_paths() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/repeated-siblings.xml.j2"),
        include_str!("fixtures/reverse-extract/repeated-siblings.xml"),
    ))
    .unwrap();

    assert_eq!(report.values[&variable("first")], "A");
    assert_eq!(report.values[&variable("second")], "B");
    assert_eq!(
        report.occurrences[1].path[1],
        ExtractionPathSegment::Xml(XmlPathSegment::Element {
            name: "item".to_owned(),
            ordinal: 1,
        })
    );
}

#[test]
fn fixture_decodes_entities_and_keeps_empty_scalar() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/entities-whitespace-empty.xml.j2"),
        include_str!("fixtures/reverse-extract/entities-whitespace-empty.xml"),
    ))
    .unwrap();

    assert_eq!(report.values[&variable("value")], "A & B");
    assert_eq!(report.values[&variable("empty")], "");
}

#[test]
fn fixture_accepts_xml_declaration_comments_and_static_text() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/declaration-comments.xml.j2"),
        include_str!("fixtures/reverse-extract/declaration-comments.xml"),
    ))
    .unwrap();

    assert_eq!(report.values[&variable("value")], "Ada");
    assert_eq!(report.diagnostics, Vec::new());
}

#[test]
fn fixture_extracts_static_prefix_and_suffix() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/static-prefix-suffix.xml.j2"),
        include_str!("fixtures/reverse-extract/static-prefix-suffix.xml"),
    ))
    .unwrap();

    assert_eq!(report.values[&variable("name")], "Ada");
}

#[test]
fn fixture_adjacent_variables_fail_closed_as_ambiguous() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/ambiguous-adjacent.xml.j2"),
        include_str!("fixtures/reverse-extract/ambiguous-adjacent.xml"),
    ))
    .unwrap_err();

    assert!(matches!(error, ExtractError::AmbiguousStructure { .. }));
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractAmbiguous
    );
}

#[test]
fn fixture_conflicting_occurrences_are_reported_without_a_value() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/same-variable-conflicting-occurrences.xml.j2"),
        include_str!("fixtures/reverse-extract/same-variable-conflicting-occurrences.xml"),
    ))
    .unwrap();

    assert!(!report.values.contains_key(&variable("name")));
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.kind == ExtractionDiagnosticKind::Ambiguous })
    );
}

#[test]
fn fixture_malformed_xml_returns_stable_error() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/malformed.xml.j2"),
        include_str!("fixtures/reverse-extract/malformed.xml"),
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractMalformed
    );
    assert!(matches!(error, ExtractError::MalformedXml { .. }));
}

#[test]
fn fixture_unsupported_filter_returns_stable_error() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/unsupported-filter.xml.j2"),
        include_str!("fixtures/reverse-extract/unsupported-filter.xml"),
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractUnsupported
    );
}

#[test]
fn fixture_missing_occurrence_returns_not_observed_warning() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/missing-occurrence.xml.j2"),
        include_str!("fixtures/reverse-extract/missing-occurrence.xml"),
    ))
    .unwrap();

    assert!(report.values.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == sc_composer::DiagnosticCode::WarnExtractNotObserved
    }));
}

#[test]
fn fixture_wrong_structure_returns_unsupported_error() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/wrong-structure.xml.j2"),
        include_str!("fixtures/reverse-extract/wrong-structure.xml"),
    ))
    .unwrap_err();

    assert!(matches!(error, ExtractError::UnsupportedSyntax { .. }));
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractUnsupported
    );
    assert!(error.recovery_hints().iter().any(|hint| {
        matches!(
            &hint.kind,
            sc_composer::RecoveryHintKind::UnsupportedConstruct { .. }
        )
    }));
}

#[test]
fn fixture_wrong_tag_structure_returns_unsupported_error() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/wrong-tag-structure.xml.j2"),
        include_str!("fixtures/reverse-extract/wrong-tag-structure.xml"),
    ))
    .unwrap_err();

    assert!(matches!(error, ExtractError::UnsupportedSyntax { .. }));
    assert!(
        error
            .to_string()
            .contains("does not match template structure")
    );
}

#[test]
fn fixture_wrong_child_structure_returns_unsupported_error() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/wrong-child-structure.xml.j2"),
        include_str!("fixtures/reverse-extract/wrong-child-structure.xml"),
    ))
    .unwrap_err();

    assert!(matches!(error, ExtractError::UnsupportedSyntax { .. }));
    assert!(error.to_string().contains("child structure does not match"));
}

#[test]
fn fixture_wrong_child_kind_structure_returns_unsupported_error() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/wrong-child-kind-structure.xml.j2"),
        include_str!("fixtures/reverse-extract/wrong-child-kind-structure.xml"),
    ))
    .unwrap_err();

    assert!(matches!(error, ExtractError::UnsupportedSyntax { .. }));
    assert!(error.to_string().contains("node structure does not match"));
}

#[test]
fn fixture_namespace_policy_fails_closed() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/namespace.xml.j2"),
        include_str!("fixtures/reverse-extract/namespace.xml"),
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractUnsupported
    );
}

#[test]
fn fixture_xml_reports_match_frozen_h2_baseline() {
    let baseline: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/reverse-extract/xml-regression-baseline.json"
    ))
    .unwrap();
    let cases = [
        (
            "attributes",
            include_str!("fixtures/reverse-extract/attributes.xml.j2"),
            include_str!("fixtures/reverse-extract/attributes.xml"),
        ),
        (
            "repeated-siblings",
            include_str!("fixtures/reverse-extract/repeated-siblings.xml.j2"),
            include_str!("fixtures/reverse-extract/repeated-siblings.xml"),
        ),
        (
            "entities-whitespace-empty",
            include_str!("fixtures/reverse-extract/entities-whitespace-empty.xml.j2"),
            include_str!("fixtures/reverse-extract/entities-whitespace-empty.xml"),
        ),
        (
            "declaration-comments",
            include_str!("fixtures/reverse-extract/declaration-comments.xml.j2"),
            include_str!("fixtures/reverse-extract/declaration-comments.xml"),
        ),
        (
            "static-prefix-suffix",
            include_str!("fixtures/reverse-extract/static-prefix-suffix.xml.j2"),
            include_str!("fixtures/reverse-extract/static-prefix-suffix.xml"),
        ),
        (
            "same-variable-conflicting-occurrences",
            include_str!("fixtures/reverse-extract/same-variable-conflicting-occurrences.xml.j2"),
            include_str!("fixtures/reverse-extract/same-variable-conflicting-occurrences.xml"),
        ),
        (
            "missing-occurrence",
            include_str!("fixtures/reverse-extract/missing-occurrence.xml.j2"),
            include_str!("fixtures/reverse-extract/missing-occurrence.xml"),
        ),
        (
            "empty-scalar",
            "<root><value> {{ value }} </value></root>",
            "<root><value/></root>",
        ),
    ];
    for (id, template, rendered) in cases {
        let report = extract(&request(template, rendered)).unwrap();
        assert_eq!(serde_json::to_value(report).unwrap(), baseline[id]);
    }
}

#[test]
fn json_extracts_string_values_with_object_and_array_paths() {
    let report = extract(&json_request(
        r#"{"name":"{{ name }}","items":[{"id":"{{ first }}"},{"id":"{{ second }}"}],"enabled":true,"count":3}"#,
        r#"{"name":"Ada","items":[{"id":"A"},{"id":"B"}],"enabled":true,"count":3}"#,
    ))
    .unwrap();

    assert_eq!(report.values[&variable("name")], "Ada");
    assert_eq!(report.values[&variable("first")], "A");
    assert_eq!(report.values[&variable("second")], "B");
    assert!(report.occurrences.iter().any(|occurrence| {
        occurrence.variable == variable("first")
            && occurrence.path
                == vec![
                    ExtractionPathSegment::Json(JsonPathSegment::ObjectKey {
                        key: "items".to_owned(),
                    }),
                    ExtractionPathSegment::Json(JsonPathSegment::ArrayIndex { index: 0 }),
                    ExtractionPathSegment::Json(JsonPathSegment::ObjectKey {
                        key: "id".to_owned(),
                    }),
                ]
            && occurrence.source
                == ExtractionSource::Json(sc_composer::JsonExtractionSource::StringValue)
    }));
}

#[test]
fn json_fixture_extracts_realistic_atm_payload_values_and_paths() {
    let rendered = fixture_with_team_lead_sender(include_str!(
        "fixtures/reverse-extract/json-atm-payload.json"
    ));
    let report = extract(&json_request(
        include_str!("fixtures/reverse-extract/json-atm-payload.json.j2"),
        &rendered,
    ))
    .unwrap();

    assert_eq!(
        report.values[&variable("message_id")],
        "01KZ2BV5Z6VCRQYDQWYSAZA8GB"
    );
    assert_eq!(report.values[&variable("sender")], team_lead_sender());
    assert_eq!(
        report.values[&variable("action_name")],
        "execute the assigned task"
    );
    assert_eq!(
        report.values[&variable("description")],
        "H.2 JSON extraction core"
    );
    assert!(report.occurrences.iter().any(|occurrence| {
        occurrence.variable == variable("action_name")
            && occurrence.path
                == vec![
                    ExtractionPathSegment::Json(JsonPathSegment::ObjectKey {
                        key: "actions".to_owned(),
                    }),
                    ExtractionPathSegment::Json(JsonPathSegment::ArrayIndex { index: 0 }),
                    ExtractionPathSegment::Json(JsonPathSegment::ObjectKey {
                        key: "action".to_owned(),
                    }),
                ]
    }));
}

#[test]
fn json_repeated_variable_is_ambiguous_without_overwriting_values() {
    let report = extract(&json_request(
        r#"{"first":"{{ name }}","second":"{{ name }}"}"#,
        r#"{"first":"Ada","second":"Grace"}"#,
    ))
    .unwrap();

    assert!(report.values.is_empty());
    assert_eq!(report.occurrences.len(), 2);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == sc_composer::DiagnosticCode::ErrExtractJsonAmbiguous
            && diagnostic.kind == ExtractionDiagnosticKind::Ambiguous
    }));
}

#[test]
fn json_rejects_malformed_duplicate_missing_and_unsupported_boundaries() {
    let malformed = extract(&json_request(
        r#"{"name":"{{ name }}"}"#,
        r#"{"name":"Ada""#,
    ))
    .unwrap_err();
    assert_eq!(
        malformed.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonMalformed
    );

    let duplicate = extract(&json_request(
        r#"{"name":"{{ name }}"}"#,
        r#"{"name":"Ada","name":"Grace"}"#,
    ))
    .unwrap_err();
    assert_eq!(
        duplicate.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonDuplicateKey
    );

    let missing = extract(&json_request(
        r#"{"name":"{{ name }}"}"#,
        r#"{"other":"Ada"}"#,
    ))
    .unwrap_err();
    assert_eq!(
        missing.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonPathMissing
    );

    let key = extract(&json_request(
        r#"{"{{ key }}":"value"}"#,
        r#"{"name":"value"}"#,
    ))
    .unwrap_err();
    assert_eq!(
        key.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonValueUnsupported
    );

    let typed = extract(&json_request(r#"{"count":{{ count }}}"#, r#"{"count":42}"#)).unwrap_err();
    assert_eq!(
        typed.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonValueUnsupported
    );
}

#[test]
fn json_fixture_rejects_malformed_payload_with_stable_diagnostic() {
    let error = extract(&json_request(
        include_str!("fixtures/reverse-extract/json-malformed.json.j2"),
        include_str!("fixtures/reverse-extract/json-malformed.json"),
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonMalformed
    );
}

#[test]
fn json_preserves_empty_and_null_values_without_typed_recovery() {
    let report = extract(&json_request(
        r#"{"empty":"{{ empty }}","null":null,"static":42}"#,
        r#"{"empty":"","null":null,"static":42}"#,
    ))
    .unwrap();

    assert_eq!(report.values[&variable("empty")], "");
    assert_eq!(report.occurrences.len(), 1);
    assert!((report.confidence - 1.0).abs() < f64::EPSILON);
}

#[test]
fn json_rejects_array_shape_static_and_adjacent_variable_mismatches() {
    let array_shape = extract(&json_request(
        r#"{"items":["{{ first }}","{{ second }}"]}"#,
        r#"{"items":["A"]}"#,
    ))
    .unwrap_err();
    assert_eq!(
        array_shape.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonShapeMismatch
    );

    let static_mismatch = extract(&json_request(
        r#"{"kind":"user-{{ id }}"}"#,
        r#"{"kind":"admin-42"}"#,
    ))
    .unwrap_err();
    assert_eq!(
        static_mismatch.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonShapeMismatch
    );

    let adjacent = extract(&json_request(
        r#"{"value":"{{ first }}{{ second }}"}"#,
        r#"{"value":"AB"}"#,
    ))
    .unwrap_err();
    assert_eq!(
        adjacent.code(),
        sc_composer::DiagnosticCode::ErrExtractJsonAmbiguous
    );
}

#[test]
fn json_include_and_exclude_filters_apply_to_occurrences() {
    let include = [variable("name")];
    let exclude = [variable("ignored")];
    let request = ExtractRequest::new(
        r#"{"name":"{{ name }}","ignored":"{{ ignored }}"}"#,
        r#"{"name":"Ada","ignored":"secret"}"#,
        ExtractFormat::Json,
        &include,
        &exclude,
    );
    let report = extract(&request).unwrap();
    assert_eq!(report.values.len(), 1);
    assert_eq!(report.values[&variable("name")], "Ada");
    assert_eq!(report.occurrences.len(), 1);
}

#[test]
fn yaml_fixture_skips_template_frontmatter_and_extracts_atm_config() {
    let rendered = fixture_with_team_lead_sender(include_str!(
        "fixtures/reverse-extract/yaml-atm-config.yaml"
    ));
    let report = extract(&ExtractRequest::new(
        include_str!("fixtures/reverse-extract/yaml-atm-config.yaml.j2"),
        &rendered,
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap();

    assert_eq!(
        report.values[&variable("message_id")],
        "01KZ2H7JBMK3850VYC637AN4XJ"
    );
    assert_eq!(report.values[&variable("sender")], team_lead_sender());
    assert_eq!(
        report.values[&variable("action_name")],
        "execute the assigned task"
    );
    let action = report
        .occurrences
        .iter()
        .find(|occurrence| occurrence.variable == variable("action_name"))
        .unwrap();
    assert_eq!(
        action.path,
        vec![
            ExtractionPathSegment::Yaml(sc_composer::YamlPathSegment::MappingKey {
                key: "actions".to_owned(),
            }),
            ExtractionPathSegment::Yaml(sc_composer::YamlPathSegment::SequenceIndex { index: 0 }),
            ExtractionPathSegment::Yaml(sc_composer::YamlPathSegment::MappingKey {
                key: "action".to_owned(),
            }),
        ]
    );
    assert_eq!(
        action.source,
        ExtractionSource::Yaml(sc_composer::YamlExtractionSource::StringScalar)
    );
}

#[test]
fn yaml_fixture_rejects_malformed_and_duplicate_documents() {
    let malformed = extract(&ExtractRequest::new(
        include_str!("fixtures/reverse-extract/yaml-malformed.yaml.j2"),
        include_str!("fixtures/reverse-extract/yaml-malformed.yaml"),
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap_err();
    assert_eq!(
        malformed.code(),
        sc_composer::DiagnosticCode::ErrExtractYamlMalformed
    );

    let duplicate = extract(&ExtractRequest::new(
        include_str!("fixtures/reverse-extract/yaml-duplicate.yaml.j2"),
        include_str!("fixtures/reverse-extract/yaml-duplicate.yaml"),
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap_err();
    assert_eq!(
        duplicate.code(),
        sc_composer::DiagnosticCode::ErrExtractYamlDuplicateKey
    );

    let stream = extract(&ExtractRequest::new(
        "name: \"{{ name }}\"",
        "---\nname: Ada\n---\nname: Grace\n",
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap_err();
    assert_eq!(
        stream.code(),
        sc_composer::DiagnosticCode::ErrExtractYamlDocumentStream
    );

    let alias = extract(&ExtractRequest::new(
        "name: \"{{ name }}\"",
        "name: &base Ada\ncopy: *base\n",
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap_err();
    assert_eq!(
        alias.code(),
        sc_composer::DiagnosticCode::ErrExtractYamlAliasUnsupported
    );

    let typed = extract(&ExtractRequest::new(
        "count: \"{{ count }}\"",
        "count: 42\n",
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap_err();
    assert_eq!(
        typed.code(),
        sc_composer::DiagnosticCode::ErrExtractYamlValueUnsupported
    );

    let missing = extract(&ExtractRequest::new(
        "name: \"{{ name }}\"",
        "other: Ada\n",
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap_err();
    assert_eq!(
        missing.code(),
        sc_composer::DiagnosticCode::ErrExtractYamlPathMissing
    );

    let sequence_shape = extract(&ExtractRequest::new(
        "items:\n  - \"{{ first }}\"\n  - \"{{ second }}\"\n",
        "items:\n  - one\n",
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap_err();
    assert_eq!(
        sequence_shape.code(),
        sc_composer::DiagnosticCode::ErrExtractYamlShapeMismatch
    );

    let null = extract(&ExtractRequest::new(
        "value: null\n",
        "value: null\n",
        ExtractFormat::Yaml,
        &[],
        &[],
    ))
    .unwrap();
    assert!(null.values.is_empty());
}

#[test]
fn toml_fixture_extracts_tables_and_array_of_table_paths() {
    let rendered = fixture_with_team_lead_sender(include_str!(
        "fixtures/reverse-extract/toml-cargo-config.toml"
    ));
    let report = extract(&toml_request(
        include_str!("fixtures/reverse-extract/toml-cargo-config.toml.j2"),
        &rendered,
    ))
    .unwrap();

    assert_eq!(report.values[&variable("package_name")], "example-app");
    assert_eq!(report.values[&variable("sender")], team_lead_sender());
    assert_eq!(report.values[&variable("serde_version")], "1.0");
    assert_eq!(report.values[&variable("bin_name")], "example-app");
    assert_eq!(report.values[&variable("second_bin_name")], "example-tool");
    let second_bin = report
        .occurrences
        .iter()
        .find(|occurrence| occurrence.variable == variable("second_bin_name"))
        .unwrap();
    assert_eq!(
        second_bin.path,
        vec![
            ExtractionPathSegment::Toml(TomlPathSegment::TableKey {
                key: "bin".to_owned(),
            }),
            ExtractionPathSegment::Toml(TomlPathSegment::ArrayIndex { index: 1 }),
            ExtractionPathSegment::Toml(TomlPathSegment::TableKey {
                key: "name".to_owned(),
            }),
        ]
    );
    assert_eq!(
        second_bin.source,
        ExtractionSource::Toml(sc_composer::TomlExtractionSource::StringValue)
    );
}

#[test]
fn toml_fixture_rejects_malformed_duplicate_and_shape_boundaries() {
    let malformed = extract(&toml_request(
        include_str!("fixtures/reverse-extract/toml-malformed.toml.j2"),
        include_str!("fixtures/reverse-extract/toml-malformed.toml"),
    ))
    .unwrap_err();
    assert_eq!(
        malformed.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlMalformed
    );

    let duplicate = extract(&toml_request(
        include_str!("fixtures/reverse-extract/toml-duplicate.toml.j2"),
        include_str!("fixtures/reverse-extract/toml-duplicate.toml"),
    ))
    .unwrap_err();
    assert_eq!(
        duplicate.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlDuplicateKey
    );

    let typed = extract(&toml_request("count = \"{{ count }}\"\n", "count = 42\n")).unwrap_err();
    assert_eq!(
        typed.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlShapeMismatch
    );

    let typed_placeholder =
        extract(&toml_request("count = {{ count }}\n", "count = 42\n")).unwrap_err();
    assert_eq!(
        typed_placeholder.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlValueUnsupported
    );

    let dynamic_key = extract(&toml_request(
        "\"{{ key }}\" = \"Ada\"\n",
        "name = \"Ada\"\n",
    ))
    .unwrap_err();
    assert_eq!(
        dynamic_key.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlValueUnsupported
    );

    let missing = extract(&toml_request(
        "[package]\nname = \"{{ name }}\"\n",
        "[other]\nname = \"Ada\"\n",
    ))
    .unwrap_err();
    assert_eq!(
        missing.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlPathMissing
    );

    let array_shape = extract(&toml_request(
        "values = [\"{{ first }}\", \"{{ second }}\"]\n",
        "values = [\"one\"]\n",
    ))
    .unwrap_err();
    assert_eq!(
        array_shape.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlShapeMismatch
    );
}

#[test]
fn toml_extraction_rejects_oversized_and_deep_inputs() {
    let oversized = format!("value = \"{}\"\n", "x".repeat(1_048_577));
    let error = extract(&toml_request("value = \"{{ value }}\"\n", &oversized)).unwrap_err();
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractInputLimit
    );

    let path = (0..66)
        .map(|index| format!("level{index}"))
        .collect::<Vec<_>>()
        .join(".");
    let template = format!("{path} = \"{{{{ value }}}}\"\n");
    let rendered = format!("{path} = \"Ada\"\n");
    let error = extract(&toml_request(&template, &rendered)).unwrap_err();
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractInputLimit
    );
}
