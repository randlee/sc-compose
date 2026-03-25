//! Variable discovery and validation semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticSeverity};
use crate::frontmatter::Frontmatter;
use crate::include::expand_includes;
use crate::resolver::resolve_template_path;
use crate::types::{
    ComposeRequest, ScalarValue, UnknownVariablePolicy, ValidationReport,
    VariableName,
};
use crate::{ComposeError, ExpandedTemplate};

#[derive(Debug, Default)]
struct ValidationState {
    context: BTreeMap<VariableName, ScalarValue>,
    required_origins: BTreeMap<VariableName, PathBuf>,
    declared_variables: BTreeSet<VariableName>,
    referenced_variables: BTreeSet<VariableName>,
}

/// Validate a compose request without rendering output.
///
/// # Errors
///
/// Returns [`ComposeError`] when resolution or include expansion fails.
pub fn validate(request: &ComposeRequest) -> Result<ValidationReport, ComposeError> {
    let resolve_result = resolve_template_path(request)?;
    let expanded = expand_includes(
        &resolve_result.resolved_path,
        &request.root,
        &request.policy,
    )?;
    let state = collect_validation_state(request, &expanded);

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

    for (variable, origin) in &state.required_origins {
        if !state.context.contains_key(variable) {
            errors.push(
                Diagnostic::new(
                    DiagnosticSeverity::Error,
                    DiagnosticCode::ErrValMissingRequired,
                    format!("missing required variable: {variable}"),
                )
                .with_path(origin.clone()),
            );
        }
    }

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

    let declared_or_referenced = state
        .declared_variables
        .union(&state.referenced_variables)
        .cloned()
        .collect::<BTreeSet<_>>();
    let provided_variables = request
        .vars_input
        .keys()
        .chain(request.vars_env.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
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
        .with_path(resolve_result.resolved_path.clone());

        match request.policy.unknown_variable_policy {
            UnknownVariablePolicy::Error => errors.push(diagnostic),
            UnknownVariablePolicy::Warn => warnings.push(diagnostic),
            UnknownVariablePolicy::Ignore => {}
        }
    }

    Ok(ValidationReport {
        ok: errors.is_empty(),
        warnings,
        errors,
        resolve_result,
    })
}

fn collect_validation_state(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
) -> ValidationState {
    let mut state = ValidationState::default();

    for (path, frontmatter) in &expanded.frontmatters {
        if let Some(frontmatter) = frontmatter {
            merge_frontmatter(path, frontmatter, &mut state);
        }
    }

    for (name, value) in &request.vars_input {
        state.context.insert(name.clone(), value.clone());
    }
    for (name, value) in &request.vars_env {
        state
            .context
            .entry(name.clone())
            .or_insert_with(|| value.clone());
    }

    state.referenced_variables = discover_tokens(&expanded.text);
    state
}

fn merge_frontmatter(
    path: &Path,
    frontmatter: &Frontmatter,
    state: &mut ValidationState,
) {
    for variable in frontmatter.required_variables() {
        state
            .required_origins
            .entry(variable.clone())
            .or_insert_with(|| path.to_path_buf());
        state.declared_variables.insert(variable.clone());
    }

    for (variable, value) in frontmatter.defaults() {
        state.declared_variables.insert(variable.clone());
        state
            .context
            .entry(variable.clone())
            .or_insert_with(|| value.clone());
    }
}

fn discover_tokens(text: &str) -> BTreeSet<VariableName> {
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

    use crate::types::{
        ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot,
        UnknownVariablePolicy,
    };
    use crate::{DiagnosticCode, ScalarValue};

    use super::{collect_validation_state, validate};

    #[test]
    fn default_mode_preserves_undeclared_tokens_as_warnings() {
        let root = temp_root("validation_default_undeclared");
        write_file(&root.join("template.md.j2"), "hello {{ name }}\n");

        let report = validate(&request_for_file(&root, "template.md.j2", ComposePolicy::default()))
            .unwrap();

        assert!(report.ok);
        assert!(report.errors.is_empty());
        assert_eq!(report.warnings[0].code, DiagnosticCode::ErrValUndeclaredToken);
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
        let expanded =
            crate::expand_includes(&resolve_result.resolved_path, &request.root, &request.policy)
                .unwrap();
        let state = collect_validation_state(&request, &expanded);

        assert_eq!(
            state.context.get(&crate::VariableName::new("name").unwrap()),
            Some(&ScalarValue::String("parent".to_owned()))
        );
        assert!(state
            .required_origins
            .contains_key(&crate::VariableName::new("name").unwrap()));
        assert_eq!(
            state.context.get(&crate::VariableName::new("child_only").unwrap()),
            Some(&ScalarValue::String("present".to_owned()))
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
            .insert(crate::VariableName::new("name").unwrap(), ScalarValue::String("world".to_owned()));
        request
            .vars_input
            .insert(crate::VariableName::new("extra").unwrap(), ScalarValue::String("value".to_owned()));

        let report = validate(&request).unwrap();
        assert!(!report.ok);
        assert!(report
            .errors
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ErrValExtraInput));
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
        let root = std::env::temp_dir().join(format!(
            "sc-compose-{label}-{}-{nanos}",
            std::process::id()
        ));
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
