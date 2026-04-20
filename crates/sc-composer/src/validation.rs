//! Variable discovery and validation semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::ExpandedTemplate;
use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::frontmatter::{Frontmatter, parse_template_document};
use crate::types::{
    ComposeRequest, InputValue, UnknownVariablePolicy, ValidationReport, VariableName,
    VariableSource,
};

#[derive(Debug, PartialEq, Eq)]
enum RequiredPathStatus {
    Satisfied,
    MissingTopLevel,
    MissingNested { missing_path: String },
    ShapeMismatch { at_path: String },
}

#[derive(Debug, Default)]
pub(crate) struct ValidationState {
    pub(crate) context: BTreeMap<VariableName, InputValue>,
    pub(crate) variable_sources: BTreeMap<VariableName, VariableSource>,
    pub(crate) required_origins: BTreeMap<VariableName, PathBuf>,
    required_include_chains: BTreeMap<VariableName, Vec<PathBuf>>,
    default_origins: BTreeMap<VariableName, Option<PathBuf>>,
    pub(crate) declared_variables: BTreeSet<VariableName>,
    pub(crate) referenced_variables: BTreeSet<VariableName>,
}

pub(crate) fn validate_expanded(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
    resolve_result: crate::ResolveResult,
) -> ValidationReport {
    let state = collect_validation_state(request, expanded);

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if expanded.text.trim().is_empty() {
        errors.push(
            Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrValEmpty,
                "template body is empty",
            )
            .with_path(resolve_result.resolved_path.clone()),
        );
    }

    warnings.extend(missing_frontmatter_warnings(&resolve_result, expanded));
    warnings.extend(frontmatter_diagnostics(expanded));
    warnings.extend(default_usage_diagnostics(&state));
    errors.extend(missing_required_path_diagnostics(&state));

    for variable in state
        .referenced_variables
        .difference(&state.declared_variables)
        .cloned()
        .collect::<Vec<_>>()
    {
        let diagnostic = Diagnostic::new(
            if request.policy.strict_undeclared_variables {
                DiagnosticSeverity::Error
            } else {
                DiagnosticSeverity::Warning
            },
            DiagnosticCode::ErrValUndeclaredToken,
            format!("undeclared referenced token: {variable}"),
        )
        .with_path(resolve_result.resolved_path.clone());

        if request.policy.strict_undeclared_variables {
            errors.push(diagnostic);
        } else {
            warnings.push(diagnostic);
        }
    }

    push_extra_input_diagnostics(
        request,
        &state,
        &resolve_result.resolved_path,
        &mut warnings,
        &mut errors,
    );

    ValidationReport {
        ok: errors.is_empty(),
        warnings,
        errors,
        resolve_result,
    }
}

fn missing_required_path_diagnostics(state: &ValidationState) -> Vec<Diagnostic> {
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

fn push_extra_input_diagnostics(
    request: &ComposeRequest,
    state: &ValidationState,
    resolved_path: &Path,
    warnings: &mut Vec<Diagnostic>,
    errors: &mut Vec<Diagnostic>,
) {
    let declared_or_referenced = top_level_boundary_names(
        state
            .declared_variables
            .union(&state.referenced_variables)
            .cloned()
            .collect::<BTreeSet<_>>(),
    );
    let provided_variables = top_level_boundary_names(
        request
            .vars_input
            .keys()
            .chain(request.vars_env.keys())
            .cloned()
            .collect::<BTreeSet<_>>(),
    );

    for variable in provided_variables
        .difference(&declared_or_referenced)
        .cloned()
        .collect::<Vec<_>>()
    {
        let diagnostic = Diagnostic::new(
            match request.policy.unknown_variable_policy {
                UnknownVariablePolicy::Error => DiagnosticSeverity::Error,
                UnknownVariablePolicy::Warn => DiagnosticSeverity::Warning,
                UnknownVariablePolicy::Ignore => continue,
            },
            DiagnosticCode::ErrValExtraInput,
            format!("extra provided variable: {variable}"),
        )
        .with_path(resolved_path.to_path_buf());

        match request.policy.unknown_variable_policy {
            UnknownVariablePolicy::Error => errors.push(diagnostic),
            UnknownVariablePolicy::Warn => warnings.push(diagnostic),
            UnknownVariablePolicy::Ignore => {}
        }
    }
}

fn missing_frontmatter_warnings(
    resolve_result: &crate::ResolveResult,
    expanded: &ExpandedTemplate,
) -> Vec<Diagnostic> {
    expanded
        .frontmatters
        .iter()
        .filter_map(|(path, frontmatter)| {
            if frontmatter.is_some() || !file_references_variables(path) {
                return None;
            }
            let message = if *path == resolve_result.resolved_path {
                format!(
                    "root template has no frontmatter; run `sc-compose frontmatter-init {}`",
                    resolve_result.resolved_path.display()
                )
            } else {
                format!(
                    "included file has no frontmatter; run `sc-compose frontmatter-init {}`",
                    path.display()
                )
            };
            Some(
                Diagnostic::new(
                    DiagnosticSeverity::Warning,
                    DiagnosticCode::ErrValMissingFrontmatter,
                    message,
                )
                .with_path(path.clone()),
            )
        })
        .collect()
}

fn file_references_variables(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(parsed) = parse_template_document(&raw) else {
        return false;
    };
    !discover_tokens(parsed.body()).is_empty()
}

fn frontmatter_diagnostics(expanded: &ExpandedTemplate) -> Vec<Diagnostic> {
    expanded
        .frontmatters
        .iter()
        .flat_map(|(path, frontmatter)| {
            frontmatter
                .iter()
                .flat_map(|frontmatter| frontmatter.diagnostics().iter())
                .cloned()
                .map(|diagnostic| {
                    if diagnostic.path.is_some() {
                        diagnostic
                    } else {
                        diagnostic.with_path(path.clone())
                    }
                })
        })
        .collect()
}

fn default_usage_diagnostics(state: &ValidationState) -> Vec<Diagnostic> {
    state
        .variable_sources
        .iter()
        .filter_map(|(variable, source)| {
            if !matches!(
                source,
                VariableSource::TemplateInputDefault
                    | VariableSource::FrontmatterDefault
                    | VariableSource::IncludedDefault
            ) {
                return None;
            }
            let top_level = top_level_variable_name(variable);
            let used_by_reference = state
                .referenced_variables
                .iter()
                .any(|referenced| top_level_variable_name(referenced) == top_level);
            let used_by_required = state
                .required_origins
                .keys()
                .any(|required| top_level_variable_name(required) == top_level);
            if !used_by_reference && !used_by_required {
                return None;
            }

            let value = state.context.get(variable)?;
            let value_json =
                serde_json::to_string(value).unwrap_or_else(|_| "<unprintable>".to_owned());
            let diagnostic = Diagnostic::new(
                DiagnosticSeverity::Info,
                DiagnosticCode::InfoValDefaultUsed,
                format!("variable {variable} not provided, using default: {value_json}"),
            );

            Some(match source {
                VariableSource::FrontmatterDefault | VariableSource::IncludedDefault => {
                    if let Some(path) = state.default_origins.get(variable).and_then(Clone::clone) {
                        diagnostic.with_path(path)
                    } else {
                        diagnostic
                    }
                }
                VariableSource::TemplateInputDefault => diagnostic,
                VariableSource::ExplicitInput | VariableSource::Environment => unreachable!(),
            })
        })
        .collect()
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
        Some((line, column)) => diagnostic.with_location(line, column),
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
        Some((line, column)) => diagnostic.with_location(line, column),
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
    let top_level = VariableName::new(first).expect("top-level path segment remains valid");
    let Some(mut current) = context.get(&top_level) else {
        return RequiredPathStatus::MissingTopLevel;
    };
    let mut traversed = String::from(first);

    for segment in segments {
        match current {
            serde_json::Value::Object(map) => {
                let Some(next) = map.get(segment) else {
                    return RequiredPathStatus::MissingNested {
                        missing_path: format!("{traversed}.{segment}"),
                    };
                };
                traversed.push('.');
                traversed.push_str(segment);
                current = next;
            }
            _ => return RequiredPathStatus::ShapeMismatch { at_path: traversed },
        }
    }

    RequiredPathStatus::Satisfied
}

fn top_level_variable_name(variable: &VariableName) -> VariableName {
    let top_level = variable
        .as_str()
        .split('.')
        .next()
        .unwrap_or(variable.as_str());
    VariableName::new(top_level).expect("top-level path segment remains valid")
}

fn top_level_boundary_names(variables: BTreeSet<VariableName>) -> BTreeSet<VariableName> {
    variables
        .into_iter()
        .map(|variable| top_level_variable_name(&variable))
        .collect()
}

fn required_variable_location(path: &Path, variable: &str) -> Option<(usize, usize)> {
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
            return Some((line_number, column));
        }
    }

    None
}

pub(crate) fn collect_validation_state(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
) -> ValidationState {
    let mut state = ValidationState::default();
    let root_path = expanded.resolved_files.first();

    for (path, frontmatter) in &expanded.frontmatters {
        if let Some(frontmatter) = frontmatter {
            let is_root = root_path.is_some_and(|root| root == path);
            merge_frontmatter(path, frontmatter, expanded, &mut state, is_root);
        }
    }

    for (name, value) in &request.vars_defaults {
        state.context.insert(name.clone(), value.clone());
        state.default_origins.insert(name.clone(), None);
        state
            .variable_sources
            .insert(name.clone(), VariableSource::TemplateInputDefault);
    }
    for (name, value) in &request.vars_env {
        state.context.insert(name.clone(), value.clone());
        state
            .variable_sources
            .insert(name.clone(), VariableSource::Environment);
    }
    for (name, value) in &request.vars_input {
        state.context.insert(name.clone(), value.clone());
        state
            .variable_sources
            .insert(name.clone(), VariableSource::ExplicitInput);
    }

    state.referenced_variables = discover_tokens(&expanded.text);
    state
}

fn merge_frontmatter(
    path: &Path,
    frontmatter: &Frontmatter,
    expanded: &ExpandedTemplate,
    state: &mut ValidationState,
    is_root: bool,
) {
    for variable in frontmatter.required_variables() {
        state
            .required_origins
            .entry(variable.clone())
            .or_insert_with(|| path.to_path_buf());
        state
            .required_include_chains
            .entry(variable.clone())
            .or_insert_with(|| {
                expanded
                    .include_chains
                    .get(path)
                    .cloned()
                    .unwrap_or_default()
            });
        state.declared_variables.insert(variable.clone());
    }

    for (variable, value) in frontmatter.defaults() {
        state.declared_variables.insert(variable.clone());
        state
            .default_origins
            .entry(variable.clone())
            .or_insert_with(|| Some(path.to_path_buf()));
        state
            .context
            .entry(variable.clone())
            .or_insert_with(|| value.clone());
        state
            .variable_sources
            .entry(variable.clone())
            .or_insert(if is_root {
                VariableSource::FrontmatterDefault
            } else {
                VariableSource::IncludedDefault
            });
    }
}

pub(crate) fn discover_tokens(text: &str) -> BTreeSet<VariableName> {
    let mut tokens = BTreeSet::new();
    collect_tokens_from_delimiters(text, "{{", "}}", &mut tokens);
    collect_tokens_from_delimiters(text, "{%", "%}", &mut tokens);
    tokens
}

fn collect_tokens_from_delimiters(
    text: &str,
    start_delimiter: &str,
    end_delimiter: &str,
    tokens: &mut BTreeSet<VariableName>,
) {
    let mut cursor = text;
    while let Some(start) = cursor.find(start_delimiter) {
        let after_start = &cursor[start + start_delimiter.len()..];
        let Some(end) = after_start.find(end_delimiter) else {
            break;
        };
        let expression = &after_start[..end];
        collect_identifiers(expression, tokens);
        cursor = &after_start[end + end_delimiter.len()..];
    }
}

fn collect_identifiers(expression: &str, tokens: &mut BTreeSet<VariableName>) {
    const KEYWORDS: &[&str] = &[
        "if",
        "else",
        "elif",
        "endif",
        "for",
        "endfor",
        "in",
        "set",
        "true",
        "false",
        "none",
        "not",
        "and",
        "or",
        "block",
        "endblock",
        "macro",
        "endmacro",
        "filter",
        "endfilter",
    ];

    for candidate in expression.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    }) {
        if candidate.is_empty() || KEYWORDS.contains(&candidate) {
            continue;
        }
        if let Ok(variable) = VariableName::new(candidate) {
            tokens.insert(variable);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::types::{
        ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, UnknownVariablePolicy,
    };
    use crate::{DiagnosticCode, DiagnosticSeverity, validate};

    use super::collect_validation_state;

    #[test]
    fn default_mode_preserves_undeclared_tokens_as_warnings() {
        let root = temp_root("validation_default_undeclared");
        write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert!(report.ok);
        assert!(report.errors.is_empty());
        assert!(
            report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValUndeclaredToken),
            "expected undeclared-token warning"
        );
    }

    #[test]
    fn strict_mode_fails_on_undeclared_tokens() {
        let root = temp_root("validation_strict_undeclared");
        write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                strict_undeclared_variables: true,
                ..ComposePolicy::default()
            },
        ))
        .unwrap();

        assert!(!report.ok);
        assert_eq!(report.errors[0].code, DiagnosticCode::ErrValUndeclaredToken);
    }

    #[test]
    fn include_derived_defaults_and_required_variables_merge() {
        let root = temp_root("validation_include_merge");
        write_file(
            &root.join("root.md.j2"),
            "---\ndefaults:\n  name: parent\n---\n@<child.md>\n",
        );
        write_file(
            &root.join("child.md"),
            "---\nrequired_variables:\n  - name\ndefaults:\n  child_only: present\n---\nhello {{ name }}\n",
        );

        let request = request_for_file(&root, "root.md.j2", ComposePolicy::default());
        let resolve_result = crate::resolve_template_path(&request).unwrap();
        let expanded = crate::expand_includes(
            &resolve_result.resolved_path,
            &request.root,
            &request.policy,
        )
        .unwrap();
        let state = collect_validation_state(&request, &expanded);

        assert_eq!(
            state
                .context
                .get(&crate::VariableName::new("name").unwrap()),
            Some(&json!("parent"))
        );
        assert!(
            state
                .required_origins
                .contains_key(&crate::VariableName::new("name").unwrap())
        );
        assert_eq!(
            state
                .context
                .get(&crate::VariableName::new("child_only").unwrap()),
            Some(&json!("present"))
        );
    }

    #[test]
    fn environment_overrides_defaults_and_explicit_input_overrides_environment() {
        let root = temp_root("validation_precedence");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: default\n---\nhello {{ name }}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request
            .vars_env
            .insert(crate::VariableName::new("name").unwrap(), json!("env"));
        request
            .vars_input
            .insert(crate::VariableName::new("name").unwrap(), json!("input"));

        let resolve_result = crate::resolve_template_path(&request).unwrap();
        let expanded = crate::expand_includes(
            &resolve_result.resolved_path,
            &request.root,
            &request.policy,
        )
        .unwrap();
        let state = collect_validation_state(&request, &expanded);

        assert_eq!(
            state
                .context
                .get(&crate::VariableName::new("name").unwrap()),
            Some(&json!("input"))
        );
        assert_eq!(
            state
                .variable_sources
                .get(&crate::VariableName::new("name").unwrap()),
            Some(&crate::VariableSource::ExplicitInput)
        );
    }

    #[test]
    fn missing_root_frontmatter_emits_fixup_warning() {
        let root = temp_root("validation_missing_frontmatter");
        write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert!(
            report.warnings.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::ErrValMissingFrontmatter
                    && diagnostic.message.contains("sc-compose frontmatter-init")
            }),
            "expected missing-frontmatter warning with fix command"
        );
    }

    #[test]
    fn extra_input_policy_can_error() {
        let root = temp_root("validation_extra_input");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unknown_variable_policy: UnknownVariablePolicy::Error,
                ..ComposePolicy::default()
            },
        );
        request
            .vars_input
            .insert(crate::VariableName::new("name").unwrap(), json!("world"));
        request
            .vars_input
            .insert(crate::VariableName::new("extra").unwrap(), json!("value"));

        let report = validate(&request).unwrap();
        assert!(!report.ok);
        assert!(
            report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValExtraInput)
        );
    }

    #[test]
    fn input_defaults_alias_marks_optional_variable_as_known() {
        let root = temp_root("validation_input_defaults_known");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - task_id\ninput_defaults:\n  assignee: teammate\n---\nhello {{ task_id }} {{ assignee }}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unknown_variable_policy: UnknownVariablePolicy::Error,
                ..ComposePolicy::default()
            },
        );
        request
            .vars_input
            .insert(crate::VariableName::new("task_id").unwrap(), json!("SC-1"));
        request.vars_input.insert(
            crate::VariableName::new("assignee").unwrap(),
            json!("architect"),
        );

        let report = validate(&request).unwrap();
        assert!(report.ok, "{report:?}");
        assert!(
            !report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValExtraInput)
        );
    }

    #[test]
    fn input_defaults_only_var_uses_default_when_absent_emits_info_diagnostic() {
        let root = temp_root("validation_input_defaults_only_default");
        write_file(
            &root.join("template.md.j2"),
            "---\ninput_defaults:\n  assignee: teammate\n---\nhello {{ assignee }}\n",
        );

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
        assert!(
            report.warnings.iter().any(|diagnostic| {
                diagnostic.severity == DiagnosticSeverity::Info
                    && diagnostic.code == DiagnosticCode::InfoValDefaultUsed
                    && diagnostic
                        .message
                        .contains("variable assignee not provided")
                    && diagnostic.message.contains("\"teammate\"")
            }),
            "{report:?}"
        );
    }

    #[test]
    fn required_variable_is_satisfied_by_input_defaults_alias() {
        let root = temp_root("validation_required_input_defaults");
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
        assert!(
            report.warnings.iter().any(|diagnostic| {
                diagnostic.severity == DiagnosticSeverity::Info
                    && diagnostic.code == DiagnosticCode::InfoValDefaultUsed
                    && diagnostic.message.contains("using default")
                    && diagnostic.message.contains("\"world\"")
            }),
            "{report:?}"
        );
    }

    #[test]
    fn required_variable_path_pr_number_is_satisfied_by_object_input() {
        let root = temp_root("validation_required_object_path");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - pr.number\n---\nhello {{ pr.number }}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("pr").unwrap(),
            json!({
                "number": 43,
                "url": "https://example.test/pr/43",
            }),
        );

        let report = validate(&request).unwrap();

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn missing_nested_field_reports_err_val_missing_nested_field() {
        let root = temp_root("validation_missing_nested_field");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - pr.number\n---\nhello {{ pr.number }}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("pr").unwrap(),
            json!({ "url": "https://example.test/pr/43" }),
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
        let root = temp_root("validation_shape_mismatch");
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
    fn structured_defaults_replace_without_deep_merge() {
        let root = temp_root("validation_structured_default_replace");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  pr:\n    number: 7\n    url: https://example.test/pr/7\n---\nhello {{ pr.number }}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("pr").unwrap(),
            json!({
                "number": 43,
            }),
        );

        let resolve_result = crate::resolve_template_path(&request).unwrap();
        let expanded = crate::expand_includes(
            &resolve_result.resolved_path,
            &request.root,
            &request.policy,
        )
        .unwrap();
        let state = collect_validation_state(&request, &expanded);

        assert_eq!(
            state.context.get(&crate::VariableName::new("pr").unwrap()),
            Some(&json!({ "number": 43 }))
        );
    }

    #[test]
    fn extra_nested_fields_are_ignored_by_top_level_extra_input_policy() {
        let root = temp_root("validation_extra_nested_fields");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - pr.number\n---\nhello {{ pr.number }}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unknown_variable_policy: UnknownVariablePolicy::Error,
                ..ComposePolicy::default()
            },
        );
        request.vars_input.insert(
            crate::VariableName::new("pr").unwrap(),
            json!({
                "number": 43,
                "url": "https://example.test/pr/43",
                "status": "open",
            }),
        );

        let report = validate(&request).unwrap();

        assert!(report.ok, "{report:?}");
        assert!(
            !report
                .errors
                .iter()
                .any(|diagnostic| { diagnostic.code == DiagnosticCode::ErrValExtraInput })
        );
    }

    #[test]
    fn empty_template_body_emits_empty_code() {
        let root = temp_root("validation_empty_body");
        write_file(&root.join("template.md.j2"), "   \n");

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert!(!report.ok);
        assert!(
            report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValEmpty)
        );
    }

    fn request_for_file(root: &Path, file: &str, policy: ComposePolicy) -> ComposeRequest {
        ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from(file),
            },
            root: ConfiningRoot::new(root).unwrap(),
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            vars_defaults: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy,
        }
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
