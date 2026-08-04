//! Variable discovery and validation semantics.

mod diagnostics;
mod required_paths;
mod state;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::ExpandedTemplate;
use crate::types::{ComposeRequest, InputValue, ValidationReport, VariableName, VariableSource};

/// Built-in render-context variable names injected for every render.
pub const BUILTIN_VARIABLE_NAMES: [&str; 5] = [
    "TEMPLATE_NAME",
    "HOSTNAME",
    "USERNAME",
    "RENDER_DATE",
    "RENDER_TIMESTAMP",
];

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

pub(crate) use state::{collect_validation_state, inject_builtin_vars};

pub(crate) fn validate_expanded(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
    resolve_result: crate::ResolveResult,
) -> (ValidationReport, ValidationState) {
    let state = collect_validation_state(request, expanded);

    let (warnings, errors) = diagnostics::collect_policy_diagnostics(
        request,
        expanded,
        &resolve_result.resolved_path,
        &state,
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
    use crate::{ExpandedTemplate, parse_template_document};

    use super::diagnostics::missing_frontmatter_warnings_for_path;
    use super::{collect_validation_state, inject_builtin_vars};

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

        let warnings = missing_frontmatter_warnings_for_path(
            &root_template,
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
        let tokens = crate::discovery::discover_tokens(
            "{% for sprint in sprints %}{{ sprint.id }} {{ report.title }}{% endfor %}",
        );

        assert!(tokens.contains(&crate::VariableName::new("sprints").unwrap()));
        assert!(tokens.contains(&crate::VariableName::new("report.title").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("sprint").unwrap()));
        assert!(!tokens.contains(&crate::VariableName::new("sprint.id").unwrap()));
    }

    #[test]
    fn discover_tokens_handles_nested_loops_with_separate_scopes() {
        let tokens = crate::discovery::discover_tokens(
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
        let tokens = crate::discovery::discover_tokens(
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
        let call_form = crate::discovery::discover_tokens(
            "{% for item in items %}{{ loop.cycle(\"odd\", \"even\") }}{% endfor %}",
        );
        assert!(!call_form.contains(&crate::VariableName::new("loop.cycle").unwrap()));

        let bare_identifier = crate::discovery::discover_tokens(
            "{% for item in items %}{{ loop.cycle }}{% endfor %}",
        );
        assert!(bare_identifier.contains(&crate::VariableName::new("loop.cycle").unwrap()));
    }

    #[test]
    fn discover_tokens_keeps_loop_outside_scope_and_rejects_lookalikes() {
        let tokens = crate::discovery::discover_tokens(
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
        let tokens = crate::discovery::discover_tokens(
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

        let tokens = crate::discovery::discover_all_pass_tokens(&parsed);

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
