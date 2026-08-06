use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::discovery::has_bare_for_loop_over;
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
        let source = state.source_texts.get(origin).map(String::as_str);
        let include_chain = state
            .required_include_chains
            .get(variable)
            .cloned()
            .unwrap_or_default();
        let status = validate_required_path(&state.context, variable);
        let should_report_array_shape = matches!(&status, RequiredPathStatus::Satisfied)
            && !variable.as_str().contains('.')
            && !has_dotted_required_sibling(state, variable)
            && state.context.get(variable).is_some_and(|value| {
                !value.is_array()
                    && source
                        .is_some_and(|source| has_bare_for_loop_over(source, variable.as_str()))
            });

        match status {
            RequiredPathStatus::Satisfied => {}
            RequiredPathStatus::MissingTopLevel => {
                diagnostics.push(missing_required_diagnostic(
                    origin,
                    variable,
                    include_chain.clone(),
                    source,
                ));
            }
            RequiredPathStatus::MissingNested { missing_path } => {
                diagnostics.push(required_path_diagnostic(
                    DiagnosticCode::ErrValMissingNestedField,
                    origin,
                    variable,
                    format!("missing required nested field: {missing_path}"),
                    include_chain.clone(),
                    source,
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
                    include_chain.clone(),
                    source,
                ));
            }
        }

        if should_report_array_shape {
            let value = state
                .context
                .get(variable)
                .expect("array-shape check requires a present context value");
            diagnostics.push(required_path_diagnostic(
                DiagnosticCode::ErrValArrayShapeMismatch,
                origin,
                variable,
                format!(
                    "required variable {variable} is consumed as a list via `{{% for %}}` but received {}",
                    json_type_name(value)
                ),
                include_chain,
                source,
            ));
        }
    }

    diagnostics
}

fn has_dotted_required_sibling(state: &ValidationState, variable: &VariableName) -> bool {
    let prefix = format!("{}.", variable.as_str());
    state
        .required_origins
        .keys()
        .any(|candidate| candidate.as_str().starts_with(&prefix))
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn missing_required_diagnostic(
    origin: &Path,
    variable: &VariableName,
    include_chain: Vec<PathBuf>,
    source: Option<&str>,
) -> Diagnostic {
    let diagnostic = Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticCode::ErrValMissingRequired,
        format!("missing required variable: {variable}"),
    )
    .with_path(origin.to_path_buf())
    .with_include_chain(include_chain);
    match required_variable_location(source, variable.as_str()) {
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
    source: Option<&str>,
) -> Diagnostic {
    let diagnostic = Diagnostic::new(DiagnosticSeverity::Error, code, message)
        .with_path(origin.to_path_buf())
        .with_include_chain(include_chain);
    match required_variable_location(source, variable.as_str()) {
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

fn required_variable_location(raw: Option<&str>, variable: &str) -> Option<SourceLocation> {
    let raw = raw?;
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::types::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};
    use crate::{DiagnosticCode, DiagnosticSeverity, validate};

    #[test]
    fn required_variable_is_satisfied_by_input_defaults_alias() {
        let root = temp_root("required_input_defaults");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - name\ninput_defaults:\n  name: world\n---\nhello {{ name }}\n",
        );

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
        assert!(report.warnings.iter().any(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Info
                && diagnostic.code == DiagnosticCode::InfoValDefaultUsed
                && diagnostic.message.contains("using default")
                && diagnostic.message.contains("\"world\"")
        }));
    }

    #[test]
    fn required_variable_path_pr_number_is_satisfied_by_object_input() {
        let root = temp_root("required_object_path");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - pr.number\n---\nhello {{ pr.number }}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("pr").unwrap(),
            json!({"number": 43, "url": "https://example.test/pr/43"}),
        );

        let report = validate(&request).unwrap();
        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn missing_nested_field_reports_err_val_missing_nested_field() {
        let root = temp_root("missing_nested_field");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - pr.number\n---\nhello {{ pr.number }}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("pr").unwrap(),
            json!({"url": "https://example.test/pr/43"}),
        );

        let report = validate(&request).unwrap();
        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValMissingNestedField
                && diagnostic.message.contains("pr.number")
        }));
    }

    #[test]
    fn shape_mismatch_reports_err_val_shape_mismatch() {
        let root = temp_root("shape_mismatch");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - pr.number\n---\nhello {{ pr.number }}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("pr").unwrap(),
            json!("not-an-object"),
        );

        let report = validate(&request).unwrap();
        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValShapeMismatch
                && diagnostic.message.contains("pr.number")
                && diagnostic.message.contains("pr")
        }));
    }

    #[test]
    fn bare_for_loop_rejects_string_required_value() {
        let report = validate_bare_loop_value("bare_loop_string", json!("ab"));

        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValArrayShapeMismatch
                && diagnostic.message.contains("items")
                && diagnostic.message.contains("string")
        }));
    }

    #[test]
    fn bare_for_loop_rejects_object_required_value() {
        let report = validate_bare_loop_value("bare_loop_object", json!({"key": "value"}));

        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValArrayShapeMismatch
                && diagnostic.message.contains("object")
        }));
    }

    #[test]
    fn bare_for_loop_accepts_array_required_value() {
        let report = validate_bare_loop_value("bare_loop_array", json!(["a", "b"]));

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn non_iterated_required_value_does_not_trigger_array_shape_check() {
        let root = temp_root("non_iterated_required_value");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - items\n---\nvalue={{ items }}\n",
        );
        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request
            .vars_input
            .insert(crate::VariableName::new("items").unwrap(), json!("ab"));

        let report = validate(&request).unwrap();

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn dotted_required_sibling_keeps_nested_field_diagnostic() {
        let root = temp_root("bare_loop_dotted_sibling");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - items\n  - items.id\n---\n{% for item in items %}{{ item.id }}{% endfor %}\n",
        );
        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("items").unwrap(),
            json!([{"name": "missing-id"}]),
        );

        let report = validate(&request).unwrap();

        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValMissingNestedField
                && diagnostic.message.contains("items.id")
        }));
        assert!(
            !report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValArrayShapeMismatch)
        );
    }

    #[test]
    fn non_bare_for_iterables_do_not_trigger_array_shape_check() {
        for (label, iterable) in [("filtered", "items | sort"), ("qualified", "items.values")] {
            let root = temp_root(&format!("non_bare_loop_{label}"));
            write_file(
                &root.join("template.md.j2"),
                &format!(
                    "---\nrequired_variables:\n  - items\n---\n{{% for item in {iterable} %}}{{{{ item }}}}{{% endfor %}}\n"
                ),
            );
            let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
            request
                .vars_input
                .insert(crate::VariableName::new("items").unwrap(), json!("ab"));

            let report = validate(&request).unwrap();

            assert!(
                !report
                    .errors
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValArrayShapeMismatch),
                "unexpected array-shape diagnostic for {label}: {report:?}"
            );
        }
    }

    #[test]
    fn required_variable_path_array_member_id_is_satisfied_by_array_of_objects() {
        let root = temp_root("required_array_member_path");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - sprints.id\n---\n{% for sprint in sprints %}{{ sprint.id }}{% endfor %}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("sprints").unwrap(),
            json!([{"id": "S1", "stage": "qa"}, {"id": "S2", "stage": "merged"}]),
        );

        let report = validate(&request).unwrap();
        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn missing_nested_field_in_array_member_reports_err_val_missing_nested_field() {
        let root = temp_root("missing_array_member_field");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - sprints.id\n---\n{% for sprint in sprints %}{{ sprint.id }}{% endfor %}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("sprints").unwrap(),
            json!([{"id": "S1", "stage": "qa"}, {"stage": "merged"}]),
        );

        let report = validate(&request).unwrap();
        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValMissingNestedField
                && diagnostic.message.contains("sprints.id")
        }));
    }

    #[test]
    fn shape_mismatch_in_array_member_reports_err_val_shape_mismatch() {
        let root = temp_root("array_member_shape_mismatch");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - sprints.id\n---\n{% for sprint in sprints %}{{ sprint.id }}{% endfor %}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("sprints").unwrap(),
            json!([{"id": "S1", "stage": "qa"}, "bad-member"]),
        );

        let report = validate(&request).unwrap();
        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValShapeMismatch
                && diagnostic.message.contains("sprints.id")
                && diagnostic.message.contains("sprints")
        }));
    }

    #[test]
    fn public_validate_preserves_nested_array_required_path_diagnostics() {
        let root = temp_root("required_nested_array_regression");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - groups.members.id\n---\n{{ groups }}\n",
        );
        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("groups").unwrap(),
            json!([{"members": [{"id": "ok"}]}, {"members": [{}]}]),
        );

        let report = validate(&request).unwrap();
        assert!(!report.ok);
        assert_eq!(
            report.errors[0].code,
            DiagnosticCode::ErrValMissingNestedField
        );
        assert!(report.errors[0].message.contains("groups.members.id"));
    }

    fn request_for_file(root: &Path, file: &str, policy: ComposePolicy) -> ComposeRequest {
        ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from(file),
            },
            root: ConfiningRoot::new(root).unwrap(),
            vars_input: BTreeMap::new(),
            vars_env: BTreeMap::new(),
            vars_defaults: BTreeMap::new(),
            guidance_block: None,
            user_prompt: None,
            policy,
        }
    }

    fn validate_bare_loop_value(label: &str, value: serde_json::Value) -> crate::ValidationReport {
        let root = temp_root(label);
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - items\n---\n{% for item in items %}{{ item }}{% endfor %}\n",
        );
        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request
            .vars_input
            .insert(crate::VariableName::new("items").unwrap(), value);

        validate(&request).unwrap()
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}
