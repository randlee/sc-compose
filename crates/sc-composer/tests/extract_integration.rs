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
