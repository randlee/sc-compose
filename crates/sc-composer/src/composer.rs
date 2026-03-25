//! End-to-end composition orchestration.

use std::collections::BTreeMap;

use crate::ComposeError;
use crate::DiagnosticCode;
use crate::error::{ConfigError, ValidationError};
use crate::include::expand_includes;
use crate::pipeline::{Document, Parsed, assemble_output_blocks};
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
    let resolve_result = resolve_template_path(request)?;
    let raw_text = std::fs::read_to_string(&resolve_result.resolved_path).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!(
                "failed to read template source: {}",
                resolve_result.resolved_path.display()
            ),
        )
        .with_source(error)
    })?;
    let parsed_document = Document::<Parsed>::new(raw_text);
    let expanded = expand_includes(
        &resolve_result.resolved_path,
        &request.root,
        &request.policy,
    )?;
    let expanded_document = parsed_document.into_expanded(expanded.text.clone());
    let validation_report =
        crate::validation::validate_expanded(request, &resolve_result, &expanded);
    fail_if_invalid(&validation_report)?;
    let validation_state = crate::validation::collect_validation_state(request, &expanded);

    let validated_document = expanded_document.into_validated();
    let rendered_text = Renderer.render(
        validated_document.body(),
        build_render_context(&validation_state),
    )?;
    let rendered_document = validated_document.into_rendered(rendered_text);
    let rendered_text = assemble_output_blocks(
        rendered_document,
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
