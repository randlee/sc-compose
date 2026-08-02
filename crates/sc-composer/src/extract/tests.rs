use serde_json::json;

use crate::error::RecoveryHintKind;

use super::*;

fn variable(name: &str) -> VariableName {
    VariableName::new(name).unwrap()
}

fn occurrence(name: &str, rendered_text: &str) -> ExtractionOccurrence {
    ExtractionOccurrence {
        variable: variable(name),
        path: vec![OccurrencePathSegment::Node {
            label: "root".to_owned(),
            ordinal: 0,
        }],
        source: OccurrenceSource::Named {
            kind: "text".to_owned(),
            label: None,
        },
        rendered_text: Some(rendered_text.to_owned()),
    }
}

#[test]
fn request_validation_accepts_empty_filters_and_rejects_conflicts() {
    let name = variable("name");
    let request = ExtractRequest::new(
        "<x>{{ name }}</x>",
        "<x>Ada</x>",
        ExtractFormat::Xml,
        &[],
        &[],
    );
    request.validate().unwrap();

    let conflicting = ExtractRequest::new(
        "<x>{{ name }}</x>",
        "<x>Ada</x>",
        ExtractFormat::Xml,
        std::slice::from_ref(&name),
        std::slice::from_ref(&name),
    );
    let error = conflicting.validate().unwrap_err();
    assert!(matches!(error, ExtractError::InvalidRequest { .. }));
}

#[test]
fn request_validation_rejects_empty_sources_and_duplicate_filters() {
    let name = variable("name");
    let duplicate = [name.clone(), name.clone()];
    let request = ExtractRequest::new("<x/>", "<x/>", ExtractFormat::Xml, &duplicate, &[]);
    assert!(matches!(
        request.validate().unwrap_err(),
        ExtractError::InvalidRequest { .. }
    ));

    let empty = ExtractRequest::new("", "<x/>", ExtractFormat::Xml, &[], &[]);
    assert!(matches!(
        empty.validate().unwrap_err(),
        ExtractError::InvalidRequest { .. }
    ));
}

#[test]
fn valid_request_extracts_with_g2_engine() {
    let request = ExtractRequest::new(
        "<x>{{ name }}</x>",
        "<x>Ada</x>",
        ExtractFormat::Xml,
        &[],
        &[],
    );
    let report = extract(&request).unwrap();
    assert_eq!(report.values[&variable("name")], "Ada");
}

#[test]
fn weakly_anchored_variable_has_subunit_confidence() {
    let report = extract(&xml_request("<x>{{ value }}</x>", "<x>Ada</x>")).unwrap();

    assert_eq!(report.values[&variable("value")], "Ada");
    assert!(report.confidence > 0.0);
    assert!(report.confidence < 1.0);
}

#[test]
fn strong_match_confidence_exceeds_weak_match_confidence() {
    let weak = extract(&xml_request("<x>{{ value }}</x>", "<x>Ada</x>")).unwrap();
    let strong = extract(&xml_request(
        "<root><name>Hello {{ name }}!</name></root>",
        "<root><name>Hello Ada!</name></root>",
    ))
    .unwrap();

    assert!(strong.confidence > weak.confidence);
    assert!(strong.confidence > 0.99);
}

#[test]
fn errors_expose_canonical_codes_and_recovery_hints() {
    let invalid = ExtractRequest::new("", "<x/>", ExtractFormat::Xml, &[], &[])
        .validate()
        .unwrap_err();
    assert_eq!(invalid.code(), DiagnosticCode::ErrExtractInvalidRequest);
    assert!(!invalid.recovery_hints().is_empty());

    let request = ExtractRequest::new(
        "<x>{{ name | upper }}</x>",
        "<x>Ada</x>",
        ExtractFormat::Xml,
        &[],
        &[],
    );
    let unsupported = extract(&request).unwrap_err();
    assert_eq!(unsupported.code(), DiagnosticCode::ErrExtractUnsupported);
    assert!(matches!(
        &unsupported.recovery_hints()[0].kind,
        RecoveryHintKind::UnsupportedConstruct { .. }
    ));

    let malformed = extract(&xml_request("<x>{{ value }}</x>", "<x")).unwrap_err();
    assert!(matches!(
        &malformed.recovery_hints()[0].kind,
        RecoveryHintKind::InspectInput { .. }
    ));

    let ambiguous =
        extract(&xml_request("<x>{{ first }}{{ second }}</x>", "<x>AB</x>")).unwrap_err();
    assert!(matches!(
        &ambiguous.recovery_hints()[0].kind,
        RecoveryHintKind::DisambiguateOccurrences { description }
            if description.contains("adjacent")
    ));

    let duplicate = ExtractError::ambiguous(
        "variable has multiple structural occurrences: name",
        Some(OccurrenceIndex(1)),
    );
    assert!(matches!(
        &duplicate.recovery_hints()[0].kind,
        RecoveryHintKind::DisambiguateOccurrences { description }
            if description.contains("occurrence path")
    ));
}

#[test]
fn report_and_diagnostic_serialization_is_stable() {
    let diagnostic = ExtractionDiagnostic::new(
        DiagnosticCode::ErrExtractUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        "filter is outside the reversible subset",
        Some(OccurrenceIndex(2)),
    );
    let report = ExtractionReport::new(
        BTreeMap::from([(variable("answer"), "42".to_owned())]),
        vec![occurrence("answer", "42")],
        0.75,
        vec![diagnostic.clone()],
    )
    .unwrap();

    let serialized = serde_json::to_value(&report).unwrap();
    assert_eq!(serialized["values"]["answer"], json!("42"));
    assert_eq!(serialized["confidence"], json!(0.75));
    assert_eq!(
        serialized["diagnostics"][0]["code"],
        json!("ERR_EXTRACT_UNSUPPORTED")
    );
    assert_eq!(serialized["diagnostics"][0]["kind"], json!("unsupported"));
    assert_eq!(serialized["diagnostics"][0]["occurrence"], json!(2));
}

#[test]
fn confidence_rejects_non_finite_and_out_of_range_values() {
    for confidence in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, 1.01] {
        let result = ExtractionReport::<OccurrencePathSegment, OccurrenceSource>::new(
            BTreeMap::new(),
            Vec::new(),
            confidence,
            Vec::new(),
        );
        assert!(matches!(result, Err(ExtractError::InvalidRequest { .. })));
    }
}

#[test]
fn same_variable_occurrences_are_reported_without_map_overwrite() {
    let report = ExtractionReport::new(
        BTreeMap::new(),
        vec![occurrence("name", "Ada"), occurrence("name", "Grace")],
        1.0,
        Vec::new(),
    )
    .unwrap();
    assert!(!report.values.contains_key(&variable("name")));
    assert_eq!(
        report.diagnostics[0].kind,
        ExtractionDiagnosticKind::Ambiguous
    );
    assert_eq!(report.diagnostics[0].occurrence, Some(OccurrenceIndex(1)));
}

#[test]
fn string_value_semantics_do_not_infer_a_number() {
    let report = ExtractionReport::new(
        BTreeMap::from([(variable("count"), "42".to_owned())]),
        vec![occurrence("count", "42")],
        1.0,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(report.values[&variable("count")], "42");
}

fn xml_request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
    ExtractRequest::new(template, rendered, ExtractFormat::Xml, &[], &[])
}

#[test]
fn xml_extracts_attributes_text_and_static_prefix_suffix() {
    let report = extract(&xml_request(
        r#"<doc id="{{ id }}"><name>Hello {{ name }}!</name></doc>"#,
        r#"<doc id="42"><name>Hello Ada!</name></doc>"#,
    ))
    .unwrap();

    assert_eq!(report.values[&variable("id")], "42");
    assert_eq!(report.values[&variable("name")], "Ada");
    assert!(report.occurrences.iter().any(|occurrence| {
        occurrence.variable == variable("id")
            && occurrence.source
                == ExtractionSource::Attribute {
                    name: "id".to_owned(),
                }
    }));
}

#[test]
fn xml_repeated_siblings_use_distinct_structural_ordinals() {
    let report = extract(&xml_request(
        "<root><item>{{ first }}</item><item>{{ second }}</item></root>",
        "<root><item>A</item><item>B</item></root>",
    ))
    .unwrap();

    assert_eq!(report.values[&variable("first")], "A");
    assert_eq!(report.values[&variable("second")], "B");
    let item_ordinals = report
        .occurrences
        .iter()
        .map(|occurrence| occurrence.path[1].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        item_ordinals,
        vec![
            XmlPathSegment::Element {
                name: "item".to_owned(),
                ordinal: 0,
            },
            XmlPathSegment::Element {
                name: "item".to_owned(),
                ordinal: 1,
            },
        ]
    );
}

#[test]
fn xml_decodes_entities_preserves_whitespace_and_supports_empty_values() {
    let report = extract(&xml_request(
        "<root><value>  {{ value }}  </value><empty>{{ empty }}</empty></root>",
        "<root><value>  A &amp; B  </value><empty/></root>",
    ))
    .unwrap();

    assert_eq!(report.values[&variable("value")], "A & B");
    assert_eq!(report.values[&variable("empty")], "");
}

#[test]
fn xml_conflicting_same_variable_occurrences_are_ambiguous() {
    let report = extract(&xml_request(
        "<root><item>{{ name }}</item><item>{{ name }}</item></root>",
        "<root><item>Ada</item><item>Grace</item></root>",
    ))
    .unwrap();

    assert!(!report.values.contains_key(&variable("name")));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ErrExtractAmbiguous
            && diagnostic.kind == ExtractionDiagnosticKind::Ambiguous
    }));
    assert_eq!(report.occurrences.len(), 2);
    assert!(report.confidence < 0.75);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == DiagnosticCode::WarnExtractLowConfidence })
    );
}

#[test]
fn xml_rejects_malformed_and_unsupported_inputs_without_values() {
    let malformed = extract(&xml_request("<root>{{ value }}</root>", "<root")).unwrap_err();
    assert_eq!(malformed.code(), DiagnosticCode::ErrExtractMalformed);
    assert!(std::error::Error::source(&malformed).is_some());

    let unsupported = extract(&xml_request(
        "<root>{% if enabled %}{{ value }}{% endif %}</root>",
        "<root>value</root>",
    ))
    .unwrap_err();
    assert_eq!(unsupported.code(), DiagnosticCode::ErrExtractUnsupported);
}

#[test]
fn xml_rejects_dotted_expressions_as_unsupported() {
    let error = extract(&xml_request(
        "<root><name>{{ user.name }}</name></root>",
        "<root><name>Ada</name></root>",
    ))
    .unwrap_err();

    assert!(matches!(error, ExtractError::UnsupportedSyntax { .. }));
    assert_eq!(error.code(), DiagnosticCode::ErrExtractUnsupported);
}

#[test]
fn xml_extracts_underscore_and_hyphen_scalar_names() {
    let report = extract(&xml_request(
        "<root><value>{{ under_score }} {{ hy-phen }}</value></root>",
        "<root><value>Ada Grace</value></root>",
    ))
    .unwrap();

    assert_eq!(report.values[&variable("under_score")], "Ada");
    assert_eq!(report.values[&variable("hy-phen")], "Grace");
}

#[test]
fn xml_reports_missing_occurrences_without_fabricating_values() {
    let report = extract(&xml_request(
        "<root><name>{{ name }}</name></root>",
        "<root></root>",
    ))
    .unwrap();

    assert!(report.values.is_empty());
    assert!(report.confidence.abs() < f64::EPSILON);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::WarnExtractNotObserved
            && diagnostic.kind == ExtractionDiagnosticKind::NotObserved
    }));
}

#[test]
fn xml_extracts_whitespace_padded_bare_variable_from_empty_element() {
    let report = extract(&xml_request(
        "<root><value> {{ value }} </value></root>",
        "<root><value/></root>",
    ))
    .unwrap();

    assert_eq!(report.values[&variable("value")], "");
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::WarnExtractNotObserved)
    );
}

#[test]
fn xml_rejects_ambiguous_namespace_policy() {
    let error = extract(&xml_request(
        "<root xmlns:p=\"urn:test\"><p:item>{{ value }}</p:item></root>",
        "<root xmlns:p=\"urn:test\"><p:item>Ada</p:item></root>",
    ))
    .unwrap_err();

    assert_eq!(error.code(), DiagnosticCode::ErrExtractUnsupported);
}
