use std::fmt::Write as _;

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

fn request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Xml, &[], &[])
}

fn json_request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Json, &[], &[])
}

fn yaml_request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Yaml, &[], &[])
}

fn toml_request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Toml, &[], &[])
}

fn raw_request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Raw, &[], &[])
}

fn assert_input_limit(request: &ExtractRequest<'_>) {
    assert_eq!(
        extract(request).unwrap_err().code(),
        sc_composer::DiagnosticCode::ErrExtractInputLimit
    );
}

#[test]
fn raw_fixture_corpus_covers_separated_values_filters_and_excludes() {
    let report = extract(&raw_request(
        include_str!("fixtures/reverse-extract/markdown-separated.raw.j2"),
        include_str!("fixtures/reverse-extract/markdown-separated.raw"),
    ))
    .unwrap();
    assert_eq!(report.values[&variable("title")], "Launch Plan");
    assert_eq!(report.values[&variable("owner")], "Ada");
    assert_eq!(
        report
            .occurrences
            .iter()
            .map(|occurrence| occurrence.variable.to_string())
            .collect::<Vec<_>>(),
        vec!["title", "owner"]
    );

    let include = [variable("owner")];
    let included = extract(&ExtractRequest::new(
        include_str!("fixtures/reverse-extract/markdown-filters.raw.j2"),
        include_str!("fixtures/reverse-extract/markdown-filters.raw"),
        ExtractFormat::Raw,
        &include,
        &[],
    ))
    .unwrap();
    assert_eq!(included.values.len(), 1);
    assert_eq!(included.values[&variable("owner")], "Ada");

    let exclude = [variable("secret")];
    let excluded = extract(&ExtractRequest::new(
        include_str!("fixtures/reverse-extract/markdown-filters.raw.j2"),
        include_str!("fixtures/reverse-extract/markdown-filters.raw"),
        ExtractFormat::Raw,
        &[],
        &exclude,
    ))
    .unwrap();
    assert!(!excluded.values.contains_key(&variable("secret")));
    assert_eq!(excluded.occurrences.len(), 2);
}

#[test]
fn raw_fixture_corpus_preserves_rejection_codes() {
    let adjacent = extract(&raw_request(
        include_str!("fixtures/reverse-extract/markdown-adjacent.raw.j2"),
        include_str!("fixtures/reverse-extract/markdown-adjacent.raw"),
    ))
    .unwrap_err();
    assert_eq!(
        adjacent.code(),
        sc_composer::DiagnosticCode::ErrExtractAmbiguous
    );

    let delimiter = extract(&raw_request(
        include_str!("fixtures/reverse-extract/markdown-delimiter-count.raw.j2"),
        include_str!("fixtures/reverse-extract/markdown-delimiter-count.raw"),
    ))
    .unwrap_err();
    assert_eq!(
        delimiter.code(),
        sc_composer::DiagnosticCode::ErrExtractTemplateUnsupported
    );

    let static_mismatch = extract(&raw_request(
        include_str!("fixtures/reverse-extract/markdown-static-mismatch.raw.j2"),
        include_str!("fixtures/reverse-extract/markdown-static-mismatch.raw"),
    ))
    .unwrap_err();
    assert_eq!(
        static_mismatch.code(),
        sc_composer::DiagnosticCode::ErrExtractUnsupported
    );
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
fn fixture_normalizes_dirty_xml_prefix_and_reports_removed_span() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/xml-dirty-prefix.xml.j2"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix.xml"),
    ))
    .unwrap();

    assert_eq!(report.values[&variable("value")], "Ada");
    assert_eq!(
        report.occurrences[0].path[0],
        ExtractionPathSegment::Xml(XmlPathSegment::Element {
            name: "root".to_owned(),
            ordinal: 0,
        },)
    );
    let warning = report
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == sc_composer::DiagnosticCode::WarnExtractDirtyPrefixStripped
        })
        .expect("dirty-prefix recovery must be visible in the report");
    assert_eq!(warning.kind, ExtractionDiagnosticKind::NotObserved);
    assert!(warning.message.contains("bytes 0.."));
    assert!(warning.message.contains("line 2, column 1"));
}

#[test]
fn fixture_preserves_dirty_xml_prolog_and_i3_full_content() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-prolog.xml.j2"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-prolog.xml"),
    ))
    .unwrap();
    assert_eq!(report.values[&variable("value")], "Ada");
    assert!(
        report.diagnostics.iter().any(|diagnostic| diagnostic.code
            == sc_composer::DiagnosticCode::WarnExtractDirtyPrefixStripped)
    );

    let block_report = extract(&request(
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-blocks.xml.j2"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-blocks.xml"),
    ))
    .unwrap();
    assert_eq!(
        block_report.values[&variable("content")],
        "<code>Ada</code> and <message>accepted</message>"
    );
}

#[test]
fn dirty_xml_prefix_rejections_are_not_silently_dropped() {
    let template = include_str!("fixtures/reverse-extract/xml-dirty-prefix.xml.j2");
    for rendered in [
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-multiple-root.xml"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-malformed-suffix.xml"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-unterminated-comment.xml"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-unterminated-pi.xml"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-ambiguous.xml"),
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-post-root.xml"),
    ] {
        let error = extract(&request(template, rendered)).unwrap_err();
        assert_eq!(
            error.code(),
            sc_composer::DiagnosticCode::ErrExtractMalformed
        );
    }

    let dtd = extract(&request(
        template,
        include_str!("fixtures/reverse-extract/xml-dirty-prefix-doctype.xml"),
    ))
    .unwrap_err();
    assert_eq!(
        dtd.code(),
        sc_composer::DiagnosticCode::ErrExtractUnsupported
    );
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
fn fixture_extracts_xml_block_text_and_mixed_content_with_canonical_markup() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/xml-blocks.xml.j2"),
        include_str!("fixtures/reverse-extract/xml-blocks.xml"),
    ))
    .unwrap();

    assert_eq!(
        report.values[&variable("description")],
        "Fix the XML extractor in <code>sc-compose</code> and preserve &amp; review evidence."
    );
    assert_eq!(
        report.values[&variable("references")],
        "<issue number=\"193\">Gap 1</issue><link>https://github.com/randlee/sc-compose/issues/193</link>"
    );
    assert_eq!(
        report.values[&variable("workflow")],
        "\n    <step>Render</step>\n    <step priority=\"high\">Review <em>then merge</em></step>\n  "
    );
    assert!(report.occurrences.iter().all(|occurrence| {
        occurrence.source == ExtractionSource::Xml(sc_composer::XmlExtractionSource::ElementContent)
    }));
    assert_eq!(report.occurrences.len(), 3);
    assert!(report.diagnostics.is_empty());
}

#[test]
fn xml_block_multiple_placeholders_remain_unsupported() {
    let error = extract(&request(
        "<root><description>{{ first }} {{ second }}</description></root>",
        "<root><description><b>value</b></description></root>",
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlChildStructureMismatch
    );
    assert!(error.to_string().contains("node structure does not match"));
}

#[test]
fn xml_block_static_child_structure_mismatch_remains_unsupported() {
    let error = extract(&request(
        "<root><description>Review <b class=\"expected\">{{ value }}</b></description></root>",
        "<root><description>Review <b class=\"actual\">value</b></description></root>",
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlStaticMismatch
    );
    assert!(
        error
            .to_string()
            .contains("rendered static content does not match")
    );
}

#[test]
fn xml_block_dynamic_element_names_remain_unsupported() {
    let error = extract(&request(
        "<root><{{ name }}>{{ value }}</{{ name }}></root>",
        "<root><item>Ada</item></root>",
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlDynamicElementName
    );
    assert!(error.to_string().contains("dynamic XML element names"));
}

#[test]
fn xml_rejects_dynamic_element_names_after_xml_parsing() {
    let error = extract(&request(
        "<root><{name}>{{ value }}</{name}></root>",
        "<root><item>Ada</item></root>",
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlDynamicElementName
    );
    assert!(error.to_string().contains("dynamic XML element names"));
}

#[test]
fn xml_block_control_flow_is_rejected_before_matching_rendered_children() {
    let error = extract(&request(
        "<root><description>{% for item in items %}{{ item }}{% endfor %}</description></root>",
        "<root><description><item>Ada</item></description></root>",
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlControlFlowUnsupported
    );
    assert!(error.to_string().contains("Jinja statements"));
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
        sc_composer::DiagnosticCode::ErrExtractTemplateUnsupported
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

    assert!(matches!(error, ExtractError::FormatError { .. }));
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlAttributeMismatch
    );
    assert!(error.recovery_hints().iter().any(|hint| {
        matches!(
            &hint.kind,
            sc_composer::RecoveryHintKind::InspectInput { .. }
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

    assert!(matches!(error, ExtractError::FormatError { .. }));
    assert!(
        error
            .to_string()
            .contains("does not match template structure")
    );
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlElementMismatch
    );
}

#[test]
fn fixture_full_content_placeholder_accepts_additional_child_markup() {
    let report = extract(&request(
        include_str!("fixtures/reverse-extract/wrong-child-structure.xml.j2"),
        include_str!("fixtures/reverse-extract/wrong-child-structure.xml"),
    ))
    .unwrap();

    assert_eq!(report.values[&variable("value")], "Ada<extra/>");
    assert_eq!(
        report.occurrences[0].source,
        ExtractionSource::Xml(sc_composer::XmlExtractionSource::ElementContent)
    );
}

#[test]
fn fixture_wrong_child_kind_structure_returns_unsupported_error() {
    let error = extract(&request(
        include_str!("fixtures/reverse-extract/wrong-child-kind-structure.xml.j2"),
        include_str!("fixtures/reverse-extract/wrong-child-kind-structure.xml"),
    ))
    .unwrap_err();

    assert!(matches!(error, ExtractError::FormatError { .. }));
    assert!(error.to_string().contains("node structure does not match"));
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractXmlChildStructureMismatch
    );
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
        sc_composer::DiagnosticCode::ErrExtractXmlNamespaceUnsupported
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

    let nested_static_mismatch = extract(&json_request(
        r#"{"profile":{"name":"user-{{ id }}"}}"#,
        r#"{"profile":{"name":"admin-42"}}"#,
    ))
    .unwrap_err();
    assert!(
        nested_static_mismatch
            .to_string()
            .contains("$.profile.name")
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
fn yaml_flow_aliases_adjacent_to_delimiters_are_rejected() {
    for rendered in ["values: [*anchor]\n", "values: {*anchor: 1}\n"] {
        let error = extract(&yaml_request("values: [\"{{ value }}\"]\n", rendered)).unwrap_err();
        assert_eq!(
            error.code(),
            sc_composer::DiagnosticCode::ErrExtractYamlAliasUnsupported,
            "flow-style YAML features must be rejected at delimiters"
        );
    }
}

#[test]
fn yaml_non_specific_tags_are_rejected_by_the_feature_scanner() {
    for rendered in ["value: !\n", "value: ! \"malicious\"\n"] {
        let error = extract(&yaml_request("value: \"{{ value }}\"\n", rendered)).unwrap_err();
        assert_eq!(
            error.code(),
            sc_composer::DiagnosticCode::ErrExtractYamlAliasUnsupported
        );
    }
}

#[test]
fn raw_text_mismatch_diagnostics_include_nested_paths_for_all_formats() {
    let yaml = extract(&yaml_request(
        "profile:\n  name: \"user-{{ id }}\"\n",
        "profile:\n  name: \"admin-42\"\n",
    ))
    .unwrap_err();
    assert!(yaml.to_string().contains("$.profile.name"));

    let toml = extract(&toml_request(
        "[profile]\nname = \"user-{{ id }}\"\n",
        "[profile]\nname = \"admin-42\"\n",
    ))
    .unwrap_err();
    assert!(toml.to_string().contains("$.profile.name"));

    let xml = extract(&request(
        "<root><profile>user-{{ id }}</profile></root>",
        "<root><profile>admin-42</profile></root>",
    ))
    .unwrap_err();
    assert!(xml.to_string().contains("$.root[0].profile[0]"));
}

#[test]
fn raw_text_mismatch_diagnostics_preserve_utf8_candidate_byte_spans() {
    let json = extract(&json_request(
        r#"{"profile":{"name":"prefix-{{ id }}-suffix"}}"#,
        r#"{"profile":{"name":"prefix-☃"}}"#,
    ))
    .unwrap_err();
    assert!(json.to_string().contains("candidate bytes 7..10"));

    let yaml = extract(&yaml_request(
        "profile:\n  name: \"prefix-{{ id }}-suffix\"\n",
        "profile:\n  name: \"prefix-☃\"\n",
    ))
    .unwrap_err();
    assert!(yaml.to_string().contains("candidate bytes 7..10"));

    let xml = extract(&request(
        "<root><profile><name>prefix-{{ id }}-suffix</name></profile></root>",
        "<root><profile><name>prefix-☃</name></profile></root>",
    ))
    .unwrap_err();
    assert!(xml.to_string().contains("candidate bytes 7..10"));
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
fn toml_repeated_table_fixture_matches_each_element_and_preserves_paths() {
    let report = extract(&toml_request(
        include_str!("fixtures/reverse-extract/toml-repeated-tables.toml.j2"),
        include_str!("fixtures/reverse-extract/toml-repeated-tables.toml"),
    ))
    .unwrap();

    assert_eq!(
        report.values[&variable("first_target")],
        "aarch64-apple-darwin"
    );
    assert_eq!(
        report.values[&variable("second_target")],
        "x86_64-pc-windows-msvc"
    );
    assert_eq!(report.values[&variable("first_os")], "macos-14");
    assert_eq!(report.values[&variable("second_os")], "windows-latest");

    let first_target = report
        .occurrences
        .iter()
        .find(|occurrence| occurrence.variable == variable("first_target"))
        .unwrap();
    assert_eq!(
        first_target.path,
        vec![
            ExtractionPathSegment::Toml(TomlPathSegment::TableKey {
                key: "release_targets".to_owned(),
            }),
            ExtractionPathSegment::Toml(TomlPathSegment::ArrayIndex { index: 0 }),
            ExtractionPathSegment::Toml(TomlPathSegment::TableKey {
                key: "target".to_owned(),
            }),
        ]
    );

    let second_target = report
        .occurrences
        .iter()
        .find(|occurrence| occurrence.variable == variable("second_target"))
        .unwrap();
    assert_eq!(
        second_target.path,
        vec![
            ExtractionPathSegment::Toml(TomlPathSegment::TableKey {
                key: "release_targets".to_owned(),
            }),
            ExtractionPathSegment::Toml(TomlPathSegment::ArrayIndex { index: 1 }),
            ExtractionPathSegment::Toml(TomlPathSegment::TableKey {
                key: "target".to_owned(),
            }),
        ]
    );
}

#[test]
fn toml_repeated_table_shape_mismatch_fails_closed() {
    let rendered = include_str!("fixtures/reverse-extract/toml-repeated-tables.toml")
        .split_once("\n\n[[release_targets]]")
        .map_or_else(
            || panic!("repeated-table fixture must contain two table blocks"),
            |(first, _)| first.to_owned(),
        );
    let error = extract(&toml_request(
        include_str!("fixtures/reverse-extract/toml-repeated-tables.toml.j2"),
        &rendered,
    ))
    .unwrap_err();

    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractTomlShapeMismatch
    );
    assert!(
        error
            .to_string()
            .contains("TOML array-of-tables length does not match")
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

    let mut nested_template = String::from("value = ");
    let mut nested_rendered = String::from("value = ");
    for _ in 0..=64 {
        nested_template.push_str("{ value = ");
        nested_rendered.push_str("{ value = ");
    }
    nested_template.push_str("\"{{ value }}\"");
    nested_rendered.push_str("\"Ada\"");
    for _ in 0..=64 {
        nested_template.push('}');
        nested_rendered.push('}');
    }
    let nested_error = extract(&toml_request(&nested_template, &nested_rendered)).unwrap_err();
    assert_eq!(
        nested_error.code(),
        sc_composer::DiagnosticCode::ErrExtractInputLimit
    );
}

#[test]
fn json_extraction_rejects_oversized_deep_and_high_occurrence_inputs() {
    let oversized_json = format!(r#"{{"value":"{}"}}"#, "x".repeat(1_048_577));
    assert_input_limit(&json_request(r#"{"value":"{{ value }}"}"#, &oversized_json));

    let mut deep_json_template = String::from("{");
    let mut deep_json_rendered = String::from("{");
    for index in 0..66 {
        let _ = write!(deep_json_template, "\"level{index}\":{{");
        let _ = write!(deep_json_rendered, "\"level{index}\":{{");
    }
    deep_json_template.push_str("\"value\":\"{{ value }}\"");
    deep_json_rendered.push_str("\"value\":\"Ada\"");
    for _ in 0..=66 {
        deep_json_template.push('}');
        deep_json_rendered.push('}');
    }
    assert_input_limit(&json_request(&deep_json_template, &deep_json_rendered));

    let mut occurrence_json_template = String::from("{");
    let mut occurrence_json_rendered = String::from("{");
    for index in 0..10_001 {
        if index > 0 {
            occurrence_json_template.push(',');
            occurrence_json_rendered.push(',');
        }
        let _ = write!(
            occurrence_json_template,
            "\"value{index}\":\"{{{{ value{index} }}}}\""
        );
        let _ = write!(occurrence_json_rendered, "\"value{index}\":\"Ada\"");
    }
    occurrence_json_template.push('}');
    occurrence_json_rendered.push('}');
    assert_input_limit(&json_request(
        &occurrence_json_template,
        &occurrence_json_rendered,
    ));
}

#[test]
fn yaml_extraction_rejects_oversized_deep_and_high_occurrence_inputs() {
    let oversized_yaml = format!("value: \"{}\"\n", "x".repeat(1_048_577));
    assert_input_limit(&yaml_request("value: \"{{ value }}\"\n", &oversized_yaml));

    let mut deep_yaml_template = String::new();
    let mut deep_yaml_rendered = String::new();
    for depth in 0..66 {
        let indent = "  ".repeat(depth);
        let _ = writeln!(deep_yaml_template, "{indent}level{depth}:");
        let _ = writeln!(deep_yaml_rendered, "{indent}level{depth}:");
    }
    let indent = "  ".repeat(66);
    let _ = writeln!(deep_yaml_template, "{indent}value: \"{{ value }}\"");
    let _ = writeln!(deep_yaml_rendered, "{indent}value: Ada");
    assert_input_limit(&yaml_request(&deep_yaml_template, &deep_yaml_rendered));

    let mut occurrence_yaml_template = String::new();
    let mut occurrence_yaml_rendered = String::new();
    for index in 0..10_001 {
        let _ = writeln!(
            occurrence_yaml_template,
            "value{index}: \"{{{{ value{index} }}}}\""
        );
        let _ = writeln!(occurrence_yaml_rendered, "value{index}: Ada");
    }
    assert_input_limit(&yaml_request(
        &occurrence_yaml_template,
        &occurrence_yaml_rendered,
    ));
}

#[test]
fn json_depth_past_serde_recursion_guard_returns_input_limit() {
    let depth = 130;
    let mut template = String::new();
    let mut rendered = String::new();
    for index in 0..depth {
        let _ = write!(template, "{{\"level{index}\":{{");
        let _ = write!(rendered, "{{\"level{index}\":{{");
    }
    template.push_str("\"value\":\"{{ value }}\"");
    rendered.push_str("\"value\":\"Ada\"");
    for _ in 0..depth {
        template.push('}');
        rendered.push('}');
    }

    let error = extract(&json_request(&template, &rendered)).unwrap_err();
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractInputLimit
    );
}

#[test]
fn yaml_depth_past_serde_recursion_guard_returns_input_limit() {
    let depth = 130;
    let mut template = String::new();
    let mut rendered = String::new();
    for level in 0..depth {
        let indent = "  ".repeat(level);
        let _ = writeln!(template, "{indent}level{level}:");
        let _ = writeln!(rendered, "{indent}level{level}:");
    }
    let indent = "  ".repeat(depth);
    let _ = writeln!(template, "{indent}value: \"{{ value }}\"");
    let _ = writeln!(rendered, "{indent}value: Ada");

    let error = extract(&yaml_request(&template, &rendered)).unwrap_err();
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractInputLimit
    );
}

#[test]
fn xml_extraction_rejects_oversized_deep_and_high_occurrence_inputs() {
    let oversized_xml = format!("<root><value>{}</value></root>", "x".repeat(1_048_577));
    assert_input_limit(&request(
        "<root><value>{{ value }}</value></root>",
        &oversized_xml,
    ));

    let mut deep_xml_template = String::from("<root>");
    let mut deep_xml_rendered = String::from("<root>");
    for depth in 0..66 {
        let _ = write!(deep_xml_template, "<level{depth}>");
        let _ = write!(deep_xml_rendered, "<level{depth}>");
    }
    deep_xml_template.push_str("<value>{{ value }}</value>");
    deep_xml_rendered.push_str("<value>Ada</value>");
    for depth in (0..66).rev() {
        let _ = write!(deep_xml_template, "</level{depth}>");
        let _ = write!(deep_xml_rendered, "</level{depth}>");
    }
    deep_xml_template.push_str("</root>");
    deep_xml_rendered.push_str("</root>");
    assert_input_limit(&request(&deep_xml_template, &deep_xml_rendered));

    let mut occurrence_xml_template = String::from("<root>");
    let mut occurrence_xml_rendered = String::from("<root>");
    for index in 0..10_001 {
        let _ = write!(
            occurrence_xml_template,
            "<value id=\"{index}\">{{{{ value{index} }}}}</value>"
        );
        let _ = write!(occurrence_xml_rendered, "<value id=\"{index}\">Ada</value>");
    }
    occurrence_xml_template.push_str("</root>");
    occurrence_xml_rendered.push_str("</root>");
    assert_input_limit(&request(&occurrence_xml_template, &occurrence_xml_rendered));
}

#[test]
fn xml_block_occurrence_limit_is_a_durable_boundary() {
    let mut template = String::from("<root>");
    let mut rendered = String::from("<root>");
    for index in 0..10_001 {
        let _ = write!(template, "<item>{{{{ value{index} }}}}</item>");
        let _ = write!(rendered, "<item><strong>value-{index}</strong></item>");
    }
    template.push_str("</root>");
    rendered.push_str("</root>");

    assert_input_limit(&request(&template, &rendered));
}

#[test]
fn cross_format_corpus_preserves_equivalent_contracts() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/reverse-extract/cross-format-corpus.json"
    ))
    .unwrap();
    assert_eq!(corpus["schema_version"], "phase-h6-cross-format-corpus/v1");

    for case in corpus["cases"].as_array().unwrap() {
        let format = match case["format"].as_str().unwrap() {
            "json" => ExtractFormat::Json,
            "yaml" => ExtractFormat::Yaml,
            "toml" => ExtractFormat::Toml,
            other => panic!("unknown corpus format {other}"),
        };
        let include = case["include"]
            .as_array()
            .map(|names| {
                names
                    .iter()
                    .map(|name| variable(name.as_str().unwrap()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let request = ExtractRequest::new(
            case["template"].as_str().unwrap(),
            case["rendered"].as_str().unwrap(),
            format,
            &include,
            &[],
        );
        let expected_code = case["expected_code"].as_str();
        match case["kind"].as_str().unwrap() {
            "success" | "filter" => {
                let report = extract(&request).unwrap();
                let values = report
                    .values
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.clone()))
                    .collect::<std::collections::BTreeMap<_, _>>();
                let expected = case["expected_values"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(name, value)| (name.clone(), value.as_str().unwrap().to_owned()))
                    .collect::<std::collections::BTreeMap<_, _>>();
                assert_eq!(values, expected, "{}", case["id"]);
            }
            "ambiguity" => {
                let report = extract(&request).unwrap();
                assert!(report.values.is_empty(), "{}", case["id"]);
                assert!(
                    report
                        .diagnostics
                        .iter()
                        .any(|diagnostic| { Some(diagnostic.code.as_str()) == expected_code })
                );
                assert_eq!(report.occurrences.len(), 2, "{}", case["id"]);
            }
            "malformed" | "unsupported" => {
                let error = extract(&request).unwrap_err();
                assert_eq!(Some(error.code().as_str()), expected_code, "{}", case["id"]);
            }
            other => panic!("unknown corpus case kind {other}"),
        }
    }
}

#[test]
fn toml_occurrence_limit_is_a_durable_boundary() {
    let mut template = String::new();
    let mut rendered = String::new();
    for index in 0..10_001 {
        let _ = writeln!(template, "value{index} = \"{{{{ value{index} }}}}\"");
        let _ = writeln!(rendered, "value{index} = \"Ada\"");
    }
    let error = extract(&toml_request(&template, &rendered)).unwrap_err();
    assert_eq!(
        error.code(),
        sc_composer::DiagnosticCode::ErrExtractInputLimit
    );
}
