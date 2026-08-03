//! Deterministic structural matching for known-template YAML.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::ops::Range;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::diagnostics::DiagnosticCode;
use crate::error::RecoveryHintKind;
use crate::frontmatter::parse_template_document;
use crate::types::VariableName;

use super::{
    ExtractError, ExtractRequest, ExtractionDiagnosticKind, ExtractionOccurrence, ExtractionReport,
    raw_text,
};

/// YAML mapping-key or sequence-index path evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum YamlPathSegment {
    /// A YAML mapping key.
    MappingKey {
        /// Mapping key.
        key: String,
    },
    /// A zero-based YAML sequence index.
    SequenceIndex {
        /// Sequence index.
        index: usize,
    },
}

/// YAML source evidence for a recovered string value.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum YamlExtractionSource {
    /// The scalar was recovered from a YAML string value.
    StringScalar,
}

/// YAML report alias over the generic extraction report.
pub type YamlExtractionReport = ExtractionReport<YamlPathSegment, YamlExtractionSource>;

#[derive(Clone, Debug, PartialEq)]
enum YamlNode {
    Mapping(Vec<(String, YamlNode)>),
    Sequence(Vec<YamlNode>),
    String(String),
    Other(YamlScalar),
}

#[derive(Clone, Debug, PartialEq)]
enum YamlScalar {
    Bool(bool),
    Number(String),
    Null,
}

struct YamlKey(String);

impl<'de> Deserialize<'de> for YamlKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor;

        impl Visitor<'_> for KeyVisitor {
            type Value = YamlKey;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a YAML string mapping key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlKey(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlKey(v))
            }
        }

        deserializer.deserialize_any(KeyVisitor)
    }
}

impl<'de> Deserialize<'de> for YamlNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NodeVisitor;

        impl<'de> Visitor<'de> for NodeVisitor {
            type Value = YamlNode;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a YAML document value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                let mut keys = HashSet::new();
                while let Some(YamlKey(key)) = map.next_key::<YamlKey>()? {
                    if !keys.insert(key.clone()) {
                        return Err(de::Error::custom(format!(
                            "duplicate YAML mapping key: {key}"
                        )));
                    }
                    entries.push((key, map.next_value()?));
                }
                Ok(YamlNode::Mapping(entries))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(YamlNode::Sequence(values))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlNode::String(v.to_owned()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlNode::String(v))
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlNode::Other(YamlScalar::Bool(v)))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlNode::Other(YamlScalar::Number(v.to_string())))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlNode::Other(YamlScalar::Number(v.to_string())))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlNode::Other(YamlScalar::Number(v.to_string())))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(YamlNode::Other(YamlScalar::Null))
            }
        }

        deserializer.deserialize_any(NodeVisitor)
    }
}

#[derive(Clone, Debug)]
struct Capture {
    variable: VariableName,
    path: Vec<YamlPathSegment>,
    rendered_text: String,
}

pub(crate) fn extract_yaml(
    request: &ExtractRequest<'_>,
) -> Result<YamlExtractionReport, ExtractError> {
    let parsed_template = parse_template_document(request.template).map_err(|error| {
        template_error(format!("YAML template frontmatter is invalid: {error}"))
    })?;
    let template_source = parsed_template.body();
    if template_source.contains("{%") || template_source.contains("{#") {
        return Err(template_error(
            "YAML extraction does not support Jinja statements or comments",
        ));
    }
    if contains_yaml_features(template_source) {
        return Err(alias_error(
            "YAML anchors, aliases, or tags are unsupported in known templates",
        ));
    }
    let template = parse_document(template_source, true)?;
    let rendered = parse_document(request.rendered, false)?;
    let mut captures = Vec::new();
    match_yaml(&template, &rendered, &[], &mut captures)?;

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
            source: YamlExtractionSource::StringScalar,
            rendered_text: Some(capture.rendered_text),
        });
    }
    YamlExtractionReport::new_with_ambiguity_code(
        values,
        occurrences,
        1.0,
        Vec::new(),
        DiagnosticCode::ErrExtractYamlAmbiguous,
    )
}

fn selected_variable(variable: &VariableName, request: &ExtractRequest<'_>) -> bool {
    (request.include.is_empty() || request.include.contains(variable))
        && !request.exclude.contains(variable)
}

fn match_yaml(
    template: &YamlNode,
    rendered: &YamlNode,
    path: &[YamlPathSegment],
    captures: &mut Vec<Capture>,
) -> Result<(), ExtractError> {
    match (template, rendered) {
        (YamlNode::Mapping(template), YamlNode::Mapping(rendered)) => {
            for (key, template_value) in template {
                reject_dynamic_key(key)?;
                let rendered_value = rendered
                    .iter()
                    .find(|(rendered_key, _)| rendered_key == key)
                    .map(|(_, value)| value)
                    .ok_or_else(|| missing_path_error(path, key))?;
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([YamlPathSegment::MappingKey { key: key.clone() }])
                    .collect::<Vec<_>>();
                match_yaml(template_value, rendered_value, &child_path, captures)?;
            }
            if rendered
                .iter()
                .any(|(key, _)| !template.iter().any(|(expected, _)| expected == key))
            {
                return Err(shape_error(
                    path,
                    "YAML mapping keys do not match the known template",
                ));
            }
        }
        (YamlNode::Sequence(template), YamlNode::Sequence(rendered)) => {
            if template.len() != rendered.len() {
                return Err(shape_error(
                    path,
                    "YAML sequence length does not match the known template",
                ));
            }
            for (index, (template_value, rendered_value)) in
                template.iter().zip(rendered).enumerate()
            {
                let child_path = path
                    .iter()
                    .cloned()
                    .chain([YamlPathSegment::SequenceIndex { index }])
                    .collect::<Vec<_>>();
                match_yaml(template_value, rendered_value, &child_path, captures)?;
            }
        }
        (YamlNode::String(template), YamlNode::String(rendered)) => {
            let segments =
                raw_text::parse_raw_text_segments(template).map_err(map_raw_text_error)?;
            let matched = raw_text::match_raw_text(&raw_text::RawTextMatchInput {
                segments: &segments,
                rendered_candidate: rendered,
            })
            .map_err(map_raw_text_error)?;
            if let Some(ambiguity) = matched.ambiguity {
                return Err(ambiguity_error(with_span(
                    &ambiguity.message,
                    ambiguity.span,
                )));
            }
            for capture in matched.captures {
                captures.push(Capture {
                    variable: capture.variable,
                    path: path.to_owned(),
                    rendered_text: capture.rendered_text,
                });
            }
        }
        (YamlNode::String(_), YamlNode::Other(_)) => {
            return Err(value_error(
                "YAML placeholders are supported only in string scalar values",
            ));
        }
        (YamlNode::String(_), _) => {
            return Err(shape_error(path, "YAML string scalar changed type"));
        }
        (template, rendered) if template == rendered => {}
        (YamlNode::Other(YamlScalar::Null), _) => {
            return Err(shape_error(
                path,
                "YAML null value does not match the known template",
            ));
        }
        (_, _) => {
            return Err(shape_error(
                path,
                "YAML value does not match the known template",
            ));
        }
    }
    Ok(())
}

fn parse_document(source: &str, template: bool) -> Result<YamlNode, ExtractError> {
    if contains_yaml_features(source) {
        return Err(alias_error(
            "YAML anchors, aliases, or tags are unsupported in extraction input",
        ));
    }
    let mut documents = serde_yaml::Deserializer::from_str(source);
    let Some(document) = documents.next() else {
        return Err(malformed_error("YAML input must contain one document"));
    };
    let node = YamlNode::deserialize(document).map_err(|error| {
        let message = error.to_string();
        if message.starts_with("duplicate YAML mapping key") {
            duplicate_error(message)
        } else if template {
            template_error(format!(
                "YAML template is not a supported known document: {message}"
            ))
        } else {
            malformed_error(format!("YAML parser rejected rendered input: {message}"))
        }
    })?;
    if documents.next().is_some() {
        return Err(document_stream_error());
    }
    Ok(node)
}

fn contains_yaml_features(source: &str) -> bool {
    let mut single = false;
    let mut double = false;
    let bytes = source.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        match *byte {
            b'\'' if !double => single = !single,
            b'"' if !single => double = !double,
            b'&' | b'*' | b'!' if !single && !double => {
                if index > 0 && !bytes[index - 1].is_ascii_whitespace() {
                    continue;
                }
                if bytes.get(index + 1).is_some_and(|next| {
                    next.is_ascii_alphanumeric() || *next == b'_' || *next == b'-'
                }) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn reject_dynamic_key(key: &str) -> Result<(), ExtractError> {
    let segments = raw_text::parse_raw_text_segments(key).map_err(map_raw_text_error)?;
    if segments
        .iter()
        .any(|segment| matches!(segment, raw_text::RawTextSegment::Variable(_)))
    {
        return Err(value_error(
            "YAML mapping keys must be static strings; placeholders in keys are unsupported",
        ));
    }
    Ok(())
}

fn map_raw_text_error(error: raw_text::RawTextMatchError) -> ExtractError {
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
                shape_error(&[], with_span(&message, span))
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
            description: "use known-template YAML string scalars with scalar placeholders"
                .to_owned(),
        },
    )
}

fn malformed_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlMalformed,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "inspect the rendered YAML for one well-formed document".to_owned(),
        },
    )
}

fn duplicate_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlDuplicateKey,
        ExtractionDiagnosticKind::Malformed,
        message,
        RecoveryHintKind::InspectInput {
            description: "remove duplicate YAML mapping keys".to_owned(),
        },
    )
}

fn alias_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlAliasUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description: "expand YAML anchors and aliases into explicit content".to_owned(),
        },
    )
}

fn document_stream_error() -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlDocumentStream,
        ExtractionDiagnosticKind::Unsupported,
        "YAML document streams are unsupported; provide exactly one document",
        RecoveryHintKind::InspectInput {
            description: "remove additional YAML documents from the rendered input".to_owned(),
        },
    )
}

fn value_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlValueUnsupported,
        ExtractionDiagnosticKind::Unsupported,
        message,
        RecoveryHintKind::UnsupportedConstruct {
            description:
                "put placeholders in YAML string scalar values rather than keys or typed values"
                    .to_owned(),
        },
    )
}

fn shape_error(path: &[YamlPathSegment], message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlShapeMismatch,
        ExtractionDiagnosticKind::Unsupported,
        format!("{} at {}", message.into(), format_path(path)),
        RecoveryHintKind::InspectInput {
            description: "restore the rendered YAML mapping, sequence, and static-value shape"
                .to_owned(),
        },
    )
}

fn missing_path_error(path: &[YamlPathSegment], key: &str) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlPathMissing,
        ExtractionDiagnosticKind::NotObserved,
        format!(
            "YAML path {} is missing mapping key {key}",
            format_path(path)
        ),
        RecoveryHintKind::InspectInput {
            description: "render the mapping key required by the known YAML template".to_owned(),
        },
    )
}

fn ambiguity_error(message: impl Into<String>) -> ExtractError {
    ExtractError::format_error(
        DiagnosticCode::ErrExtractYamlAmbiguous,
        ExtractionDiagnosticKind::Ambiguous,
        message,
        RecoveryHintKind::DisambiguateOccurrences {
            description: "add static delimiters or use distinct YAML occurrence paths".to_owned(),
        },
    )
}

fn format_path(path: &[YamlPathSegment]) -> String {
    let mut result = String::from("$");
    for segment in path {
        match segment {
            YamlPathSegment::MappingKey { key } => {
                result.push('.');
                result.push_str(key);
            }
            YamlPathSegment::SequenceIndex { index } => {
                result.push('[');
                result.push_str(&index.to_string());
                result.push(']');
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::extract_yaml;
    use crate::extract::{ExtractFormat, ExtractRequest};
    use crate::types::VariableName;

    #[test]
    fn reports_occurrence_ambiguity_for_repeated_variable_paths() {
        let request = ExtractRequest::new(
            "first: \"{{ value }}\"\nsecond: \"{{ value }}\"\n",
            "first: Ada\nsecond: Ada\n",
            ExtractFormat::Yaml,
            &[],
            &[],
        );
        let report = extract_yaml(&request).unwrap();

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
            diagnostic.code == crate::diagnostics::DiagnosticCode::ErrExtractYamlAmbiguous
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::diagnostics::DiagnosticCode::WarnExtractLowConfidence
        }));
    }
}
