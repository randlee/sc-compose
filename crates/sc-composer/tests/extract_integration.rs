use sc_composer::{
    ExtractError, ExtractFormat, ExtractRequest, ExtractionDiagnosticKind, VariableName,
    XmlPathSegment, extract,
};

fn variable(name: &str) -> VariableName {
    VariableName::new(name).unwrap()
}

fn request(template: &'static str, rendered: &'static str) -> ExtractRequest<'static> {
    ExtractRequest::new(template, rendered, ExtractFormat::Xml, &[], &[])
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
        XmlPathSegment::Element {
            name: "item".to_owned(),
            ordinal: 1,
        }
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
            sc_composer::RecoveryHintKind::ReviewConfiguration { key }
                if key.contains("scalar syntax")
        )
    }));
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
