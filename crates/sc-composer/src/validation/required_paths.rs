use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::types::{InputValue, VariableName};

use super::ValidationState;

#[derive(Debug, PartialEq, Eq)]
enum RequiredPathStatus {
    Satisfied,
    MissingTopLevel,
    MissingNested { missing_path: String },
    ShapeMismatch { at_path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
    column: usize,
}

pub(super) fn required_path_diagnostics(state: &ValidationState) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (variable, origin) in &state.required_origins {
        let include_chain = state
            .required_include_chains
            .get(variable)
            .cloned()
            .unwrap_or_default();
        match validate_required_path(&state.context, variable) {
            RequiredPathStatus::Satisfied => {}
            RequiredPathStatus::MissingTopLevel => {
                diagnostics.push(missing_required_diagnostic(origin, variable, include_chain));
            }
            RequiredPathStatus::MissingNested { missing_path } => {
                diagnostics.push(required_path_diagnostic(
                    DiagnosticCode::ErrValMissingNestedField,
                    origin,
                    variable,
                    format!("missing required nested field: {missing_path}"),
                    include_chain,
                ));
            }
            RequiredPathStatus::ShapeMismatch { at_path } => {
                diagnostics.push(required_path_diagnostic(
                    DiagnosticCode::ErrValShapeMismatch,
                    origin,
                    variable,
                    format!(
                        "required nested field path {variable} expected an object at {at_path}"
                    ),
                    include_chain,
                ));
            }
        }
    }

    diagnostics
}

fn missing_required_diagnostic(
    origin: &Path,
    variable: &VariableName,
    include_chain: Vec<PathBuf>,
) -> Diagnostic {
    let diagnostic = Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::ErrValMissingRequired,
        format!("missing required variable: {variable}"),
    )
    .with_path(origin.to_path_buf())
    .with_include_chain(include_chain);
    match required_variable_location(origin, variable.as_str()) {
        Some(location) => diagnostic.with_location(location.line, location.column),
        None => diagnostic,
    }
}

fn required_path_diagnostic(
    code: DiagnosticCode,
    origin: &Path,
    variable: &VariableName,
    message: String,
    include_chain: Vec<PathBuf>,
) -> Diagnostic {
    let diagnostic = Diagnostic::new(DiagnosticSeverity::Error, code, message)
        .with_path(origin.to_path_buf())
        .with_include_chain(include_chain);
    match required_variable_location(origin, variable.as_str()) {
        Some(location) => diagnostic.with_location(location.line, location.column),
        None => diagnostic,
    }
}

fn validate_required_path(
    context: &BTreeMap<VariableName, InputValue>,
    variable: &VariableName,
) -> RequiredPathStatus {
    let path = variable.as_str();
    let mut segments = path.split('.');
    let Some(first) = segments.next() else {
        return RequiredPathStatus::MissingTopLevel;
    };
    let Ok(top_level) = VariableName::new(first) else {
        return RequiredPathStatus::MissingTopLevel;
    };
    let Some(current) = context.get(&top_level) else {
        return RequiredPathStatus::MissingTopLevel;
    };
    let remaining_segments = segments.collect::<Vec<_>>();
    validate_required_value(current, &remaining_segments, first)
}

fn validate_required_value(
    current: &serde_json::Value,
    segments: &[&str],
    traversed: &str,
) -> RequiredPathStatus {
    let Some((segment, rest)) = segments.split_first() else {
        return RequiredPathStatus::Satisfied;
    };

    match current {
        serde_json::Value::Object(map) => {
            let Some(next) = map.get(*segment) else {
                return RequiredPathStatus::MissingNested {
                    missing_path: format!("{traversed}.{segment}"),
                };
            };
            let next_path = format!("{traversed}.{segment}");
            validate_required_value(next, rest, &next_path)
        }
        serde_json::Value::Array(values) => {
            for value in values {
                let status = validate_required_value(value, segments, traversed);
                if !matches!(status, RequiredPathStatus::Satisfied) {
                    return status;
                }
            }
            RequiredPathStatus::Satisfied
        }
        _ => RequiredPathStatus::ShapeMismatch {
            at_path: traversed.to_string(),
        },
    }
}

fn required_variable_location(path: &Path, variable: &str) -> Option<SourceLocation> {
    let raw = std::fs::read_to_string(path).ok()?;
    let mut in_required_variables = false;

    for (index, line) in raw.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if index == 0 && trimmed != "---" {
            return None;
        }
        if index > 0 && matches!(trimmed, "---" | "...") {
            break;
        }
        if trimmed == "required_variables:" {
            in_required_variables = true;
            continue;
        }
        if !in_required_variables {
            continue;
        }
        if trimmed.ends_with(':') && trimmed != "required_variables:" {
            in_required_variables = false;
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("- ") else {
            continue;
        };
        if rest == variable {
            let column = line.find(variable).map_or(1, |offset| offset + 1);
            return Some(SourceLocation {
                line: line_number,
                column,
            });
        }
    }

    None
}
