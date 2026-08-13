use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ExpandedTemplate;
use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::discovery::discover_tokens;
use crate::frontmatter::parse_template_document;
use crate::renderer::JSON_LEGACY_WARNING;
use crate::strip_template_suffix;
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
    let (json_mode_warnings, json_mode_errors) =
        json_mode_diagnostics(request, expanded, resolved_path, state);
    warnings.extend(json_mode_warnings);
    errors.extend(json_mode_errors);
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
    push_unbound_variable_diagnostics(request, state, resolved_path, &mut warnings, &mut errors);

    (warnings, errors)
}

fn json_mode_diagnostics(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
    resolved_path: &Path,
    state: &ValidationState,
) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let root_frontmatter = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| (path == resolved_path).then(|| passes.first()))
        .flatten();
    let declared_mode =
        root_frontmatter.and_then(crate::frontmatter::Frontmatter::json_escape_mode);
    let mode_is_declared = request.policy.json_escape_mode.is_some() || declared_mode.is_some();

    if !is_json_template_path(resolved_path) {
        if mode_is_declared {
            errors.push(
                Diagnostic::new(
                    DiagnosticSeverity::Error,
                    DiagnosticCode::ErrJsonEscapeModeNonJson,
                    "JSON escape mode is only valid for JSON templates",
                )
                .with_path(resolved_path.to_path_buf()),
            );
        }
        return (warnings, errors);
    }

    let effective_mode =
        crate::resolve_json_escape_mode(request.policy.json_escape_mode, declared_mode);
    errors.extend(json_mode_include_conflict_diagnostics(
        expanded,
        resolved_path,
        effective_mode,
    ));

    let legacy_mode = matches!(effective_mode, crate::JsonEscapeMode::Legacy);
    let quoted_expressions = quoted_json_placeholder_expressions(&expanded.text);
    if legacy_mode || !quoted_expressions.is_empty() {
        warnings.push(
            Diagnostic::new(
                DiagnosticSeverity::Warning,
                DiagnosticCode::WarnJsonLegacyEscapeMode,
                JSON_LEGACY_WARNING,
            )
            .with_path(resolved_path.to_path_buf()),
        );
    }

    if legacy_mode {
        for expression in quoted_expressions {
            let name = expression;
            let Ok(name) = VariableName::new(&name) else {
                continue;
            };
            if state
                .context
                .get(&name)
                .is_some_and(|value| !value.is_string())
            {
                errors.push(
                    Diagnostic::new(
                        DiagnosticSeverity::Error,
                        DiagnosticCode::ErrJsonLegacyNonString,
                        format!(
                            "legacy JSON escape mode requires a string value for quoted placeholder `{name}`"
                        ),
                    )
                    .with_path(resolved_path.to_path_buf()),
                );
            }
        }
    }

    (warnings, errors)
}

fn json_mode_include_conflict_diagnostics(
    expanded: &ExpandedTemplate,
    resolved_path: &Path,
    root_mode: crate::JsonEscapeMode,
) -> Vec<Diagnostic> {
    expanded
        .frontmatters
        .iter()
        .filter_map(|(path, frontmatters)| {
            if path == resolved_path {
                return None;
            }
            let included_mode = frontmatters
                .iter()
                .find_map(crate::frontmatter::Frontmatter::json_escape_mode)?;
            if included_mode == root_mode {
                return None;
            }

            let include_chain = expanded
                .include_chains
                .get(path)
                .cloned()
                .unwrap_or_default();
            Some(
                Diagnostic::new(
                    DiagnosticSeverity::Error,
                    DiagnosticCode::ErrJsonModeIncludeConflict,
                    format!(
                        "included template JSON escape mode conflicts with root: root `{}` uses effective mode `{}`, but included template `{}` declares `{}`; included templates must match the root mode",
                        resolved_path.display(),
                        json_mode_name(root_mode),
                        path.display(),
                        json_mode_name(included_mode),
                    ),
                )
                .with_path(resolved_path.to_path_buf())
                .with_include_chain(include_chain),
            )
        })
        .collect()
}

const fn json_mode_name(mode: crate::JsonEscapeMode) -> &'static str {
    match mode {
        crate::JsonEscapeMode::Legacy => "legacy",
        crate::JsonEscapeMode::Auto => "auto",
    }
}

fn is_json_template_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let stripped = strip_template_suffix(file_name);
    Path::new(stripped)
        .extension()
        .and_then(|extension| extension.to_str())
        == Some("json")
}

fn quoted_json_placeholder_expressions(body: &str) -> Vec<String> {
    let mut expressions = Vec::new();
    let mut search_from = 0;
    while let Some(relative_open) = body[search_from..].find("{{") {
        let open = search_from + relative_open;
        let Some(relative_close) = body[open + 2..].find("}}") else {
            break;
        };
        let close = open + 2 + relative_close + 2;
        let before = body[..open]
            .chars()
            .rev()
            .find(|character| !character.is_whitespace());
        let after = body[close..]
            .chars()
            .find(|character| !character.is_whitespace());
        if before == Some('"') && after == Some('"') {
            let expression = body[open + 2..close - 2].trim();
            if !expression.is_empty() {
                expressions.push(expression.to_owned());
            }
        }
        search_from = close;
    }
    expressions
}

fn push_unbound_variable_diagnostics(
    request: &ComposeRequest,
    state: &ValidationState,
    resolved_path: &Path,
    warnings: &mut Vec<Diagnostic>,
    errors: &mut Vec<Diagnostic>,
) {
    let policy = request
        .policy
        .unbound_variable_policy
        .unwrap_or(request.policy.unknown_variable_policy);
    if matches!(policy, UnknownVariablePolicy::Ignore) {
        return;
    }

    let referenced = per_pass_referenced_variables(state).map_or_else(
        || state.referenced_variables.clone(),
        |by_pass| by_pass.values().flatten().cloned().collect(),
    );
    for variable in referenced {
        if is_builtin_variable(&variable)
            || super::required_paths::is_bound_path(&state.context, &variable)
        {
            continue;
        }

        let diagnostic = Diagnostic::new(
            match policy {
                UnknownVariablePolicy::Error => DiagnosticSeverity::Error,
                UnknownVariablePolicy::Warn => DiagnosticSeverity::Warning,
                UnknownVariablePolicy::Ignore => unreachable!(),
            },
            DiagnosticCode::ErrValUnboundVariable,
            format!("unbound variable: {variable}"),
        )
        .with_path(resolved_path.to_path_buf());

        match policy {
            UnknownVariablePolicy::Error => errors.push(diagnostic),
            UnknownVariablePolicy::Warn => warnings.push(diagnostic),
            UnknownVariablePolicy::Ignore => unreachable!(),
        }
    }
}

fn is_builtin_variable(variable: &VariableName) -> bool {
    super::BUILTIN_VARIABLE_NAMES
        .iter()
        .any(|name| *name == variable.as_str())
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
            if !frontmatters.is_empty() || !file_references_variables(path, expanded) {
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

fn file_references_variables(path: &Path, expanded: &ExpandedTemplate) -> bool {
    let Some(raw) = expanded.source_texts.get(path) else {
        return false;
    };
    let Ok(parsed) = parse_template_document(raw) else {
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
    use crate::{DiagnosticCode, DiagnosticSeverity, ExpandedTemplate, validate};

    use super::missing_frontmatter_warnings_for_path;

    #[test]
    fn default_mode_preserves_undeclared_tokens_as_warnings() {
        let root = temp_root("diagnostics_default_undeclared");
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
                .any(|diagnostic| { diagnostic.code == DiagnosticCode::ErrValUndeclaredToken })
        );
    }

    #[test]
    fn unknown_error_policy_rejects_referenced_but_unbound_variables() {
        let root = temp_root("diagnostics_unbound_error");
        write_file(
            &root.join("template.md.j2"),
            "Task: {{ bound }}\nMissing: {{ missing }}\n",
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
            .insert(crate::VariableName::new("bound").unwrap(), json!("hello"));

        let report = validate(&request).unwrap();

        assert!(!report.ok, "unbound reference was accepted: {report:?}");
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUnboundVariable
                && diagnostic.message.contains("missing")
        }));
        assert!(!report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUnboundVariable
                && diagnostic.message.contains("unbound variable: bound")
        }));
    }

    #[test]
    fn unknown_warn_policy_distinguishes_referenced_but_unbound_variables() {
        let root = temp_root("diagnostics_unbound_warn");
        write_file(&root.join("template.md.j2"), "Missing: {{ missing }}\n");

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unknown_variable_policy: UnknownVariablePolicy::Warn,
                ..ComposePolicy::default()
            },
        ))
        .unwrap();

        assert!(
            report.ok,
            "warn policy should remain renderable: {report:?}"
        );
        assert!(report.warnings.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUnboundVariable
                && diagnostic.message.contains("missing")
        }));
    }

    #[test]
    fn explicit_unbound_policy_is_independent_of_extra_input_policy() {
        let root = temp_root("diagnostics_unbound_policy_independent");
        write_file(&root.join("template.md.j2"), "Missing: {{ missing }}\n");

        let mut error_request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unknown_variable_policy: UnknownVariablePolicy::Ignore,
                unbound_variable_policy: Some(UnknownVariablePolicy::Error),
                ..ComposePolicy::default()
            },
        );
        error_request
            .vars_input
            .insert(crate::VariableName::new("extra").unwrap(), json!("value"));
        let error_report = validate(&error_request).unwrap();
        assert!(
            error_report
                .errors
                .iter()
                .any(|diagnostic| { diagnostic.code == DiagnosticCode::ErrValUnboundVariable })
        );
        assert!(
            !error_report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValExtraInput)
        );

        let ignore_report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unknown_variable_policy: UnknownVariablePolicy::Error,
                unbound_variable_policy: Some(UnknownVariablePolicy::Ignore),
                ..ComposePolicy::default()
            },
        ))
        .unwrap();
        assert!(
            !ignore_report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValUnboundVariable)
        );
    }

    #[test]
    fn bound_defaults_and_locals_are_not_reported_as_unbound() {
        let root = temp_root("diagnostics_bound_scopes");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  fallback: default\nrequired_variables:\n  - items\n---\n{% for item in items %}{{ item.name }}{% endfor %}{% set local = fallback %}{{ local }}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                unbound_variable_policy: Some(UnknownVariablePolicy::Error),
                ..ComposePolicy::default()
            },
        );
        request.vars_input.insert(
            crate::VariableName::new("items").unwrap(),
            json!([{ "name": "one" }]),
        );

        let report = validate(&request).unwrap();

        assert!(report.ok, "bound values were reported missing: {report:?}");
        assert!(
            !report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValUnboundVariable)
        );
    }

    #[test]
    fn unbound_and_undeclared_axes_are_reported_independently() {
        let root = temp_root("diagnostics_unbound_vs_undeclared");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - declared_missing\n---\n{{ declared_missing }} {{ supplied_undeclared }}\n",
        );

        let mut request = request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy {
                strict_undeclared_variables: true,
                unbound_variable_policy: Some(UnknownVariablePolicy::Error),
                ..ComposePolicy::default()
            },
        );
        request.vars_input.insert(
            crate::VariableName::new("supplied_undeclared").unwrap(),
            json!("present"),
        );

        let report = validate(&request).unwrap();

        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUnboundVariable
                && diagnostic.message.contains("declared_missing")
        }));
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUndeclaredToken
                && diagnostic.message.contains("supplied_undeclared")
        }));
        assert!(!report.errors.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValUnboundVariable
                && diagnostic.message.contains("supplied_undeclared")
        }));
    }

    #[test]
    fn strict_mode_fails_on_undeclared_tokens() {
        let root = temp_root("diagnostics_strict_undeclared");
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
    fn missing_root_frontmatter_emits_fixup_warning() {
        let root = temp_root("diagnostics_missing_frontmatter");
        write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert!(report.warnings.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValMissingFrontmatter
                && diagnostic.message.contains("sc-compose frontmatter-init")
        }));
    }

    #[test]
    fn missing_frontmatter_in_diamond_include_is_reported_once_per_file() {
        let root = temp_root("diagnostics_missing_frontmatter_diamond");
        let leaf_path = root.join("leaf.md.j2");
        write_file(
            &root.join("template.md.j2"),
            "---\n---\n@<parent-a.md.j2>\n@<parent-b.md.j2>\n",
        );
        write_file(&root.join("parent-a.md.j2"), "---\n---\n@<leaf.md.j2>\n");
        write_file(&root.join("parent-b.md.j2"), "---\n---\n@<leaf.md.j2>\n");
        write_file(&leaf_path, "hello {{ name }}\n");
        let leaf = leaf_path.canonicalize().unwrap();

        let request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        let resolved = crate::resolve_template_path(&request).unwrap();
        let expanded =
            crate::expand_includes(&resolved.resolved_path, &request.root, &request.policy)
                .unwrap();

        assert_eq!(
            expanded
                .resolved_files
                .iter()
                .filter(|path| *path == &leaf)
                .count(),
            1
        );
        assert!(expanded.source_texts.contains_key(&leaf));

        let report = validate(&request).unwrap();
        assert_eq!(
            report
                .warnings
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == DiagnosticCode::ErrValMissingFrontmatter
                        && diagnostic.message.contains("leaf.md.j2")
                })
                .count(),
            1,
            "diamond include emitted duplicate missing-frontmatter diagnostics: {report:?}"
        );
    }

    #[test]
    fn missing_frontmatter_in_single_include_is_reported_once() {
        let root = temp_root("diagnostics_missing_frontmatter_single");
        write_file(&root.join("template.md.j2"), "---\n---\n@<leaf.md.j2>\n");
        write_file(&root.join("leaf.md.j2"), "hello {{ name }}\n");

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert_eq!(
            report
                .warnings
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == DiagnosticCode::ErrValMissingFrontmatter
                        && diagnostic.message.contains("leaf.md.j2")
                })
                .count(),
            1,
            "single include emitted an unexpected missing-frontmatter count: {report:?}"
        );
    }

    #[test]
    fn missing_included_frontmatter_emits_fixup_warning_for_include() {
        let root = temp_root("diagnostics_missing_included_frontmatter");
        let root_template = root.join("template.md.j2");
        write_file(&root_template, "---\nrequired_variables:\n  - name\n---\n");
        write_file(
            &root.join("partials").join("body.md.j2"),
            "hello {{ name }}\n",
        );

        let warnings = missing_frontmatter_warnings_for_path(
            &root_template,
            &ExpandedTemplate {
                text: "hello {{ name }}\n".to_owned(),
                resolved_files: vec![
                    root.join("template.md.j2"),
                    root.join("partials").join("body.md.j2"),
                ],
                frontmatters: vec![
                    (
                        root.join("template.md.j2"),
                        vec![crate::Frontmatter::empty()],
                    ),
                    (root.join("partials").join("body.md.j2"), Vec::new()),
                ],
                include_chains: BTreeMap::default(),
                source_texts: [(
                    root.join("partials").join("body.md.j2"),
                    "hello {{ name }}\n".to_owned(),
                )]
                .into_iter()
                .collect(),
                composition_fingerprint: None,
            },
        );

        let included_name = Path::new("partials").join("body.md.j2");
        let included_name = included_name.to_string_lossy();
        assert!(warnings.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValMissingFrontmatter
                && diagnostic
                    .message
                    .contains("included file has no frontmatter")
                && diagnostic.message.contains(included_name.as_ref())
        }));
    }

    #[test]
    fn missing_included_frontmatter_uses_cached_source_after_disk_mutation() {
        let root = temp_root("diagnostics_cached_source_text");
        let root_template = root.join("template.md.j2");
        let included = root.join("partials/body.md.j2");
        write_file(
            &root_template,
            "---\nname: template\n---\n@<partials/body.md.j2>\n",
        );
        write_file(&included, "hello {{ name }}\n");

        let request = request_for_file(&root, "template.md.j2", ComposePolicy::default());
        let resolved = crate::resolve_template_path(&request).unwrap();
        let expanded =
            crate::expand_includes(&resolved.resolved_path, &request.root, &request.policy)
                .unwrap();

        write_file(&included, "plain text after expansion\n");

        let warnings = missing_frontmatter_warnings_for_path(&resolved.resolved_path, &expanded);

        let included_name = Path::new("partials").join("body.md.j2");
        let included_name = included_name.to_string_lossy();
        assert!(warnings.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ErrValMissingFrontmatter
                && diagnostic
                    .message
                    .contains("included file has no frontmatter")
                && diagnostic.message.contains(included_name.as_ref())
        }));
    }

    #[test]
    fn extra_input_policy_can_error() {
        let root = temp_root("diagnostics_extra_input");
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
        let root = temp_root("diagnostics_extra_input_warn");
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
        let root = temp_root("diagnostics_input_defaults_known");
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
        let root = temp_root("diagnostics_input_defaults_only_default");
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
        assert!(report.warnings.iter().any(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Info
                && diagnostic.code == DiagnosticCode::InfoValDefaultUsed
                && diagnostic
                    .message
                    .contains("variable assignee not provided")
                && diagnostic.message.contains("\"teammate\"")
        }));
    }

    #[test]
    fn extra_nested_fields_are_ignored_by_top_level_extra_input_policy() {
        let root = temp_root("diagnostics_extra_nested_fields");
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
            json!({"number": 43, "url": "https://example.test/pr/43", "status": "open"}),
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
    fn empty_template_body_emits_empty_code() {
        let root = temp_root("diagnostics_empty_body");
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

    #[test]
    fn public_validate_preserves_default_before_undeclared_diagnostics() {
        let root = temp_root("diagnostics_default_order");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  known: fallback\n---\n{{ known }} {{ missing }}\n",
        );

        let report = validate(&request_for_file(
            &root,
            "template.md.j2",
            ComposePolicy::default(),
        ))
        .unwrap();

        assert_eq!(report.warnings[0].code, DiagnosticCode::InfoValDefaultUsed);
        assert_eq!(
            report.warnings[1].code,
            DiagnosticCode::ErrValUndeclaredToken
        );
    }

    #[test]
    fn public_validate_preserves_extra_input_policy_after_undeclared_diagnostics() {
        let root = temp_root("diagnostics_extra_order");
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables: []\n---\nhello {{ missing }}\n",
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
            .insert(crate::VariableName::new("extra").unwrap(), json!("value"));

        let report = validate(&request).unwrap();
        assert_eq!(
            report.warnings[0].code,
            DiagnosticCode::ErrValUndeclaredToken
        );
        assert_eq!(report.warnings[1].code, DiagnosticCode::ErrValExtraInput);
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
