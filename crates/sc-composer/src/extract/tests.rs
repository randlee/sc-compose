use serde_json::json;

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
fn valid_request_fails_closed_until_g2_engine_exists() {
    let request = ExtractRequest::new(
        "<x>{{ name }}</x>",
        "<x>Ada</x>",
        ExtractFormat::Xml,
        &[],
        &[],
    );
    let error = extract(&request).unwrap_err();
    assert_eq!(
        error.diagnostic().unwrap().kind,
        ExtractionDiagnosticKind::Unsupported
    );
}

#[test]
fn errors_expose_canonical_codes_and_recovery_hints() {
    let invalid = ExtractRequest::new("", "<x/>", ExtractFormat::Xml, &[], &[])
        .validate()
        .unwrap_err();
    assert_eq!(invalid.code(), DiagnosticCode::ErrExtractInvalidRequest);
    assert!(!invalid.recovery_hints().is_empty());

    let request = ExtractRequest::new(
        "<x>{{ name }}</x>",
        "<x>Ada</x>",
        ExtractFormat::Xml,
        &[],
        &[],
    );
    let unsupported = extract(&request).unwrap_err();
    assert_eq!(unsupported.code(), DiagnosticCode::ErrExtractUnsupported);
    assert!(!unsupported.recovery_hints().is_empty());
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
fn same_variable_occurrences_fail_ambiguously_without_map_overwrite() {
    let result = ExtractionReport::new(
        BTreeMap::new(),
        vec![occurrence("name", "Ada"), occurrence("name", "Grace")],
        1.0,
        Vec::new(),
    );
    let error = result.unwrap_err();
    assert_eq!(
        error.diagnostic().unwrap().kind,
        ExtractionDiagnosticKind::Ambiguous
    );
    assert_eq!(
        error.diagnostic().unwrap().occurrence,
        Some(OccurrenceIndex(1))
    );
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
