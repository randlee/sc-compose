//! Deterministic structural matching for known-template TOML.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::ops::Range;

use crate::diagnostics::DiagnosticCode;
use crate::error::RecoveryHintKind;
use crate::frontmatter::parse_template_document;
use crate::types::VariableName;

use super::{
    ExtractError, ExtractRequest, ExtractionDiagnosticKind, ExtractionOccurrence, ExtractionReport,
    raw_text,
};

const MAX_TOML_INPUT_BYTES: usize = 1_048_576;
const MAX_TOML_NESTING_DEPTH: usize = 64;
const MAX_TOML_OCCURRENCES: usize = 10_000;

/// TOML table-key or array-index path evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TomlPathSegment {
    /// A TOML table or value key.
    TableKey {
        /// Table or value key.
        key: String,
    },
    /// A zero-based TOML array or array-of-table index.
    ArrayIndex {
        /// Array index.
        index: usize,
    },
}

/// TOML source evidence for a recovered string value.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TomlExtractionSource {
    /// The scalar was recovered from a TOML string value.
    StringValue,
}

/// TOML report alias over the generic extraction report.
pub type TomlExtractionReport = ExtractionReport<TomlPathSegment, TomlExtractionSource>;

#[derive(Clone, Debug)]
struct Capture {
    variable: VariableName,
    path: Vec<TomlPathSegment>,
    rendered_text: String,
}

pub(crate) fn extract_toml(
    request: &ExtractRequest<'_>,
) -> Result<TomlExtractionReport, ExtractError> {
    validate_input_size(request.template, "template")?;
    validate_input_size(request.rendered, "rendered TOML")?;
    let parsed_template = parse_template_document(request.template).map_err(|error| {
        template_error(format!("TOML template frontmatter is invalid: {error}"))
    })?;
    let template_source = parsed_template.body();
    validate_parse_depth(template_source)?;
    if template_source.contains("{%") || template_source.contains("{#") {
        return Err(template_error(
            "TOML extraction does not support Jinja statements or comments",
        ));
    }
    let template = parse_document(template_source, true)?;
    validate_parse_depth(request.rendered)?;
    let rendered = parse_document(request.rendered, false)?;
    validate_value_limits(&template, 0)?;
    validate_value_limits(&rendered, 0)?;
    let mut captures = Vec::new();
    match_toml(&template, &rendered, &[], &mut captures)?;

    let selected = captures
        .into_iter()
        .filter(|capture| selected_variable(&capture.variable, request))
        .collect::<Vec<_>>();
    let mut values = BTreeMap::new();
    let mut occurrences = Vec::new();
    for capture in selected {
        values
            .entry(capture.variable.clone())
            .or_insert_with(|| capture.rendered_text.clone());
        occurrences.push(ExtractionOccurrence {
            variable: capture.variable,
            path: capture.path,
            source: TomlExtractionSource::StringValue,
            rendered_text: Some(capture.rendered_text),
        });
    }
    TomlExtractionReport::new_with_ambiguity_code(
        values,
        occurrences,
        1.0,
        Vec::new(),
        DiagnosticCode::ErrExtractTomlAmbiguous,
    )
}

fn selected_variable(variable: &VariableName, request: &ExtractRequest<'_>) -> bool {
    (request.include.is_empty() || request.include.contains(variable))
        && !request.exclude.contains(variable)
}

fn match_toml(
    template: &toml::Value,
    rendered: &toml::Value,
    path: &[TomlPathSegment],
    captures: &mut Vec<Capture>,
) -> Result<(), ExtractError> {
    match (template, rendered) {
        (toml::Value::Table(template), toml::Value::Table(rendered)) => {
            for (key, template_value) in template {
                reject_dynamic_key(key, path)?;
                let rendered_value = rendered
                    .get(key)
                    .ok_or_else(|| missing_path_error(path, key))?;
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([TomlPathSegment::TableKey { key: key.clone() }])
                    .collect::<Vec<_>>();
                match_toml(template_value, rendered_value, &child_path, captures)?;
            }
            if rendered.keys().any(|key| !template.contains_key(key)) {
                return Err(shape_error(
                    path,
                    "TOML table keys do not match the known template",
                ));
            }
        }
        (toml::Value::Array(template), toml::Value::Array(rendered)) => {
            if template.len() != rendered.len() {
                return Err(shape_error(
                    path,
                    "TOML array length does not match the known template",
                ));
            }
            for (index, (template_value, rendered_value)) in
                template.iter().zip(rendered).enumerate()
            {
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([TomlPathSegment::ArrayIndex { index }])
                    .collect::<Vec<_>>();
                match_toml(template_value, rendered_value, &child_path, captures)?;
            }
        }
        (toml::Value::String(template), toml::Value::String(rendered)) => {
            let segments = raw_text::parse_raw_text_segments(template)
                .map_err(|error| map_raw_text_error(error, path))?;
            let matched = raw_text::match_raw_text(&raw_text::RawTextMatchInput {
                segments: &segments,
                rendered_candidate: rendered,
            })
            .map_err(|error| map_raw_text_error(error, path))?;
            if let Some(ambiguity) = matched.ambiguity {
                return Err(ambiguity_error(with_span(
                    &ambiguity.message,
                    ambiguity.span,
                )));
            }
            for capture in matched.captures {
                if captures.len() >= MAX_TOML_OCCURRENCES {
                    return Err(input_limit_error(format!(
                        "TOML extraction exceeded the maximum of {MAX_TOML_OCCURRENCES} occurrences"
                    )));
                }
                captures.push(Capture {
                    variable: capture.variable,
                    path: path.to_owned(),
                    rendered_text: capture.rendered_text,
                });
            }
        }
        (toml::Value::String(_), _) => {
            return Err(shape_error(path, "TOML string value changed type"));
        }
        (template, rendered) if template == rendered => {}
        (_, _) => {
            return Err(shape_error(
                path,
                "TOML value does not match the known template",
            ));
        }
    }
    Ok(())
}

fn validate_input_size(source: &str, label: &str) -> Result<(), ExtractError> {
    if source.len() > MAX_TOML_INPUT_BYTES {
        return Err(input_limit_error(format!(
            "TOML {label} input is {} bytes; maximum is {MAX_TOML_INPUT_BYTES} bytes",
            source.len()
        )));
    }
    Ok(())
}

fn validate_value_limits(value: &toml::Value, depth: usize) -> Result<(), ExtractError> {
    if depth > MAX_TOML_NESTING_DEPTH {
        return Err(input_limit_error(format!(
            "TOML nesting depth exceeds the maximum of {MAX_TOML_NESTING_DEPTH}"
        )));
    }
    match value {
        toml::Value::Array(values) => {
            for value in values {
                validate_value_limits(value, depth + 1)?;
            }
        }
        toml::Value::Table(values) => {
            for value in values.values() {
                validate_value_limits(value, depth + 1)?;
            }
        }
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => {}
    }
    Ok(())
}

fn reject_dynamic_key(key: &str, path: &[TomlPathSegment]) -> Result<(), ExtractError> {
    let segments =
        raw_text::parse_raw_text_segments(key).map_err(|error| map_raw_text_error(error, path))?;
    if segments
        .iter()
        .any(|segment| matches!(segment, raw_text::RawTextSegment::Variable(_)))
    {
        return Err(value_error(
            "TOML table and value keys must be static; placeholders in keys are unsupported",
        ));
    }
    Ok(())
}

fn parse_document(source: &str, template: bool) -> Result<toml::Value, ExtractError> {
    toml::from_str::<toml::Value>(source).map_err(|error| {
        let message = error.to_string();
        if message.contains("duplicate key") {
            duplicate_error_with_source(message, error)
        } else if template && source.contains("{{") {
            value_error_with_source(
                "TOML placeholders are only supported in basic or literal string values",
                error,
            )
        } else if template {
            template_error_with_source(
                format!("TOML template is not a supported known document: {message}"),
                error,
            )
        } else {
            malformed_error_with_source(
                format!("TOML parser rejected rendered input: {message}"),
                error,
            )
        }
    })
}

fn validate_parse_depth(source: &str) -> Result<(), ExtractError> {
    let mut depth = 0;
    let mut quote: Option<(u8, bool)> = None;
    let mut comment = false;
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if comment {
            if byte == b'\n' {
                comment = false;
            }
            index += 1;
            continue;
        }
        if let Some((delimiter, multiline)) = quote {
            if multiline && bytes.get(index..index + 3) == Some(&[delimiter; 3]) {
                quote = None;
                index += 3;
            } else if !multiline && delimiter == b'"' && byte == b'\\' {
                index = index.saturating_add(2).min(bytes.len());
            } else if byte == delimiter {
                quote = None;
                index += 1;
            } else {
                index += 1;
            }
            continue;
        }
        match byte {
            b'"' | b'\'' => {
                let multiline = bytes.get(index..index + 3) == Some(&[byte; 3]);
                quote = Some((byte, multiline));
                index += if multiline { 3 } else { 1 };
            }
            b'#' => {
                comment = true;
                index += 1;
            }
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_TOML_NESTING_DEPTH {
                    return Err(input_limit_error(format!(
                        "TOML nesting depth exceeds the maximum of {MAX_TOML_NESTING_DEPTH}"
                    )));
                }
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            _ => index += 1,
        }
    }
    Ok(())
}

fn map_raw_text_error(
    error: raw_text::RawTextMatchError,
    path: &[TomlPathSegment],
) -> ExtractError {
    match error.scope() {
        raw_text::RawTextErrorScope::Request => match error {
            raw_text::RawTextMatchError::InvalidTemplate { span, message }
            | raw_text::RawTextMatchError::StaticMismatch { span, message }
            | raw_text::RawTextMatchError::AmbiguousDelimiter { span, message } => {
                template_error(with_span(&message, span))
            }
        },
        raw_text::RawTextErrorScope::Occurrence => match error {
            raw_text::RawTextMatchError::InvalidTemplate { span, message } => {
                template_error(with_span(&message, span))
            }
            raw_text::RawTextMatchError::StaticMismatch { span, message } => {
                shape_error(path, with_span(&message, span))
            }
            raw_text::RawTextMatchError::AmbiguousDelimiter { span, message } => {
                ambiguity_error(with_span(&message, span))
            }
        },
    }
}

fn with_span(message: &str, span: Option<Range<usize>>) -> String {
    span.map_or_else(
        || message.to_owned(),
        |span| format!("{message} (candidate bytes {}..{})", span.start, span.end),
    )
}

fn template_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractTemplateUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "use known-template TOML string values with scalar placeholders"
                .to_owned(),
        },
    )
}

fn template_error_with_source(
    message: impl Into<String>,
    source: impl std::error::Error,
) -> ExtractError {
    ExtractError::format_error_with_source(
        DiagnosticCode::ErrExtractTemplateUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "use known-template TOML string values with scalar placeholders"
                .to_owned(),
        },
        source,
    )
}

fn malformed_error_with_source(
    message: impl Into<String>,
    source: impl std::error::Error,
) -> ExtractError {
    ExtractError::format_error_with_source(
        DiagnosticCode::ErrExtractTomlMalformed,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "inspect the rendered TOML for one well-formed document".to_owned(),
        },
        source,
    )
}

fn input_limit_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractInputLimit,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "reduce TOML input size, nesting depth, or occurrence count".to_owned(),
        },
    )
}

fn duplicate_error_with_source(
    message: impl Into<String>,
    source: impl std::error::Error,
) -> ExtractError {
    ExtractError::format_error_with_source(
        DiagnosticCode::ErrExtractTomlDuplicateKey,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "remove duplicate TOML keys or table declarations".to_owned(),
        },
        source,
    )
}

fn value_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractTomlValueUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "put placeholders in TOML basic or literal string values rather than keys or typed values"
                .to_owned(),
        },
    )
}

fn value_error_with_source(
    message: impl Into<String>,
    source: impl std::error::Error,
) -> ExtractError {
    ExtractError::format_error_with_source(
        DiagnosticCode::ErrExtractTomlValueUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "put placeholders in TOML basic or literal string values rather than keys or typed values"
                .to_owned(),
        },
        source,
    )
}

fn shape_error(path: &[TomlPathSegment], message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractTomlShapeMismatch,
        ExtractionDiagnosticKind::Unsupported,
        format!("{} at {}", message.into(), format_path(path)),
        RecoveryHintKind::InspectInput {
            description: "restore the rendered TOML table, array, and static-value shape"
                .to_owned(),
        },
    )
}

fn missing_path_error(path: &[TomlPathSegment], key: &str) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractTomlPathMissing,
        ExtractionDiagnosticKind::NotObserved,
        format!("TOML path {} is missing table key {key}", format_path(path)),
        RecoveryHintKind::InspectInput {
            description: "render the table or value key required by the known TOML template"
                .to_owned(),
        },
    )
}

fn ambiguity_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractTomlAmbiguous,
        ExtractionDiagnosticKind::Ambiguous,
        message,
        RecoveryHintKind::DisambiguateOccurrences {
            description: "add static delimiters or use distinct TOML occurrence paths".to_owned(),
        },
    )
}

fn format_path(path: &[TomlPathSegment]) -> String {
    let mut result = String::from("$");
    for segment in path {
        match segment {
            TomlPathSegment::TableKey { key } => {
                let _ = write!(result, ".{key}");
            }
            TomlPathSegment::ArrayIndex { index } => {
                let _ = write!(result, "[{index}]");
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::extract_toml;
    use crate::extract::{ExtractFormat, ExtractRequest};
    use crate::types::VariableName;

    #[test]
    fn reports_occurrence_ambiguity_for_repeated_variable_paths() {
        let request = ExtractRequest::new(
            "[first]\nvalue = \"{{ value }}\"\n[second]\nvalue = \"{{ value }}\"\n",
            "[first]\nvalue = \"Ada\"\n[second]\nvalue = \"Ada\"\n",
            ExtractFormat::Toml,
            &[],
            &[],
        );
        let report = extract_toml(&request).unwrap();

        assert!(report.values.is_empty());
        assert_eq!(report.occurrences.len(), 2);
        assert!(
            report
                .occurrences
                .iter()
                .all(|occurrence| { occurrence.variable == VariableName::new("value").unwrap() })
        );
        assert_eq!(report.diagnostics.len(), 2);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::diagnostics::DiagnosticCode::ErrExtractTomlAmbiguous
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::diagnostics::DiagnosticCode::WarnExtractLowConfidence
        }));
    }
}
