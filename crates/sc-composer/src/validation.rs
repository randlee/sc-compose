//! Variable discovery and validation semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;

use crate::ExpandedTemplate;
use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::discovery::{discover_all_pass_tokens, discover_tokens};
use crate::frontmatter::{Frontmatter, parse_template_document};
use crate::types::{
    ComposeRequest, InputValue, UnknownVariablePolicy, ValidationReport, VariableName,
    VariableSource,
};

const RENDER_DATE_FORMAT: &[time::format_description::FormatItem<'static>] =
    format_description!("[year]-[month]-[day]");

/// Built-in render-context variable names injected for every render.
pub const BUILTIN_VARIABLE_NAMES: [&str; 5] = [
    "TEMPLATE_NAME",
    "HOSTNAME",
    "USERNAME",
    "RENDER_DATE",
    "RENDER_TIMESTAMP",
];

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

#[derive(Debug, Default)]
pub(crate) struct ValidationState {
    pub(crate) context: BTreeMap<VariableName, InputValue>,
    pub(crate) variable_sources: BTreeMap<VariableName, VariableSource>,
    pub(crate) required_origins: BTreeMap<VariableName, PathBuf>,
    required_include_chains: BTreeMap<VariableName, Vec<PathBuf>>,
    default_origins: BTreeMap<VariableName, Option<PathBuf>>,
    default_pass_numbers: BTreeMap<VariableName, BTreeSet<usize>>,
    pub(crate) declared_variables: BTreeSet<VariableName>,
    pub(crate) referenced_variables: BTreeSet<VariableName>,
    referenced_variables_by_pass: BTreeMap<usize, BTreeSet<VariableName>>,
    declared_variables_by_pass: BTreeMap<usize, BTreeSet<VariableName>>,
}

pub(crate) fn validate_expanded(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
    resolve_result: crate::ResolveResult,
) -> (ValidationReport, ValidationState) {
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

    for variable in undeclared_referenced_variables(&state) {
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

    (
        ValidationReport {
            ok: errors.is_empty(),
            warnings,
            errors,
            resolve_result,
        },
        state,
    )
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
        .filter_map(|(path, frontmatters)| {
            if !frontmatters.is_empty() || !file_references_variables(path) {
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
        .flat_map(|(path, frontmatters)| {
            frontmatters
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
            let used_by_reference = default_used_by_reference(state, variable);
            let used_by_required = state
                .required_origins
                .keys()
                .any(|required| default_satisfies_path(variable, required));
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
                VariableSource::ExplicitInput
                | VariableSource::Environment
                | VariableSource::Builtin => unreachable!(),
            })
        })
        .collect()
}

fn undeclared_referenced_variables(state: &ValidationState) -> Vec<VariableName> {
    let Some(referenced_variables_by_pass) = per_pass_referenced_variables(state) else {
        return state
            .referenced_variables
            .difference(&state.declared_variables)
            .cloned()
            .collect();
    };

    referenced_variables_by_pass
        .iter()
        .flat_map(|(pass_number, variables)| {
            let declared = state
                .declared_variables_by_pass
                .get(pass_number)
                .unwrap_or(&state.declared_variables);
            variables.difference(declared).cloned()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn default_used_by_reference(state: &ValidationState, variable: &VariableName) -> bool {
    let Some(referenced_variables_by_pass) = per_pass_referenced_variables(state) else {
        return state
            .referenced_variables
            .iter()
            .any(|referenced| default_satisfies_path(variable, referenced));
    };

    if let Some(pass_numbers) = state.default_pass_numbers.get(variable) {
        return pass_numbers.iter().any(|pass_number| {
            referenced_variables_by_pass
                .get(pass_number)
                .into_iter()
                .flatten()
                .any(|referenced| default_satisfies_path(variable, referenced))
        });
    }

    referenced_variables_by_pass
        .values()
        .flatten()
        .any(|referenced| default_satisfies_path(variable, referenced))
}

fn default_satisfies_path(default_variable: &VariableName, referenced: &VariableName) -> bool {
    referenced == default_variable
        || referenced
            .as_str()
            .strip_prefix(default_variable.as_str())
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn per_pass_referenced_variables(
    state: &ValidationState,
) -> Option<&BTreeMap<usize, BTreeSet<VariableName>>> {
    (!state.referenced_variables_by_pass.is_empty()).then_some(&state.referenced_variables_by_pass)
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

fn top_level_variable_name(variable: &VariableName) -> VariableName {
    let top_level = variable
        .as_str()
        .split('.')
        .next()
        .unwrap_or(variable.as_str());
    VariableName::new(top_level).unwrap_or_else(|_| variable.clone())
}

fn top_level_boundary_names(variables: BTreeSet<VariableName>) -> BTreeSet<VariableName> {
    variables
        .into_iter()
        .map(|variable| top_level_variable_name(&variable))
        .collect()
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

pub(crate) fn collect_validation_state(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
) -> ValidationState {
    let mut state = ValidationState::default();

    for (path, frontmatters) in &expanded.frontmatters {
        if !frontmatters.is_empty() {
            let is_root = expanded
                .resolved_files
                .first()
                .is_some_and(|root| root == path);
            for frontmatter in frontmatters {
                merge_frontmatter(path, frontmatter, expanded, &mut state, is_root);
            }
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

    declare_builtin_variables(&mut state);
    populate_pass_validation_maps(&mut state, expanded);
    state
}

fn populate_pass_validation_maps(state: &mut ValidationState, expanded: &ExpandedTemplate) {
    let root_path = expanded.resolved_files.first();
    let root_passes = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| {
            root_path
                .is_some_and(|root| path == root)
                .then(|| passes.clone())
        })
        .unwrap_or_default();
    if root_passes.is_empty() {
        state.referenced_variables = discover_tokens(&expanded.text);
        return;
    }

    let parsed = crate::frontmatter::ParsedTemplate::from_parts(root_passes, expanded.text.clone());

    let referenced_variables_by_pass = discover_all_pass_tokens(&parsed);
    let root_pass_declared = parsed
        .passes()
        .iter()
        .flat_map(|pass| {
            pass.required_variables()
                .iter()
                .chain(pass.defaults().keys())
                .cloned()
        })
        .collect::<BTreeSet<_>>();
    let shared_declared = state
        .declared_variables
        .difference(&root_pass_declared)
        .cloned()
        .collect::<BTreeSet<_>>();

    state.referenced_variables = referenced_variables_by_pass
        .values()
        .flatten()
        .cloned()
        .collect();
    state.referenced_variables_by_pass = referenced_variables_by_pass;
    state.declared_variables_by_pass = parsed
        .passes()
        .iter()
        .map(|pass| {
            let declared = shared_declared
                .iter()
                .cloned()
                .chain(pass.required_variables().iter().cloned())
                .chain(pass.defaults().keys().cloned())
                .collect::<BTreeSet<_>>();
            (usize::from(pass.pass_number()), declared)
        })
        .collect();

    for pass in parsed.passes() {
        let pass_number = usize::from(pass.pass_number());
        for variable in pass.defaults().keys() {
            state
                .default_pass_numbers
                .entry(variable.clone())
                .or_default()
                .insert(pass_number);
        }
    }
}

pub(crate) fn inject_builtin_vars(state: &mut ValidationState, template_path: &Path) {
    let now = OffsetDateTime::now_utc();
    insert_builtin_var(
        state,
        BUILTIN_VARIABLE_NAMES[0],
        template_path.file_name().map_or_else(
            || "unknown".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        ),
    );
    insert_builtin_var(state, BUILTIN_VARIABLE_NAMES[1], current_hostname());
    insert_builtin_var(state, BUILTIN_VARIABLE_NAMES[2], current_username());
    insert_builtin_var(state, BUILTIN_VARIABLE_NAMES[3], format_render_date(now));
    insert_builtin_var(
        state,
        BUILTIN_VARIABLE_NAMES[4],
        format_render_timestamp(now),
    );
}

fn declare_builtin_variables(state: &mut ValidationState) {
    for raw_name in BUILTIN_VARIABLE_NAMES {
        if let Ok(name) = VariableName::new(raw_name) {
            state.declared_variables.insert(name);
        }
    }
}

fn insert_builtin_var(state: &mut ValidationState, raw_name: &'static str, value: String) {
    let Ok(name) = VariableName::new(raw_name) else {
        return;
    };
    let preserve_caller_value = matches!(
        state.variable_sources.get(&name),
        Some(VariableSource::Environment | VariableSource::ExplicitInput)
    );
    if !preserve_caller_value {
        state
            .context
            .insert(name.clone(), serde_json::Value::String(value));
        state
            .variable_sources
            .insert(name.clone(), VariableSource::Builtin);
    }
    state.declared_variables.insert(name);
}

fn format_render_date(now: OffsetDateTime) -> String {
    now.format(RENDER_DATE_FORMAT)
        .unwrap_or_else(|_| render_date_fallback(now))
}

fn format_render_timestamp(now: OffsetDateTime) -> String {
    now.format(&Rfc3339)
        .unwrap_or_else(|_| render_timestamp_fallback(now))
}

fn render_date_fallback(now: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        u8::from(now.month()),
        now.day()
    )
}

fn render_timestamp_fallback(now: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn current_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn current_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_owned())
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::types::{
        ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, ResolveResult,
        UnknownVariablePolicy,
    };
    use crate::{DiagnosticCode, DiagnosticSeverity, validate};
    use crate::{ExpandedTemplate, parse_template_document};

    use super::{collect_validation_state, inject_builtin_vars, missing_frontmatter_warnings};

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
    fn strict_mode_accepts_approved_loop_context_builtins() {
        let root = temp_root("validation_loop_context");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - items\n---\n{% for item in items %}{{ loop }} {{ loop.index }} {{ loop.index0 }} {{ loop.revindex }} {{ loop.revindex0 }} {{ loop.first }} {{ loop.last }} {{ loop.length }} {{ loop.depth }} {{ loop.depth0 }} {{ loop.cycle(\"odd\", \"even\") }}:{{ item }}{% endfor %}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                strict_undeclared_variables: true,
                ..ComposePolicy::default()
            },
        );
        request.vars_input.insert(
            crate::VariableName::new("items").unwrap(),
            json!(["one", "two"]),
        );

        let report = validate(&request).unwrap();

        assert!(report.ok, "{report:?}");
        assert!(
            !report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.message.contains("loop."))
        );
    }

    #[test]
    fn strict_mode_accepts_nested_loop_context_and_shadowed_bindings() {
        let root = temp_root("validation_nested_loop_context");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - groups\n---\n{% for group in groups %}{{ group.name }}{% for item in group.items %}{{ group.name }}={{ item.name }}:{{ loop.depth }}:{{ loop.last }}{% endfor %}{% endfor %}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                strict_undeclared_variables: true,
                ..ComposePolicy::default()
            },
        );
        request.vars_input.insert(
            crate::VariableName::new("groups").unwrap(),
            json!([{ "name": "one", "items": [{ "name": "a" }] }]),
        );

        let report = validate(&request).unwrap();

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn strict_mode_keeps_loop_outside_scope_and_lookalikes_undeclared() {
        let root = temp_root("validation_loop_context_boundaries");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - items\n---\noutside={{ loop.last }}\n{% for item in items %}inside={{ loop.anything }}{% endfor %}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                strict_undeclared_variables: true,
                ..ComposePolicy::default()
            },
        );
        request
            .vars_input
            .insert(crate::VariableName::new("items").unwrap(), json!(["one"]));

        let report = validate(&request).unwrap();

        assert!(!report.ok);
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUndeclaredToken
                && diagnostic.message.contains("loop.last")
        }));
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUndeclaredToken
                && diagnostic.message.contains("loop.anything")
        }));
    }

    #[test]
    fn per_pass_validation_is_independent_for_higher_brace_tokens() {
        let root = temp_root("validation_multipass_independent");
        write_file(
            &root.join("template.md.j2"),
            "---\npass: 2\ndefaults:\n  team: wyvern\n---\n---\npass: 1\ndefaults:\n  task: smoke\n---\nouter={{{ missing_team }}}\ninner={{ task }}\n",
        );

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty(), "{report:?}");
        assert!(report.warnings.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUndeclaredToken
                && diagnostic.message.contains("missing_team")
        }));
        assert!(
            !report.warnings.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::InfoValDefaultUsed
                    && diagnostic.message.contains("variable team not provided")
            }),
            "{report:?}"
        );
    }

    #[test]
    fn strict_mode_errors_on_undeclared_higher_pass_variable() {
        let root = temp_root("validation_multipass_strict");
        write_file(
            &root.join("template.md.j2"),
            "---\npass: 2\n---\n---\npass: 1\n---\nouter={{{ missing_team }}}\ninner={{ task }}\n",
        );

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                strict_undeclared_variables: true,
                ..ComposePolicy::default()
            },
        ))
        .unwrap();

        assert!(!report.ok, "{report:?}");
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUndeclaredToken
                && diagnostic.message.contains("missing_team")
        }));
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
        let mut state = collect_validation_state(&request, &expanded);
        inject_builtin_vars(&mut state, &resolve_result.resolved_path);

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
        let mut state = collect_validation_state(&request, &expanded);
        inject_builtin_vars(&mut state, &resolve_result.resolved_path);

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
    fn builtins_are_available_without_frontmatter_declarations() {
        let root = temp_root("validation_builtins");
        write_file(
            &root.join("template.md.j2"),
            "{{ TEMPLATE_NAME }} {{ HOSTNAME }} {{ USERNAME }} {{ RENDER_DATE }} {{ RENDER_TIMESTAMP }}\n",
        );

        let request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        let report = validate(&request).unwrap();
        assert!(
            !report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValUndeclaredToken)
        );

        let resolve_result = crate::resolve_template_path(&request).unwrap();
        let expanded = crate::expand_includes(
            &resolve_result.resolved_path,
            &request.root,
            &request.policy,
        )
        .unwrap();
        let mut state = collect_validation_state(&request, &expanded);
        inject_builtin_vars(&mut state, &resolve_result.resolved_path);

        assert_eq!(
            state
                .context
                .get(&crate::VariableName::new("TEMPLATE_NAME").unwrap()),
            Some(&json!("template.md.j2"))
        );
        for name in ["HOSTNAME", "USERNAME", "RENDER_DATE", "RENDER_TIMESTAMP"] {
            let variable = crate::VariableName::new(name).unwrap();
            let value = state
                .context
                .get(&variable)
                .and_then(serde_json::Value::as_str);
            assert!(
                value.is_some_and(|value| !value.is_empty()),
                "{name} missing"
            );
            assert_eq!(
                state.variable_sources.get(&variable),
                Some(&crate::VariableSource::Builtin)
            );
        }
    }

    #[test]
    fn builtins_override_defaults_and_can_be_overridden_by_env_and_input() {
        let root = temp_root("validation_builtin_precedence");
        write_file(
            &root.join("report.md.j2"),
            "---\ndefaults:\n  HOSTNAME: default-host\n  USERNAME: default-user\n  RENDER_DATE: 2000-01-01\n---\n{{ HOSTNAME }} {{ USERNAME }} {{ RENDER_DATE }}\n",
        );

        let mut request = request_for_file(&root, "report.md.j2", ComposePolicy::default());
        request.vars_env.insert(
            crate::VariableName::new("HOSTNAME").unwrap(),
            json!("env-host"),
        );
        request.vars_input.insert(
            crate::VariableName::new("USERNAME").unwrap(),
            json!("input-user"),
        );

        let resolve_result = crate::resolve_template_path(&request).unwrap();
        let expanded = crate::expand_includes(
            &resolve_result.resolved_path,
            &request.root,
            &request.policy,
        )
        .unwrap();
        let mut state = collect_validation_state(&request, &expanded);
        inject_builtin_vars(&mut state, &resolve_result.resolved_path);

        assert_eq!(
            state
                .context
                .get(&crate::VariableName::new("HOSTNAME").unwrap()),
            Some(&json!("env-host"))
        );
        assert_eq!(
            state
                .variable_sources
                .get(&crate::VariableName::new("HOSTNAME").unwrap()),
            Some(&crate::VariableSource::Environment)
        );
        assert_eq!(
            state
                .context
                .get(&crate::VariableName::new("USERNAME").unwrap()),
            Some(&json!("input-user"))
        );
        assert_eq!(
            state
                .variable_sources
                .get(&crate::VariableName::new("USERNAME").unwrap()),
            Some(&crate::VariableSource::ExplicitInput)
        );

        let render_date = state
            .context
            .get(&crate::VariableName::new("RENDER_DATE").unwrap())
            .and_then(serde_json::Value::as_str)
            .unwrap();
        assert_ne!(render_date, "2000-01-01");
        assert_eq!(
            state
                .variable_sources
                .get(&crate::VariableName::new("RENDER_DATE").unwrap()),
            Some(&crate::VariableSource::Builtin)
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
    fn missing_included_frontmatter_emits_fixup_warning_for_include() {
        let root = temp_root("validation_missing_included_frontmatter");
        let root_template = root.join("template.md.j2");
        write_file(&root_template, "---\nrequired_variables:\n  - name\n---\n");
        write_file(&root.join("partials/body.md.j2"), "hello {{ name }}\n");

        let warnings = missing_frontmatter_warnings(
            &ResolveResult {
                resolved_path: root_template,
                attempted_paths: Vec::new(),
                ambiguity_candidates: Vec::new(),
            },
            &ExpandedTemplate {
                text: "hello {{ name }}\n".to_owned(),
                resolved_files: vec![
                    root.join("template.md.j2"),
                    root.join("partials/body.md.j2"),
                ],
                frontmatters: vec![
                    (
                        root.join("template.md.j2"),
                        vec![crate::Frontmatter::empty()],
                    ),
                    (root.join("partials/body.md.j2"), Vec::new()),
                ],
                include_chains: BTreeMap::default(),
            },
        );

        assert!(warnings.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValMissingFrontmatter
                && diagnostic
                    .message
                    .contains("included file has no frontmatter")
                && diagnostic.message.contains("partials/body.md.j2")
        }));
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
    fn extra_input_policy_can_warn() {
        let root = temp_root("validation_extra_input_warn");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unknown_variable_policy: UnknownVariablePolicy::Warn,
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
        assert!(report.ok);
        assert!(report.warnings.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValExtraInput
                && diagnostic.severity == DiagnosticSeverity::Warning
        }));
        assert!(report.errors.is_empty());
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
    fn required_variable_path_array_member_id_is_satisfied_by_array_of_objects() {
        let root = temp_root("validation_required_array_member_path");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - sprints.id\n---\n{% for sprint in sprints %}{{ sprint.id }}{% endfor %}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("sprints").unwrap(),
            json!([
                { "id": "S1", "stage": "qa" },
                { "id": "S2", "stage": "merged" }
            ]),
        );

        let report = validate(&request).unwrap();

        assert!(report.ok, "{report:?}");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn missing_nested_field_in_array_member_reports_err_val_missing_nested_field() {
        let root = temp_root("validation_missing_array_member_field");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - sprints.id\n---\n{% for sprint in sprints %}{{ sprint.id }}{% endfor %}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("sprints").unwrap(),
            json!([
                { "id": "S1", "stage": "qa" },
                { "stage": "merged" }
            ]),
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
        let root = temp_root("validation_array_member_shape_mismatch");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - sprints.id\n---\n{% for sprint in sprints %}{{ sprint.id }}{% endfor %}\n",
        );

        let mut request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        request.vars_input.insert(
            crate::VariableName::new("sprints").unwrap(),
            json!([
                { "id": "S1", "stage": "qa" },
                "bad-member"
            ]),
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
    fn discover_tokens_attributes_loop_body_references_to_iterable() {
        let tokens = super::discover_tokens(
            "{% for sprint in sprints %}{{ sprint.id }} {{ report.title }}{% endfor %}",
        );

        assert!(tokens.contains(&crate::VariableName::new("sprints").unwrap()));
        assert!(tokens.contains(&crate::VariableName::new("report.title").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("sprint").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("sprint.id").unwrap()));
    }

    #[test]
    fn discover_tokens_handles_nested_loops_with_separate_scopes() {
        let tokens = super::discover_tokens(
            "{% for sprint in sprints %}{% for finding in sprint_findings %}{{ finding.id }} {{ sprint.title }} {{ report.url }}{% endfor %}{% endfor %}",
        );

        assert!(tokens.contains(&crate::VariableName::new("sprints").unwrap()));
        assert!(tokens.contains(&crate::VariableName::new("sprint_findings").unwrap()));
        assert!(tokens.contains(&crate::VariableName::new("report.url").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("finding").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("finding.id").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("sprint").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("sprint.title").unwrap()));
    }

    #[test]
    fn discover_tokens_scopes_approved_loop_context_names() {
        let tokens = super::discover_tokens(
            "{% for item in items if include_item %}{{ loop }} {{ loop.index }} {{ loop.index0 }} {{ loop.revindex }} {{ loop.revindex0 }} {{ loop.first }} {{ loop.last }} {{ loop.length }} {{ loop.depth }} {{ loop.depth0 }} {{ loop.cycle(\"odd\", \"even\") }} {{ item.name }} {{ caller() }}{% endfor %}",
        );

        for expected in ["items", "include_item", "caller"] {
            assert!(
                tokens.contains(&crate::VariableName::new(expected).unwrap()),
                "missing discovered token {expected}: {tokens:?}"
            );
        }
        for implicit in [
            "loop",
            "loop.index",
            "loop.index0",
            "loop.revindex",
            "loop.revindex0",
            "loop.first",
            "loop.last",
            "loop.length",
            "loop.depth",
            "loop.depth0",
            "loop.cycle",
            "odd",
            "even",
        ] {
            assert!(
                !tokens.contains(&crate::VariableName::new(implicit).unwrap()),
                "unexpected loop-context or literal token {implicit}: {tokens:?}"
            );
        }
    }

    #[test]
    fn discover_tokens_requires_loop_cycle_call_form() {
        let call_form = super::discover_tokens(
            "{% for item in items %}{{ loop.cycle(\"odd\", \"even\") }}{% endfor %}",
        );
        assert!(!call_form.contains(&crate::VariableName::new("loop.cycle").unwrap()));

        let bare_identifier =
            super::discover_tokens("{% for item in items %}{{ loop.cycle }}{% endfor %}");
        assert!(bare_identifier.contains(&crate::VariableName::new("loop.cycle").unwrap()));
    }

    #[test]
    fn discover_tokens_keeps_loop_outside_scope_and_rejects_lookalikes() {
        let tokens = super::discover_tokens(
            "{{ loop.last }} {% for item in items %}{{ loop.anything }} {{ loop.cycle_extra }} {{ item }}{% endfor %}",
        );

        for expected in ["loop.last", "loop.anything", "loop.cycle_extra"] {
            assert!(
                tokens.contains(&crate::VariableName::new(expected).unwrap()),
                "missing ordinary token {expected}: {tokens:?}"
            );
        }
    }

    #[test]
    fn discover_tokens_keeps_nested_iterables_and_shadowing_scoped() {
        let tokens = super::discover_tokens(
            "{% for group in groups %}{% for group in nested_groups %}{{ group.name }} {{ report.url }} {{ loop.last }}{% endfor %}{% endfor %}",
        );

        for expected in ["groups", "nested_groups", "report.url"] {
            assert!(
                tokens.contains(&crate::VariableName::new(expected).unwrap()),
                "missing nested-loop token {expected}: {tokens:?}"
            );
        }
        for bound in ["group", "group.name", "loop.last"] {
            assert!(
                !tokens.contains(&crate::VariableName::new(bound).unwrap()),
                "unexpected bound or implicit token {bound}: {tokens:?}"
            );
        }
    }

    #[test]
    fn discover_tokens_with_brace_count_finds_standard_and_higher_brace_tokens() {
        let double = crate::discovery::discover_tokens_with_brace_count("{{ a }}", 2);
        let triple = crate::discovery::discover_tokens_with_brace_count("{{{ a }}}", 3);

        assert_eq!(double, [crate::VariableName::new("a").unwrap()].into());
        assert_eq!(triple, [crate::VariableName::new("a").unwrap()].into());
    }

    #[test]
    fn discover_tokens_with_brace_count_does_not_match_lower_brace_inside_higher_brace() {
        let tokens =
            crate::discovery::discover_tokens_with_brace_count("{{{ outer }}} {{ inner }}", 3);

        assert_eq!(tokens, [crate::VariableName::new("outer").unwrap()].into());
        assert!(
            crate::discovery::discover_tokens_with_brace_count("{{ a }}", 3).is_empty(),
            "double-brace expression should not be matched by triple-brace discovery"
        );
    }

    #[test]
    fn discover_tokens_with_brace_count_ignores_higher_brace_when_scanning_lower_brace() {
        let tokens =
            crate::discovery::discover_tokens_with_brace_count("{{{ outer }}} {{ inner }}", 2);

        assert_eq!(tokens, [crate::VariableName::new("inner").unwrap()].into());
    }

    #[test]
    fn discover_all_pass_tokens_returns_per_pass_maps() {
        let parsed = parse_template_document(
            "---\npass: 1\n---\n---\npass: 2\n---\n{{ inner }} {{{ outer }}}",
        )
        .unwrap();

        let tokens = super::discover_all_pass_tokens(&parsed);

        assert_eq!(
            tokens.get(&1).cloned().unwrap_or_default(),
            [crate::VariableName::new("inner").unwrap()].into()
        );
        assert_eq!(
            tokens.get(&2).cloned().unwrap_or_default(),
            [crate::VariableName::new("outer").unwrap()].into()
        );
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
