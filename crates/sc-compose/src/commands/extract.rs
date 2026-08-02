use std::fmt::Write as _;
use std::path::Path;

use anyhow::anyhow;
use sc_composer::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, ExtractError, ExtractionDiagnostic,
    ExtractionDiagnosticKind, ExtractionPathSegment, ExtractionReport, ExtractionSource,
    RecoveryHint, RecoveryHintKind, XmlExtractionSource, XmlPathSegment,
};

use crate::cli::ExtractArgs;
use crate::path_utils::to_forward_slash;
use crate::{CommandError, print_json};

pub(crate) fn run_extract(args: &ExtractArgs) -> Result<i32, CommandError> {
    let template = read_input(&args.template, "template")?;
    let rendered = read_input(&args.rendered, "rendered output")?;
    let include = parse_filter(&args.include, "include")?;
    let exclude = parse_filter(&args.exclude, "exclude")?;
    let request = sc_composer::ExtractRequest::new(
        &template,
        &rendered,
        sc_composer::ExtractFormat::Xml,
        &include,
        &exclude,
    );
    let report = sc_composer::extract(&request).map_err(|error| extract_error(&error))?;

    if args.json {
        emit_json(args, &report).map_err(json_output_error)?;
    } else {
        emit_text(args, &report);
    }

    Ok(crate::exit_codes::SUCCESS)
}

fn read_input(path: &Path, label: &str) -> Result<String, CommandError> {
    std::fs::read_to_string(path).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!(error).context(format!("failed to read {label} file {}", path.display())),
            DiagnosticCode::ErrConfigRead,
            vec![RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: path.to_owned(),
            })],
        )
    })
}

fn parse_filter(
    names: &[String],
    filter_name: &str,
) -> Result<Vec<sc_composer::VariableName>, CommandError> {
    names
        .iter()
        .map(|name| {
            sc_composer::VariableName::new(name).map_err(|error| {
                CommandError::usage_with_code_and_hints(
                    anyhow!("invalid {filter_name} variable `{name}`: {error}"),
                    DiagnosticCode::ErrExtractInvalidRequest,
                    vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                        key: format!("use a valid {filter_name} variable name"),
                    })],
                )
            })
        })
        .collect()
}

fn extract_error(error: &ExtractError) -> CommandError {
    let code = error.code();
    let message = extract_error_message(error);
    CommandError {
        exit_code: match error {
            ExtractError::InvalidRequest { .. } => crate::exit_codes::USAGE_FAIL,
            ExtractError::MalformedXml { .. }
            | ExtractError::UnsupportedSyntax { .. }
            | ExtractError::AmbiguousStructure { .. }
            | ExtractError::FormatError { .. } => crate::exit_codes::VALIDATION_OR_RENDER_FAIL,
        },
        diagnostic_code: Some(code),
        diagnostics: vec![Diagnostic::new(DiagnosticSeverity::Error, code, &message)],
        recovery_hints: error.recovery_hints().to_vec(),
        error: anyhow!(message),
    }
}

fn json_output_error(error: anyhow::Error) -> CommandError {
    CommandError::usage_with_code_and_hints(
        error.context("failed to write JSON output"),
        DiagnosticCode::ErrRenderWrite,
        vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
            key: "JSON output destination (stdout)".to_owned(),
        })],
    )
}

fn extract_error_message(error: &ExtractError) -> String {
    match error {
        ExtractError::InvalidRequest { message, .. } => message.clone(),
        ExtractError::MalformedXml { diagnostic, .. }
        | ExtractError::UnsupportedSyntax { diagnostic, .. }
        | ExtractError::AmbiguousStructure { diagnostic, .. }
        | ExtractError::FormatError { diagnostic, .. } => diagnostic.message.clone(),
    }
}

fn emit_json(
    args: &ExtractArgs,
    report: &ExtractionReport<ExtractionPathSegment, ExtractionSource>,
) -> anyhow::Result<()> {
    let warnings = report
        .diagnostics
        .iter()
        .map(extraction_diagnostic_json)
        .collect::<Vec<_>>();
    print_json(
        serde_json::json!({
            "template": to_forward_slash(&args.template),
            "rendered": to_forward_slash(&args.rendered),
            "format": "xml",
            "values": report
                .values
                .iter()
                .map(|(name, value)| (name.to_string(), value.clone()))
                .collect::<std::collections::BTreeMap<_, _>>(),
            "occurrences": report.occurrences.iter().map(extraction_occurrence_json).collect::<Vec<_>>(),
            "confidence": report.confidence,
            "warnings": warnings,
        }),
        report
            .diagnostics
            .iter()
            .map(extraction_diagnostic)
            .collect(),
    )
}

fn emit_text(
    args: &ExtractArgs,
    report: &ExtractionReport<ExtractionPathSegment, ExtractionSource>,
) {
    println!("template: {}", to_forward_slash(&args.template));
    println!("rendered: {}", to_forward_slash(&args.rendered));
    println!("format: xml");
    println!("confidence: {:.4}", report.confidence);
    if report.values.is_empty() {
        println!("values: <none>");
    } else {
        println!("values:");
        for (name, value) in &report.values {
            println!("  {name}: {}", bounded_text(value));
        }
    }
    if report.occurrences.is_empty() {
        println!("occurrences: <none>");
    } else {
        println!("occurrences:");
        for occurrence in &report.occurrences {
            println!(
                "  {} at {} ({}) = {}",
                occurrence.variable,
                format_path(&occurrence.path),
                format_source(&occurrence.source),
                occurrence
                    .rendered_text
                    .as_deref()
                    .map_or_else(|| "<none>".to_owned(), bounded_text),
            );
        }
    }
    if report.diagnostics.is_empty() {
        println!("warnings: <none>");
    } else {
        println!("warnings:");
        for diagnostic in &report.diagnostics {
            println!("  {}: {}", diagnostic.code.as_str(), diagnostic.message);
        }
    }
}

fn bounded_text(value: &str) -> String {
    const MAX_CHARS: usize = 120;
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix:?}...")
    } else {
        format!("{prefix:?}")
    }
}

fn format_path(path: &[ExtractionPathSegment]) -> String {
    let mut formatted = String::new();
    for segment in path {
        match segment {
            ExtractionPathSegment::Xml(XmlPathSegment::Element { name, ordinal }) => {
                let _ = write!(formatted, "/{name}[{ordinal}]");
            }
            ExtractionPathSegment::Xml(XmlPathSegment::Attribute { name }) => {
                let _ = write!(formatted, "@{name}");
            }
            ExtractionPathSegment::Json(sc_composer::JsonPathSegment::ObjectKey { key }) => {
                let _ = write!(formatted, ".{key}");
            }
            ExtractionPathSegment::Json(sc_composer::JsonPathSegment::ArrayIndex { index }) => {
                let _ = write!(formatted, "[{index}]");
            }
        }
    }
    formatted
}

fn format_source(source: &ExtractionSource) -> &'static str {
    match source {
        ExtractionSource::Xml(XmlExtractionSource::Attribute { .. }) => "attribute",
        ExtractionSource::Xml(XmlExtractionSource::TextNode) => "text_node",
        ExtractionSource::Json(sc_composer::JsonExtractionSource::StringValue) => "string_value",
    }
}

fn extraction_occurrence_json(
    occurrence: &sc_composer::ExtractionOccurrence<ExtractionPathSegment, ExtractionSource>,
) -> serde_json::Value {
    serde_json::json!({
        "variable": occurrence.variable.to_string(),
        "path": occurrence.path.iter().map(path_segment_json).collect::<Vec<_>>(),
        "source": source_json(&occurrence.source),
        "rendered_text": occurrence.rendered_text,
    })
}

fn path_segment_json(segment: &ExtractionPathSegment) -> serde_json::Value {
    match segment {
        ExtractionPathSegment::Xml(XmlPathSegment::Element { name, ordinal }) => {
            serde_json::json!({"kind": "element", "name": name, "ordinal": ordinal})
        }
        ExtractionPathSegment::Xml(XmlPathSegment::Attribute { name }) => {
            serde_json::json!({"kind": "attribute", "name": name})
        }
        ExtractionPathSegment::Json(sc_composer::JsonPathSegment::ObjectKey { key }) => {
            serde_json::json!({"kind": "object_key", "key": key})
        }
        ExtractionPathSegment::Json(sc_composer::JsonPathSegment::ArrayIndex { index }) => {
            serde_json::json!({"kind": "array_index", "index": index})
        }
    }
}

fn source_json(source: &ExtractionSource) -> serde_json::Value {
    match source {
        ExtractionSource::Xml(XmlExtractionSource::Attribute { name }) => {
            serde_json::json!({"kind": "attribute", "name": name})
        }
        ExtractionSource::Xml(XmlExtractionSource::TextNode) => {
            serde_json::json!({"kind": "text_node"})
        }
        ExtractionSource::Json(sc_composer::JsonExtractionSource::StringValue) => {
            serde_json::json!({"kind": "string_value"})
        }
    }
}

fn extraction_diagnostic_json(diagnostic: &ExtractionDiagnostic) -> serde_json::Value {
    serde_json::json!({
        "code": diagnostic.code.as_str(),
        "kind": diagnostic_kind(diagnostic.kind),
        "message": diagnostic.message,
        "occurrence": diagnostic.occurrence.map(|index| index.0),
    })
}

fn extraction_diagnostic(diagnostic: &ExtractionDiagnostic) -> Diagnostic {
    Diagnostic::new(
        DiagnosticSeverity::Warning,
        diagnostic.code,
        diagnostic.message.clone(),
    )
}

fn diagnostic_kind(kind: ExtractionDiagnosticKind) -> &'static str {
    match kind {
        ExtractionDiagnosticKind::Unsupported => "unsupported",
        ExtractionDiagnosticKind::Ambiguous => "ambiguous",
        ExtractionDiagnosticKind::NotObserved => "not_observed",
        ExtractionDiagnosticKind::Malformed => "malformed",
    }
}

#[cfg(test)]
mod tests {
    use super::json_output_error;
    use crate::exit_codes::USAGE_FAIL;
    use anyhow::anyhow;
    use sc_composer::DiagnosticCode;

    #[test]
    fn json_output_error_preserves_diagnostic_and_recovery_metadata() {
        let error = json_output_error(anyhow!("stdout is closed"));

        assert_eq!(error.exit_code, USAGE_FAIL);
        assert_eq!(error.diagnostic_code, Some(DiagnosticCode::ErrRenderWrite));
        assert_eq!(error.diagnostics[0].code, DiagnosticCode::ErrRenderWrite);
        assert_eq!(error.recovery_hints.len(), 1);
    }
}
