//! Deterministic structural matching for the supported XML extraction subset.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;

use serde::{Deserialize, Serialize};

use crate::diagnostics::DiagnosticCode;
use crate::error::RecoveryHintKind;
use crate::frontmatter::parse_template_document;
use crate::types::VariableName;

use super::xml_prefix::{RemovedPrefix, normalize_rendered};
use super::{
    ExtractError, ExtractRequest, ExtractionDiagnostic, ExtractionDiagnosticKind,
    ExtractionOccurrence, ExtractionReport, raw_text,
};
use xml_evidence::{
    Capture, Evidence, collect_expected_evidence, collect_template_occurrences, path_exists,
};
use xml_model::{XmlDocument, XmlElement, XmlNode, parse_xml};

#[path = "xml_evidence.rs"]
mod xml_evidence;
#[path = "xml_match.rs"]
mod xml_match;
#[path = "xml_model.rs"]
mod xml_model;
#[path = "xml_reject.rs"]
mod xml_reject;
#[path = "xml_serialize.rs"]
mod xml_serialize;

const MAX_XML_INPUT_BYTES: usize = 1_048_576;
const MAX_XML_OCCURRENCES: usize = 10_000;

/// XML element/attribute path evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmlPathSegment {
    /// An XML element and its zero-based ordinal among same-named siblings.
    Element {
        /// Element name.
        name: String,
        /// Zero-based sibling ordinal.
        ordinal: usize,
    },
    /// An XML attribute on the preceding element path.
    Attribute {
        /// Attribute name.
        name: String,
    },
}

/// XML source evidence for a recovered scalar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmlExtractionSource {
    /// The scalar was recovered from an XML attribute.
    Attribute {
        /// Attribute name.
        name: String,
    },
    /// The scalar was recovered from an XML text node.
    TextNode,
    /// The value occupies an element's complete content and may include
    /// canonicalized child markup.
    ElementContent,
}

/// XML occurrence report entry.
pub type XmlExtractionOccurrence = ExtractionOccurrence<XmlPathSegment, XmlExtractionSource>;

/// XML report over the generic extraction contract.
pub type XmlExtractionReport = ExtractionReport<XmlPathSegment, XmlExtractionSource>;

/// Extract values from a known XML template and rendered XML document.
pub(crate) fn extract_xml(
    request: &ExtractRequest<'_>,
) -> Result<XmlExtractionReport, ExtractError> {
    validate_input_size(request.template, "template")?;
    validate_input_size(request.rendered, "rendered XML")?;
    let parsed_template = parse_template_document(request.template).map_err(|error| {
        ExtractError::unsupported_with_source(
            format!("template frontmatter is not supported: {error}"),
            error,
        )
    })?;
    let template_source = parsed_template.body();
    let normalized = normalize_rendered(request.rendered)?;
    xml_reject::reject_unsupported_template_syntax(template_source)?;
    xml_reject::reject_dynamic_element_syntax(template_source)?;
    let template = parse_xml(template_source)?;
    let rendered = parse_xml(&normalized.source)?;
    xml_reject::reject_dynamic_element_names(&template)?;
    xml_reject::reject_namespaces(&template)?;
    xml_reject::reject_namespaces(&rendered)?;

    if let Some(mut report) = missing_occurrence_report(&template, &rendered, request)? {
        if let Some(prefix) = normalized.removed.as_ref() {
            report
                .diagnostics
                .insert(0, dirty_prefix_diagnostic(prefix));
        }
        return Ok(report);
    }

    let mut captures = Vec::new();
    let mut evidence = Evidence::default();
    collect_expected_evidence(&template.root, &mut evidence)?;
    let root_path = vec![XmlPathSegment::Element {
        name: template.root.name.clone(),
        ordinal: 0,
    }];
    xml_match::match_element(
        &template.root,
        &rendered.root,
        &root_path,
        &mut captures,
        &mut evidence,
    )?;

    let selected = captures
        .into_iter()
        .filter(|capture| selected_variable(&capture.variable, request))
        .collect::<Vec<_>>();

    let mut values = BTreeMap::new();
    let mut occurrences = Vec::new();
    let mut diagnostics = normalized
        .removed
        .as_ref()
        .map(dirty_prefix_diagnostic)
        .into_iter()
        .collect::<Vec<_>>();

    for capture in selected {
        values
            .entry(capture.variable.clone())
            .or_insert_with(|| capture.rendered_text.clone());
        occurrences.push(XmlExtractionOccurrence {
            variable: capture.variable,
            path: capture.path,
            source: capture.source,
            rendered_text: Some(capture.rendered_text),
        });
    }

    let evidence_total = evidence.expected_structural + evidence.expected_static;
    let matched_evidence = evidence.structural_matches + evidence.static_matches;
    let confidence = if evidence_total == 0 {
        0.0
    } else {
        evidence_confidence(matched_evidence, evidence_total)
    };
    if confidence < 0.75 {
        diagnostics.push(ExtractionDiagnostic::new(
            DiagnosticCode::WarnExtractLowConfidence,
            ExtractionDiagnosticKind::NotObserved,
            "insufficient structural or static evidence for a high-confidence extraction",
            None,
        ));
    }

    XmlExtractionReport::new(values, occurrences, confidence, diagnostics)
}

fn evidence_confidence(matched: usize, total: usize) -> f64 {
    let matched = u32::try_from(matched).unwrap_or(u32::MAX);
    let total = u32::try_from(total).unwrap_or(u32::MAX);
    f64::from(matched) / f64::from(total)
}

fn dirty_prefix_diagnostic(prefix: &RemovedPrefix) -> ExtractionDiagnostic {
    ExtractionDiagnostic::new(
        DiagnosticCode::WarnExtractDirtyPrefixStripped,
        ExtractionDiagnosticKind::NotObserved,
        format!(
            "stripped rendered XML preamble bytes 0..{} (line {}, column {})",
            prefix.byte_end, prefix.line, prefix.column
        ),
        None,
    )
}

fn selected_variable(variable: &VariableName, request: &ExtractRequest<'_>) -> bool {
    (request.include.is_empty() || request.include.contains(variable))
        && !request.exclude.contains(variable)
}

fn missing_occurrence_report(
    template: &XmlDocument,
    rendered: &XmlDocument,
    request: &ExtractRequest<'_>,
) -> Result<Option<XmlExtractionReport>, ExtractError> {
    let mut template_occurrences = Vec::new();
    collect_template_occurrences(
        &template.root,
        &[XmlPathSegment::Element {
            name: template.root.name.clone(),
            ordinal: 0,
        }],
        &mut template_occurrences,
    )?;
    let missing_variables = template_occurrences
        .iter()
        .filter(|occurrence| selected_variable(&occurrence.variable, request))
        .filter(|occurrence| !path_exists(&rendered.root, &occurrence.path))
        .map(|occurrence| occurrence.variable.clone())
        .collect::<BTreeSet<_>>();
    if missing_variables.is_empty() {
        return Ok(None);
    }
    let mut diagnostics = missing_variables
        .into_iter()
        .map(|variable| {
            ExtractionDiagnostic::new(
                DiagnosticCode::WarnExtractNotObserved,
                ExtractionDiagnosticKind::NotObserved,
                format!("variable occurrence was not observed in rendered XML: {variable}"),
                None,
            )
        })
        .collect::<Vec<_>>();
    diagnostics.push(ExtractionDiagnostic::new(
        DiagnosticCode::WarnExtractLowConfidence,
        ExtractionDiagnosticKind::NotObserved,
        "no structural occurrence was observed for the selected variables",
        None,
    ));
    Ok(Some(XmlExtractionReport::new(
        BTreeMap::new(),
        Vec::new(),
        0.0,
        diagnostics,
    )?))
}

fn is_single_variable(value: &str) -> bool {
    value.trim().starts_with("{{") && value.trim().ends_with("}}")
}

fn parse_value_segments<'a>(
    value: &'a str,
    path: &[XmlPathSegment],
) -> Result<Vec<raw_text::RawTextSegment<'a>>, ExtractError> {
    raw_text::parse_raw_text_segments(value).map_err(|error| map_raw_text_error(error, path))
}

fn map_raw_text_error(error: raw_text::RawTextMatchError, path: &[XmlPathSegment]) -> ExtractError {
    match error.scope() {
        raw_text::RawTextErrorScope::Request => match error {
            raw_text::RawTextMatchError::InvalidTemplate { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractTemplateUnsupported,
                    ExtractionDiagnosticKind::Unsupported,
                    with_span(&message, span),
                    RecoveryHintKind::UnsupportedConstruct {
                        description: "use supported scalar XML placeholders".to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::StaticMismatch { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractXmlStaticMismatch,
                    ExtractionDiagnosticKind::Unsupported,
                    with_span(&message, span),
                    RecoveryHintKind::InspectInput {
                        description: "align rendered XML static content with the known template"
                            .to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::AmbiguousDelimiter { span, message } => {
                ExtractError::ambiguous(with_span(&message, span), None)
            }
        },
        raw_text::RawTextErrorScope::Occurrence => match error {
            raw_text::RawTextMatchError::InvalidTemplate { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractTemplateUnsupported,
                    ExtractionDiagnosticKind::Unsupported,
                    with_span(&message, span),
                    RecoveryHintKind::UnsupportedConstruct {
                        description: "use supported scalar XML placeholders".to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::StaticMismatch { span, message } => {
                ExtractError::format_error(
                    DiagnosticCode::ErrExtractXmlStaticMismatch,
                    ExtractionDiagnosticKind::Unsupported,
                    format!("{} at {}", with_span(&message, span), format_path(path)),
                    RecoveryHintKind::InspectInput {
                        description: "align rendered XML static content with the known template"
                            .to_owned(),
                    },
                )
            }
            raw_text::RawTextMatchError::AmbiguousDelimiter { span, message } => {
                if message.contains("adjacent variable") {
                    ExtractError::ambiguous_delimiter(
                        "adjacent XML variable expressions have no structural delimiter",
                    )
                } else {
                    ExtractError::ambiguous(with_span(&message, span), None)
                }
            }
        },
    }
}

fn with_span(message: &str, span: Option<std::ops::Range<usize>>) -> String {
    span.map_or_else(
        || message.to_owned(),
        |span| format!("{message} (candidate bytes {}..{})", span.start, span.end),
    )
}

fn format_path(path: &[XmlPathSegment]) -> String {
    let mut result = String::from("$");
    for segment in path {
        match segment {
            XmlPathSegment::Element { name, ordinal } => {
                result.push('.');
                result.push_str(name);
                result.push('[');
                result.push_str(&ordinal.to_string());
                result.push(']');
            }
            XmlPathSegment::Attribute { name } => {
                result.push_str(".@");
                result.push_str(name);
            }
        }
    }
    result
}

fn validate_input_size(source: &str, label: &str) -> Result<(), ExtractError> {
    if source.len() > MAX_XML_INPUT_BYTES {
        return Err(input_limit_error(format!(
            "XML {label} input is {} bytes; maximum is {MAX_XML_INPUT_BYTES} bytes",
            source.len()
        )));
    }
    Ok(())
}

fn input_limit_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractInputLimit,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "reduce XML input size, nesting depth, or occurrence count".to_owned(),
        },
    )
}

fn malformed(message: String) -> ExtractError {
    ExtractError::malformed(message)
}

fn malformed_with_source<E>(message: String, source: E) -> ExtractError
where
    E: StdError + Send + Sync + 'static,
{
    ExtractError::malformed_with_source(message, source)
}
