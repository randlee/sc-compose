use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ExpandedTemplate;
use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::discovery::discover_tokens;
use crate::frontmatter::parse_template_document;
use crate::types::{ComposeRequest, UnknownVariablePolicy, VariableName, VariableSource};

use super::ValidationState;

pub(super) fn collect_policy_diagnostics(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
    resolved_path: &Path,
    state: &ValidationState,
) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if expanded.text.trim().is_empty() {
        errors.push(
            Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticCode::ErrValEmpty,
                "template body is empty",
            )
            .with_path(resolved_path.to_path_buf()),
        );
    }

    warnings.extend(missing_frontmatter_warnings_for_path(
        resolved_path,
        expanded,
    ));
    warnings.extend(frontmatter_diagnostics(expanded));
    warnings.extend(default_usage_diagnostics(state));
    errors.extend(super::required_paths::required_path_diagnostics(state));

    for variable in undeclared_referenced_variables(state) {
        let diagnostic = Diagnostic::new(
            if request.policy.strict_undeclared_variables {
                DiagnosticSeverity::Error
            } else {
                DiagnosticSeverity::Warning
            },
            DiagnosticCode::ErrValUndeclaredToken,
            format!("undeclared referenced token: {variable}"),
        )
        .with_path(resolved_path.to_path_buf());

        if request.policy.strict_undeclared_variables {
            errors.push(diagnostic);
        } else {
            warnings.push(diagnostic);
        }
    }

    push_extra_input_diagnostics(request, state, resolved_path, &mut warnings, &mut errors);

    (warnings, errors)
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

pub(super) fn missing_frontmatter_warnings_for_path(
    resolved_path: &Path,
    expanded: &ExpandedTemplate,
) -> Vec<Diagnostic> {
    expanded
        .frontmatters
        .iter()
        .filter_map(|(path, frontmatters)| {
            if !frontmatters.is_empty() || !file_references_variables(path) {
                return None;
            }
            let message = if path == resolved_path {
                format!(
                    "root template has no frontmatter; run `sc-compose frontmatter-init {}`",
                    resolved_path.display()
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
