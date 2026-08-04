//! Known-template extraction for unstructured raw text.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostics::DiagnosticCode;
use crate::error::RecoveryHintKind;
use crate::frontmatter::parse_template_document;
use crate::types::VariableName;

use super::raw_text::RawTextMatchError;
use super::{
    ExtractError, ExtractRequest, ExtractionDiagnostic, ExtractionDiagnosticKind,
    ExtractionOccurrence, ExtractionReport, raw_text,
};

/// Raw-text byte and line/column evidence for one captured value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPathSegment {
    /// Zero-based inclusive byte offset in the rendered source.
    pub byte_start: usize,
    /// Zero-based exclusive byte offset in the rendered source.
    pub byte_end: usize,
    /// One-based line containing the beginning of the span.
    pub line: usize,
    /// One-based column containing the beginning of the span.
    pub column: usize,
}

/// Source evidence for a raw-text capture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawExtractionSource {
    /// The value was captured from a rendered text span.
    TextSpan,
}

/// Raw-text extraction report.
pub type RawExtractionReport = ExtractionReport<RawPathSegment, RawExtractionSource>;

/// Extract values from a known template and rendered unstructured text.
pub(crate) fn extract_raw(
    request: &ExtractRequest<'_>,
) -> Result<RawExtractionReport, ExtractError> {
    let parsed_template = parse_template_document(request.template).map_err(|error| {
        template_error(format!("raw-text template frontmatter is invalid: {error}"))
    })?;
    let template = parsed_template.body();
    if template.contains("{%") || template.contains("{#") {
        return Err(template_error(
            "raw-text extraction does not support Jinja statements or comments",
        ));
    }

    let segments = raw_text::parse_raw_text_segments(template).map_err(raw_error)?;
    let matched = raw_text::match_raw_text(&raw_text::RawTextMatchInput {
        segments: &segments,
        rendered_candidate: request.rendered,
    })
    .map_err(raw_error)?;

    let mut values = BTreeMap::new();
    let mut occurrences = Vec::new();
    for capture in matched.captures {
        if !selected_variable(&capture.variable, request) {
            continue;
        }
        values
            .entry(capture.variable.clone())
            .or_insert_with(|| capture.rendered_text.clone());
        let (line, column) = line_column(request.rendered, capture.span.start);
        occurrences.push(ExtractionOccurrence {
            variable: capture.variable,
            path: vec![RawPathSegment {
                byte_start: capture.span.start,
                byte_end: capture.span.end,
                line,
                column,
            }],
            source: RawExtractionSource::TextSpan,
            rendered_text: Some(capture.rendered_text),
        });
    }

    let confidence = if matched.static_matches == 0 {
        0.0
    } else {
        1.0
    };
    let diagnostics = if confidence == 0.0 && !occurrences.is_empty() {
        vec![ExtractionDiagnostic::new(
            DiagnosticCode::WarnExtractLowConfidence,
            ExtractionDiagnosticKind::NotObserved,
            "raw-text capture has no static anchor; confidence is reduced",
            None,
        )]
    } else {
        Vec::new()
    };

    RawExtractionReport::new(values, occurrences, confidence, diagnostics)
}

fn selected_variable(variable: &VariableName, request: &ExtractRequest<'_>) -> bool {
    (request.include.is_empty() || request.include.contains(variable))
        && !request.exclude.contains(variable)
}

fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = &source[..byte_offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = source[column_start..byte_offset].chars().count() + 1;
    (line, column)
}

fn template_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractTemplateUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "use static text and double-brace scalar placeholders".to_owned(),
        },
    )
}

fn raw_error(error: RawTextMatchError) -> ExtractError {
    match error {
        RawTextMatchError::InvalidTemplate { span, message }
        | RawTextMatchError::StaticMismatch { span, message } => {
            ExtractError::format_error(
                DiagnosticCode::ErrExtractTemplateUnsupported,
                ExtractionDiagnosticKind::Unsupported,
                with_span(&message, span),
                RecoveryHintKind::UnsupportedConstruct {
                    description: "align rendered static text with the known template and use supported scalar placeholders".to_owned(),
                },
            )
        }
        RawTextMatchError::AmbiguousDelimiter { span, message } => {
            ExtractError::ambiguous_delimiter(with_span(&message, span))
        }
    }
}

fn with_span(message: &str, span: Option<std::ops::Range<usize>>) -> String {
    span.map_or_else(
        || message.to_owned(),
        |span| format!("{message} (candidate bytes {}..{})", span.start, span.end),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(template: &'a str, rendered: &'a str) -> ExtractRequest<'a> {
        ExtractRequest::new(
            template,
            rendered,
            super::super::ExtractFormat::Raw,
            &[],
            &[],
        )
    }

    #[test]
    fn extracts_markdown_values_with_raw_spans() {
        let report = extract_raw(&request(
            "# {{ title }}\n\nOwner: {{ owner }}",
            "# Launch\n\nOwner: Ada",
        ))
        .expect("raw extraction");

        assert_eq!(
            report.values[&VariableName::new("title").unwrap()],
            "Launch"
        );
        assert_eq!(report.values[&VariableName::new("owner").unwrap()], "Ada");
        assert_eq!(report.occurrences[0].path[0].byte_start, 2);
        assert_eq!(report.occurrences[0].path[0].line, 1);
        assert_eq!(report.occurrences[0].path[0].column, 3);
        assert_eq!(report.occurrences[1].path[0].line, 3);
    }

    #[test]
    fn filters_keep_matching_context_but_remove_reported_values() {
        let include = vec![VariableName::new("owner").unwrap()];
        let request = ExtractRequest::new(
            "Title: {{ title }}; Owner: {{ owner }}",
            "Title: Launch; Owner: Ada",
            super::super::ExtractFormat::Raw,
            &include,
            &[],
        );
        let report = extract_raw(&request).expect("raw extraction");

        assert_eq!(report.values.len(), 1);
        assert_eq!(report.values[&VariableName::new("owner").unwrap()], "Ada");
        assert_eq!(report.occurrences.len(), 1);
    }

    #[test]
    fn pure_variable_is_recovered_with_low_confidence_warning() {
        let report = extract_raw(&request("{{ value }}", "anything")).expect("raw extraction");

        assert_eq!(
            report.values[&VariableName::new("value").unwrap()],
            "anything"
        );
        assert!(report.confidence.abs() < f64::EPSILON);
        assert_eq!(
            report.diagnostics[0].code,
            DiagnosticCode::WarnExtractLowConfidence
        );
    }

    #[test]
    fn raw_mode_rejects_statements_static_mismatches_and_adjacent_variables() {
        for (template, rendered, code) in [
            (
                "{% for item in items %}{{ item }}{% endfor %}",
                "one",
                DiagnosticCode::ErrExtractTemplateUnsupported,
            ),
            (
                "Hello {{ name }}",
                "Goodbye Ada",
                DiagnosticCode::ErrExtractTemplateUnsupported,
            ),
            (
                "{{ first }}{{ second }}",
                "AdaJones",
                DiagnosticCode::ErrExtractAmbiguous,
            ),
        ] {
            assert_eq!(
                extract_raw(&request(template, rendered))
                    .unwrap_err()
                    .code(),
                code
            );
        }
    }
}
