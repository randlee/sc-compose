//! End-to-end composition orchestration.

use std::collections::BTreeMap;

use crate::ComposeError;
use crate::error::ValidationError;
use crate::include::expand_includes;
use crate::observer::{
    CompositionObserver, IncludeOutcomeEvent, NoopObserver, RenderOutcomeEvent,
    ResolveOutcomeEvent, ValidationOutcomeEvent,
};
use crate::renderer::Renderer;
use crate::resolver::resolve_template_path;
use crate::types::{ComposeRequest, ComposeResult, ScalarValue, ValidationReport};

/// Compose a request end to end: resolve, expand includes, validate, render,
/// and assemble output blocks.
///
/// # Errors
///
/// Returns [`ComposeError`] for operational failures and fatal validation or
/// render failures.
pub fn compose(request: &ComposeRequest) -> Result<ComposeResult, ComposeError> {
    let mut observer = NoopObserver;
    compose_with_observer(request, &mut observer)
}

/// Compose a request end to end while emitting observer callbacks.
///
/// # Errors
///
/// Returns [`ComposeError`] for operational failures and fatal validation or
/// render failures.
pub fn compose_with_observer(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
) -> Result<ComposeResult, ComposeError> {
    let resolve_result = match resolve_template_path(request) {
        Ok(result) => {
            observer.on_resolve_outcome(&ResolveOutcomeEvent {
                resolved_path: Some(result.resolved_path.clone()),
                attempted_paths: result.attempted_paths.clone(),
                code: None,
            });
            result
        }
        Err(error) => {
            emit_resolve_error(observer, &error);
            return Err(error);
        }
    };
    let expanded = expand_includes(
        &resolve_result.resolved_path,
        &request.root,
        &request.policy,
    )
    .inspect_err(|error| {
        emit_include_error(observer, error);
    })?;
    observer.on_include_outcome(&IncludeOutcomeEvent {
        resolved_files: expanded.resolved_files.clone(),
        include_chain: Vec::new(),
        code: None,
    });

    let validation_report =
        crate::validation::validate_expanded(request, &expanded, resolve_result.clone());
    observer.on_validation_outcome(&ValidationOutcomeEvent {
        warnings: validation_report.warnings.clone(),
        errors: validation_report.errors.clone(),
    });
    fail_if_invalid(&validation_report)?;
    let validation_state = crate::validation::collect_validation_state(request, &expanded);

    let renderer = Renderer::new();
    let rendered_text = renderer
        .render(&expanded.text, build_render_context(&validation_state))
        .inspect_err(|error| {
            observer.on_render_outcome(&RenderOutcomeEvent {
                rendered_bytes: None,
                code: error.code(),
            });
        })?;
    observer.on_render_outcome(&RenderOutcomeEvent {
        rendered_bytes: Some(rendered_text.len()),
        code: None,
    });
    let rendered_text = assemble_output(
        &rendered_text,
        request.guidance_block.as_deref(),
        request.user_prompt.as_deref(),
    );

    Ok(ComposeResult {
        rendered_text,
        resolved_files: expanded.resolved_files,
        resolve_result,
        variable_sources: validation_state.variable_sources,
        warnings: validation_report.warnings,
    })
}

fn emit_resolve_error(observer: &mut dyn CompositionObserver, error: &ComposeError) {
    if let ComposeError::Resolve(resolve_error) = error {
        observer.on_resolve_outcome(&ResolveOutcomeEvent {
            resolved_path: None,
            attempted_paths: resolve_error.attempted_paths().to_vec(),
            code: resolve_error.code(),
        });
    }
}

fn emit_include_error(observer: &mut dyn CompositionObserver, error: &ComposeError) {
    if let ComposeError::Include(include_error) = error {
        observer.on_include_outcome(&IncludeOutcomeEvent {
            resolved_files: Vec::new(),
            include_chain: include_error.include_chain().to_vec(),
            code: include_error.code(),
        });
    }
}

fn fail_if_invalid(report: &ValidationReport) -> Result<(), ComposeError> {
    if report.errors.is_empty() {
        Ok(())
    } else {
        let first = &report.errors[0];
        let code = first.code;
        Err(ValidationError::new(code, first.message.clone()).into())
    }
}

fn build_render_context(
    state: &crate::validation::ValidationState,
) -> BTreeMap<String, serde_json::Value> {
    state
        .context
        .iter()
        .map(|(key, value)| (key.to_string(), scalar_to_json(value.clone())))
        .collect()
}

fn scalar_to_json(value: ScalarValue) -> serde_json::Value {
    match value {
        ScalarValue::String(value) => serde_json::Value::String(value),
        ScalarValue::Number(value) => serde_json::Value::Number(value),
        ScalarValue::Boolean(value) => serde_json::Value::Bool(value),
        ScalarValue::Null => serde_json::Value::Null,
    }
}

fn assemble_output(
    profile_body: &str,
    guidance_block: Option<&str>,
    user_prompt: Option<&str>,
) -> String {
    let mut blocks = vec![profile_body.trim_end().to_owned()];
    if let Some(guidance) = guidance_block.filter(|value| !value.is_empty()) {
        blocks.push(guidance.to_owned());
    }
    if let Some(prompt) = user_prompt.filter(|value| !value.is_empty()) {
        blocks.push(prompt.to_owned());
    }
    blocks.join("\n\n")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::types::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};
    use crate::{ScalarValue, VariableName, VariableSource, compose};

    #[test]
    fn compose_renders_and_appends_guidance_and_prompt() {
        let root = temp_root("compose_blocks");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}",
        );

        let result = compose(&ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            guidance_block: Some("guidance".to_owned()),
            user_prompt: Some("prompt".to_owned()),
            policy: ComposePolicy::default(),
        })
        .unwrap();

        assert_eq!(result.rendered_text, "hello world\n\nguidance\n\nprompt");
        assert_eq!(
            result
                .variable_sources
                .get(&VariableName::new("name").unwrap()),
            Some(&VariableSource::FrontmatterDefault)
        );
    }

    #[test]
    fn compose_prefers_explicit_input_variable_sources() {
        let root = temp_root("compose_sources");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: default\n---\nhello {{ name }}",
        );

        let mut vars_input = BTreeMap::default();
        vars_input.insert(
            VariableName::new("name").unwrap(),
            ScalarValue::String("explicit".to_owned()),
        );

        let result = compose(&ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input,
            vars_env: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        })
        .unwrap();

        assert_eq!(result.rendered_text, "hello explicit");
        assert_eq!(
            result
                .variable_sources
                .get(&VariableName::new("name").unwrap()),
            Some(&VariableSource::ExplicitInput)
        );
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
