//! Deterministic structural matching for known-template JSON.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::de::{self, DeserializeSeed, Deserializer, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::diagnostics::DiagnosticCode;
use crate::error::RecoveryHintKind;
use crate::frontmatter::parse_template_document;
use crate::types::VariableName;

use super::{
    ExtractError, ExtractRequest, ExtractionDiagnosticKind, ExtractionOccurrence, ExtractionReport,
    raw_text,
};

#[path = "json_limits.rs"]
mod limits;

pub(super) const MAX_JSON_INPUT_BYTES: usize = 1_048_576;
pub(super) const MAX_JSON_NESTING_DEPTH: usize = 64;
const MAX_JSON_OCCURRENCES: usize = 10_000;

/// JSON object-key or array-index path evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonPathSegment {
    /// A JSON object key.
    ObjectKey {
        /// Object key.
        key: String,
    },
    /// A zero-based JSON array index.
    ArrayIndex {
        /// Array index.
        index: usize,
    },
}

/// JSON source evidence for a recovered string value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonExtractionSource {
    /// The scalar was recovered from a JSON string value.
    StringValue,
}

/// JSON report alias over the generic extraction report.
pub type JsonExtractionReport = ExtractionReport<JsonPathSegment, JsonExtractionSource>;

#[derive(Clone, Debug)]
struct Capture {
    variable: VariableName,
    path: Vec<JsonPathSegment>,
    rendered_text: String,
}

#[derive(Default)]
struct Evidence {
    compared_values: usize,
}

pub(crate) fn extract_json(
    request: &ExtractRequest<'_>,
) -> Result<JsonExtractionReport, ExtractError> {
    limits::validate_input_size(request.template, "template")?;
    limits::validate_input_size(request.rendered, "rendered JSON")?;
    let parsed_template = parse_template_document(request.template).map_err(|error| {
        template_error(format!("JSON template frontmatter is invalid: {error}"))
    })?;
    let template_source = parsed_template.body();
    limits::validate_parse_depth(template_source)?;
    if template_source.contains("{%") || template_source.contains("{#") {
        return Err(template_error(
            "JSON extraction does not support Jinja statements or comments",
        ));
    }
    let template = parse_document(template_source, true)?;
    limits::validate_parse_depth(request.rendered)?;
    let rendered = parse_document(request.rendered, false)?;
    limits::validate_value_limits(&template, 0)?;
    limits::validate_value_limits(&rendered, 0)?;
    let mut captures = Vec::new();
    let mut evidence = Evidence::default();
    match_json(&template, &rendered, &[], &mut captures, &mut evidence)?;

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
            source: JsonExtractionSource::StringValue,
            rendered_text: Some(capture.rendered_text),
        });
    }
    let confidence = if evidence.compared_values == 0 {
        0.0
    } else {
        1.0
    };
    JsonExtractionReport::new_with_ambiguity_code(
        values,
        occurrences,
        confidence,
        Vec::new(),
        DiagnosticCode::ErrExtractJsonAmbiguous,
    )
}

fn selected_variable(variable: &VariableName, request: &ExtractRequest<'_>) -> bool {
    (request.include.is_empty() || request.include.contains(variable))
        && !request.exclude.contains(variable)
}

fn match_json(
    template: &serde_json::Value,
    rendered: &serde_json::Value,
    path: &[JsonPathSegment],
    captures: &mut Vec<Capture>,
    evidence: &mut Evidence,
) -> Result<(), ExtractError> {
    match (template, rendered) {
        (serde_json::Value::Object(template), serde_json::Value::Object(rendered)) => {
            for (key, template_value) in template {
                reject_dynamic_key(key, path)?;
                let rendered_value = rendered
                    .get(key)
                    .ok_or_else(|| missing_path_error(path, key))?;
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([JsonPathSegment::ObjectKey { key: key.clone() }])
                    .collect::<Vec<_>>();
                match_json(
                    template_value,
                    rendered_value,
                    &child_path,
                    captures,
                    evidence,
                )?;
            }
            if rendered.keys().any(|key| !template.contains_key(key)) {
                return Err(shape_error(
                    path,
                    "JSON object keys do not match the known template",
                ));
            }
        }
        (serde_json::Value::Array(template), serde_json::Value::Array(rendered)) => {
            if template.len() != rendered.len() {
                return Err(shape_error(
                    path,
                    "JSON array length does not match the known template",
                ));
            }
            for (index, (template_value, rendered_value)) in
                template.iter().zip(rendered).enumerate()
            {
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([JsonPathSegment::ArrayIndex { index }])
                    .collect::<Vec<_>>();
                match_json(
                    template_value,
                    rendered_value,
                    &child_path,
                    captures,
                    evidence,
                )?;
            }
        }
        (serde_json::Value::String(template), serde_json::Value::String(rendered)) => {
            let segments = raw_text::parse_raw_text_segments(template)
                .map_err(|error| map_raw_text_error(error, path))?;
            let matched = raw_text::match_raw_text(&raw_text::RawTextMatchInput {
                segments: &segments,
                rendered_candidate: rendered,
            })
            .map_err(|error| map_raw_text_error(error, path))?;
            evidence.compared_values += 1;
            if let Some(ambiguity) = matched.ambiguity {
                return Err(ambiguity_error(raw_text::format_diagnostic_message(
                    &ambiguity.message,
                    ambiguity.span,
                )));
            }
            for capture in matched.captures {
                if captures.len() >= MAX_JSON_OCCURRENCES {
                    return Err(input_limit_error(format!(
                        "JSON extraction exceeded the maximum of {MAX_JSON_OCCURRENCES} occurrences"
                    )));
                }
                debug_assert_eq!(&rendered[capture.span.clone()], capture.rendered_text);
                captures.push(Capture {
                    variable: capture.variable,
                    path: path.to_owned(),
                    rendered_text: capture.rendered_text,
                });
            }
        }
        (serde_json::Value::String(_), _) => {
            return Err(shape_error(path, "JSON string value changed type"));
        }
        (template, rendered) if template == rendered => {
            evidence.compared_values += 1;
        }
        (serde_json::Value::Null, _) => {
            return Err(shape_error(
                path,
                "JSON null value does not match the known template",
            ));
        }
        (_, _) => {
            return Err(shape_error(
                path,
                "JSON value does not match the known template",
            ));
        }
    }
    Ok(())
}

fn reject_dynamic_key(key: &str, path: &[JsonPathSegment]) -> Result<(), ExtractError> {
    let segments =
        raw_text::parse_raw_text_segments(key).map_err(|error| map_raw_text_error(error, path))?;
    if segments
        .iter()
        .any(|segment| matches!(segment, raw_text::RawTextSegment::Variable(_)))
    {
        return Err(value_error(
            "JSON object keys must be static strings; placeholders in keys are unsupported",
        ));
    }
    Ok(())
}

fn parse_document(source: &str, template: bool) -> Result<serde_json::Value, ExtractError> {
    let mut deserializer = serde_json::Deserializer::from_str(source);
    let value = StrictJsonValue::deserialize(&mut deserializer).map_err(|error| {
        let message = error.to_string();
        if message.starts_with("JSON nesting depth exceeds the maximum") {
            input_limit_error(message)
        } else if message.starts_with("duplicate JSON object key") {
            duplicate_error_with_source(message, error)
        } else if template && source.contains("{{") {
            value_error_with_source(
                "JSON placeholders are only supported in string values",
                error,
            )
        } else if template {
            template_error_with_source(
                format!("JSON template is not a supported known document: {message}"),
                error,
            )
        } else {
            malformed_error_with_source(
                format!("JSON parser rejected rendered input: {message}"),
                error,
            )
        }
    })?;
    deserializer.end().map_err(|error| {
        if template {
            template_error_with_source(
                format!("JSON template contains trailing input: {error}"),
                error,
            )
        } else {
            malformed_error_with_source(
                format!("JSON rendered input contains trailing input: {error}"),
                error,
            )
        }
    })?;
    Ok(value.0)
}

fn map_raw_text_error(
    error: raw_text::RawTextMatchError,
    path: &[JsonPathSegment],
) -> ExtractError {
    match error {
        raw_text::RawTextMatchError::InvalidTemplate { span, message } => {
            template_error(raw_text::format_diagnostic_message(&message, span))
        }
        raw_text::RawTextMatchError::StaticMismatch { span, message } => {
            shape_error(path, raw_text::format_diagnostic_message(&message, span))
        }
        raw_text::RawTextMatchError::AmbiguousDelimiter { span, message } => {
            ambiguity_error(raw_text::format_diagnostic_message(&message, span))
        }
    }
}

fn template_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractTemplateUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "use known-template JSON string values with scalar placeholders"
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
            description: "use known-template JSON string values with scalar placeholders"
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
        DiagnosticCode::ErrExtractJsonMalformed,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "inspect the rendered JSON for one well-formed value".to_owned(),
        },
        source,
    )
}

pub(super) fn input_limit_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractInputLimit,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "reduce JSON input size, nesting depth, or occurrence count".to_owned(),
        },
    )
}

fn duplicate_error_with_source(
    message: impl Into<String>,
    source: impl std::error::Error,
) -> ExtractError {
    ExtractError::format_error_with_source(
        DiagnosticCode::ErrExtractJsonDuplicateKey,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "remove duplicate JSON object keys".to_owned(),
        },
        source,
    )
}

fn value_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractJsonValueUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "put placeholders in JSON string values rather than keys or typed values"
                .to_owned(),
        },
    )
}

fn value_error_with_source(
    message: impl Into<String>,
    source: impl std::error::Error,
) -> ExtractError {
    ExtractError::format_error_with_source(
        DiagnosticCode::ErrExtractJsonValueUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "put placeholders in JSON string values rather than keys or typed values"
                .to_owned(),
        },
        source,
    )
}

fn shape_error(path: &[JsonPathSegment], message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractJsonShapeMismatch,
        ExtractionDiagnosticKind::Unsupported,
        format!("{} at {}", message.into(), format_path(path)),
        RecoveryHintKind::InspectInput {
            description: "restore the rendered JSON object, array, and static-value shape"
                .to_owned(),
        },
    )
}

fn missing_path_error(path: &[JsonPathSegment], key: &str) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractJsonPathMissing,
        ExtractionDiagnosticKind::NotObserved,
        format!(
            "JSON path {} is missing object key {key}",
            format_path(path)
        ),
        RecoveryHintKind::InspectInput {
            description: "render the object key required by the known JSON template".to_owned(),
        },
    )
}

fn ambiguity_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractJsonAmbiguous,
        ExtractionDiagnosticKind::Ambiguous,
        message,
        RecoveryHintKind::DisambiguateOccurrences {
            description: "add static delimiters or use distinct JSON occurrence paths".to_owned(),
        },
    )
}

fn format_path(path: &[JsonPathSegment]) -> String {
    let mut result = String::from("$");
    for segment in path {
        match segment {
            JsonPathSegment::ObjectKey { key } => {
                let _ = write!(result, ".{key}");
            }
            JsonPathSegment::ArrayIndex { index } => {
                let _ = write!(result, "[{index}]");
            }
        }
    }
    result
}

#[derive(Clone, Debug)]
struct StrictJsonValue(serde_json::Value);

impl<'de> Deserialize<'de> for StrictJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize_at(deserializer, 0)
    }
}

struct StrictJsonSeed {
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictJsonSeed {
    type Value = StrictJsonValue;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictJsonValue::deserialize_at(deserializer, self.depth)
    }
}

impl StrictJsonValue {
    fn deserialize_at<'de, D>(deserializer: D, depth: usize) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StrictVisitor {
            depth: usize,
        }

        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictJsonValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("one JSON value without duplicate object keys")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJsonValue(serde_json::Value::Bool(v)))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJsonValue(serde_json::Value::Number(v.into())))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJsonValue(serde_json::Value::Number(v.into())))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let number = serde_json::Number::from_f64(v)
                    .ok_or_else(|| E::custom("JSON number is not finite"))?;
                Ok(StrictJsonValue(serde_json::Value::Number(number)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJsonValue(serde_json::Value::String(v.to_owned())))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJsonValue(serde_json::Value::String(v)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(StrictJsonValue(serde_json::Value::Null))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                if self.depth > MAX_JSON_NESTING_DEPTH {
                    return Err(de::Error::custom(format!(
                        "JSON nesting depth exceeds the maximum of {MAX_JSON_NESTING_DEPTH}"
                    )));
                }
                let mut values = Vec::new();
                while let Some(value) = seq.next_element_seed(StrictJsonSeed {
                    depth: self.depth + 1,
                })? {
                    values.push(value.0);
                }
                Ok(StrictJsonValue(serde_json::Value::Array(values)))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                if self.depth > MAX_JSON_NESTING_DEPTH {
                    return Err(de::Error::custom(format!(
                        "JSON nesting depth exceeds the maximum of {MAX_JSON_NESTING_DEPTH}"
                    )));
                }
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!(
                            "duplicate JSON object key: {key}"
                        )));
                    }
                    let value = map.next_value_seed(StrictJsonSeed {
                        depth: self.depth + 1,
                    })?;
                    values.insert(key, value.0);
                }
                Ok(StrictJsonValue(serde_json::Value::Object(values)))
            }
        }

        if depth > MAX_JSON_NESTING_DEPTH {
            return Err(D::Error::custom(format!(
                "JSON nesting depth exceeds the maximum of {MAX_JSON_NESTING_DEPTH}"
            )));
        }
        deserializer.deserialize_any(StrictVisitor { depth })
    }
}
