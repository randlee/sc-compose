//! End-to-end composition orchestration.

use std::collections::BTreeMap;
use std::path::Path;

use crate::ComposeError;
use crate::diagnostics::DiagnosticCode;
use crate::error::{ConfigError, ValidationError};
use crate::frontmatter::{Frontmatter, ParsedTemplate};
use crate::include::expand_includes;
use crate::observer::{
    CompositionObserver, IncludeOutcomeEvent, NoopObserver, PassEndEvent, PassStartEvent,
    RenderOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
};
use crate::path_utils::to_forward_slash;
use crate::renderer::Renderer;
use crate::resolver::resolve_template_path;
use crate::types::{ComposeRequest, ComposeResult, InputValue, PassConfig, VariableName};

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
    observer.on_resolve_attempt(&ResolveAttemptEvent {
        template: resolve_attempt_label(request),
    });
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
            notify_resolve_error(observer, &error);
            return Err(error);
        }
    };
    let expanded = expand_includes(
        &resolve_result.resolved_path,
        &request.root,
        &request.policy,
    )
    .inspect_err(|error| {
        notify_include_error(observer, error);
    })?;
    observer.on_include_outcome(&IncludeOutcomeEvent {
        resolved_files: expanded.resolved_files.clone(),
        include_chain: Vec::new(),
        code: None,
    });

    compose_expanded(request, observer, resolve_result, expanded)
}

/// Compose using a template expansion that was already resolved and read.
///
/// This entry point is for callers that perform a preflight expansion before
/// rendering. It emits the normal resolve/include observer events and then
/// reuses the supplied expansion without reading the template files again.
///
/// # Errors
///
/// Returns [`ComposeError`] for fatal validation or render failures.
pub fn compose_with_observer_and_expanded(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
    resolve_result: crate::ResolveResult,
    expanded: crate::ExpandedTemplate,
) -> Result<ComposeResult, ComposeError> {
    observer.on_resolve_attempt(&ResolveAttemptEvent {
        template: resolve_attempt_label(request),
    });
    observer.on_resolve_outcome(&ResolveOutcomeEvent {
        resolved_path: Some(resolve_result.resolved_path.clone()),
        attempted_paths: resolve_result.attempted_paths.clone(),
        code: None,
    });
    observer.on_include_outcome(&IncludeOutcomeEvent {
        resolved_files: expanded.resolved_files.clone(),
        include_chain: Vec::new(),
        code: None,
    });

    compose_expanded(request, observer, resolve_result, expanded)
}

fn compose_expanded(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
    resolve_result: crate::ResolveResult,
    expanded: crate::ExpandedTemplate,
) -> Result<ComposeResult, ComposeError> {
    let (mut validation_report, mut validation_state) =
        crate::validation::validate_expanded(request, &expanded, resolve_result);
    let validation_outcome = ValidationOutcomeEvent {
        warnings: std::mem::take(&mut validation_report.warnings),
        errors: std::mem::take(&mut validation_report.errors),
    };
    observer.on_validation_outcome(&validation_outcome);
    fail_if_invalid(validation_outcome.errors)?;

    let root_passes = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| {
            (path == &validation_report.resolve_result.resolved_path).then(|| passes.clone())
        })
        .unwrap_or_default();
    let parsed = ParsedTemplate::from_parts(root_passes, expanded.text.clone());
    let template_name = resolved_template_name(&validation_report.resolve_result.resolved_path);
    let rendered_text = if parsed.passes().len() > 1 {
        let contexts = build_pass_contexts(
            parsed.passes(),
            &request.policy.passes,
            &mut validation_state,
            &validation_report.resolve_result.resolved_path,
        );
        render_all_with_observer(
            &parsed,
            &contexts,
            &template_name,
            observer,
            crate::resolve_json_escape_mode(
                request.policy.json_escape_mode,
                parsed.frontmatter().and_then(Frontmatter::json_escape_mode),
            ),
        )?
    } else {
        let renderer = Renderer::with_json_escape_mode(crate::resolve_json_escape_mode(
            request.policy.json_escape_mode,
            parsed.frontmatter().and_then(Frontmatter::json_escape_mode),
        ));
        renderer
            .render_named(
                &template_name,
                &expanded.text,
                build_render_context(
                    &mut validation_state,
                    &validation_report.resolve_result.resolved_path,
                ),
            )
            .inspect_err(|error| {
                observer.on_render_outcome(&RenderOutcomeEvent {
                    rendered_bytes: None,
                    code: error.code(),
                });
            })?
    };
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
        resolve_result: validation_report.resolve_result,
        variable_sources: validation_state.variable_sources,
        warnings: validation_outcome.warnings,
        composition_fingerprint: expanded.composition_fingerprint,
    })
}

/// Render all passes in sequence for a parsed multi-pass template.
///
/// # Errors
///
/// Returns [`ComposeError`] when the number of contexts does not match the
/// number of passes, when a context pass number does not match the parsed
/// header pass number, or when rendering any pass fails.
pub fn render_all(
    parsed: &ParsedTemplate,
    contexts: &[(u8, BTreeMap<VariableName, InputValue>)],
) -> Result<String, ComposeError> {
    let mut observer = NoopObserver;
    render_all_with_observer(
        parsed,
        contexts,
        "inline",
        &mut observer,
        crate::JsonEscapeMode::Auto,
    )
}

/// Protect next-higher-brace expressions from lower-brace rendering passes.
#[must_use]
pub fn protect_higher_braces(text: &str, brace_count: usize) -> String {
    let higher_brace_count = brace_count + 1;
    let open_delim = "{".repeat(higher_brace_count);
    let close_delim = "}".repeat(higher_brace_count);

    if !text.contains(&open_delim) {
        return text.to_owned();
    }

    let mut result = String::with_capacity(text.len());
    let mut cursor = 0usize;
    loop {
        let Some(found) = text[cursor..].find(&open_delim) else {
            result.push_str(&text[cursor..]);
            break;
        };
        let absolute_start = cursor + found;
        let after_open = absolute_start + open_delim.len();
        let Some(relative_end) = text[after_open..].find(&close_delim) else {
            result.push_str(&text[cursor..]);
            break;
        };
        let absolute_end = after_open + relative_end + close_delim.len();
        result.push_str(&text[cursor..absolute_start]);
        result.push_str("{% raw %}");
        result.push_str(&text[absolute_start..absolute_end]);
        result.push_str("{% endraw %}");
        cursor = absolute_end;
    }

    result
}

fn resolve_attempt_label(request: &ComposeRequest) -> String {
    match &request.mode {
        crate::types::ComposeMode::Profile { kind, name } => format!("{kind:?}:{name}"),
        crate::types::ComposeMode::File { template_path } => to_forward_slash(template_path),
    }
}

fn resolved_template_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("inline")
        .to_owned()
}

fn notify_resolve_error(observer: &mut dyn CompositionObserver, error: &ComposeError) {
    if let ComposeError::Resolve(resolve_error) = error {
        observer.on_resolve_outcome(&ResolveOutcomeEvent {
            resolved_path: None,
            attempted_paths: resolve_error.attempted_paths().to_vec(),
            code: Some(resolve_error.code()),
        });
    }
}

fn notify_include_error(observer: &mut dyn CompositionObserver, error: &ComposeError) {
    if let ComposeError::Include(include_error) = error {
        observer.on_include_outcome(&IncludeOutcomeEvent {
            resolved_files: Vec::new(),
            include_chain: include_error.include_chain().to_vec(),
            code: Some(include_error.code()),
        });
    }
}

fn fail_if_invalid(errors: Vec<crate::Diagnostic>) -> Result<(), ComposeError> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::from_diagnostics(errors).into())
    }
}

fn build_render_context(
    state: &mut crate::validation::ValidationState,
    template_path: &Path,
) -> BTreeMap<String, serde_json::Value> {
    crate::validation::inject_builtin_vars(state, template_path);
    state
        .context
        .iter()
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect()
}

fn build_pass_contexts(
    passes: &[Frontmatter],
    pass_configs: &[PassConfig],
    state: &mut crate::validation::ValidationState,
    template_path: &Path,
) -> Vec<(u8, BTreeMap<VariableName, InputValue>)> {
    crate::validation::inject_builtin_vars(state, template_path);
    let frontmatter_defaults = passes
        .iter()
        .map(|pass| (pass.pass_number(), pass.defaults().clone()))
        .collect::<BTreeMap<_, _>>();

    if !pass_configs.is_empty() {
        return pass_configs
            .iter()
            .map(|pass| {
                let mut context = frontmatter_defaults
                    .get(&pass.pass_number)
                    .cloned()
                    .unwrap_or_default();
                // Multi-pass contexts currently receive the full flattened validation context.
                // That is intentionally harmless today because each render pass only resolves
                // its own brace width, so same-name values from other passes stay inert unless
                // we ever add a cross-pass surface that ignores delimiter isolation.
                for (name, value) in &state.context {
                    context.insert(name.clone(), value.clone());
                }
                for (name, value) in &pass.defaults {
                    context.insert(name.clone(), value.clone());
                }
                (pass.pass_number, context)
            })
            .collect();
    }

    passes
        .iter()
        .map(|pass| {
            let mut context = pass.defaults().clone();
            // See note above: whole-context merging is loose, but currently inert because
            // lower/higher pass delimiters prevent cross-pass names from resolving here.
            for (name, value) in &state.context {
                context.insert(name.clone(), value.clone());
            }
            (pass.pass_number(), context)
        })
        .collect()
}

fn render_all_with_observer(
    parsed: &ParsedTemplate,
    contexts: &[(u8, BTreeMap<VariableName, InputValue>)],
    template_name: &str,
    observer: &mut dyn CompositionObserver,
    json_escape_mode: crate::JsonEscapeMode,
) -> Result<String, ComposeError> {
    if contexts.len() != parsed.passes().len() {
        return Err(ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!(
                "expected {} render contexts for {} passes, got {}",
                parsed.passes().len(),
                parsed.passes().len(),
                contexts.len()
            ),
        )
        .into());
    }

    let mut body = parsed.body().to_owned();
    for (frontmatter, (context_pass_number, variables)) in
        parsed.passes().iter().zip(contexts.iter())
    {
        let header_pass_number = frontmatter.pass_number();
        if *context_pass_number != header_pass_number {
            return Err(ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!(
                    "render context pass {context_pass_number} does not match header pass {header_pass_number}"
                ),
            )
            .into());
        }

        let brace_count = usize::from(header_pass_number) + 1;
        let open = "{".repeat(brace_count);
        let close = "}".repeat(brace_count);
        let renderer =
            Renderer::with_delimiters_and_json_escape_mode(&open, &close, json_escape_mode)?;
        let protected_body = protect_higher_braces(&body, brace_count);
        let mut merged_variables = frontmatter.defaults().clone();
        for (name, value) in variables {
            merged_variables.insert(name.clone(), value.clone());
        }
        let render_context = merged_variables
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect::<BTreeMap<_, _>>();

        observer.on_pass_start(&PassStartEvent::new(header_pass_number));
        body = renderer
            .render_named(template_name, &protected_body, render_context)
            .inspect_err(|error| {
                observer.on_render_outcome(&RenderOutcomeEvent {
                    rendered_bytes: None,
                    code: error.code(),
                });
            })?;
        observer.on_pass_end(&PassEndEvent::new(header_pass_number));
    }

    Ok(body)
}

/// Combine rendered content with optional guidance and user-prompt blocks.
#[must_use]
pub fn assemble_output(
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

    use serde_json::json;

    use crate::observer::{
        CompositionObserver, IncludeOutcomeEvent, RenderOutcomeEvent, ResolveAttemptEvent,
        ResolveOutcomeEvent, ValidationOutcomeEvent,
    };
    use crate::types::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};
    use crate::{
        ComposeError, DiagnosticCode, VariableName, VariableSource, compose, compose_with_observer,
    };

    #[derive(Default)]
    struct CapturingObserver {
        attempts: Vec<ResolveAttemptEvent>,
        resolve: Vec<ResolveOutcomeEvent>,
        include: Vec<IncludeOutcomeEvent>,
        validation: Vec<ValidationOutcomeEvent>,
        render: Vec<RenderOutcomeEvent>,
    }

    impl CompositionObserver for CapturingObserver {
        fn on_resolve_attempt(&mut self, event: &ResolveAttemptEvent) {
            self.attempts.push(event.clone());
        }

        fn on_resolve_outcome(&mut self, event: &ResolveOutcomeEvent) {
            self.resolve.push(event.clone());
        }

        fn on_include_outcome(&mut self, event: &IncludeOutcomeEvent) {
            self.include.push(event.clone());
        }

        fn on_validation_outcome(&mut self, event: &ValidationOutcomeEvent) {
            self.validation.push(event.clone());
        }

        fn on_render_outcome(&mut self, event: &RenderOutcomeEvent) {
            self.render.push(event.clone());
        }
    }

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
            vars_defaults: BTreeMap::default(),
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
    fn compose_fails_closed_for_an_unbound_reference_under_error_policy() {
        let root = temp_root("compose_unbound_error");
        write_file(&root.join("template.md.j2"), "hello {{ missing }}");

        let error = compose(&ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            vars_defaults: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy {
                unknown_variable_policy: crate::UnknownVariablePolicy::Error,
                ..ComposePolicy::default()
            },
        })
        .unwrap_err();

        assert_eq!(error.code(), Some(DiagnosticCode::ErrValUnboundVariable));
    }

    #[test]
    fn compose_prefers_explicit_input_variable_sources() {
        let root = temp_root("compose_sources");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: default\n---\nhello {{ name }}",
        );

        let mut vars_input = BTreeMap::default();
        vars_input.insert(VariableName::new("name").unwrap(), json!("explicit"));

        let result = compose(&ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input,
            vars_env: BTreeMap::default(),
            vars_defaults: BTreeMap::default(),
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

    #[test]
    fn compose_injects_builtin_variables_and_keeps_override_precedence() {
        let root = temp_root("compose_builtins");
        write_file(
            &root.join("report.md.j2"),
            "---\ndefaults:\n  HOSTNAME: default-host\n  USERNAME: default-user\n---\n{{ TEMPLATE_NAME }}|{{ HOSTNAME }}|{{ USERNAME }}|{{ RENDER_DATE }}|{{ RENDER_TIMESTAMP }}",
        );

        let mut vars_env = BTreeMap::default();
        vars_env.insert(VariableName::new("HOSTNAME").unwrap(), json!("env-host"));
        let mut vars_input = BTreeMap::default();
        vars_input.insert(VariableName::new("USERNAME").unwrap(), json!("cli-user"));

        let result = compose(&ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("report.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input,
            vars_env,
            vars_defaults: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        })
        .unwrap();

        let parts = result.rendered_text.split('|').collect::<Vec<_>>();
        assert_eq!(parts[0], "report.md.j2");
        assert_eq!(parts[1], "env-host");
        assert_eq!(parts[2], "cli-user");
        assert_eq!(parts[3].len(), 10);
        assert!(parts[4].contains('T'));
        assert_eq!(
            result
                .variable_sources
                .get(&VariableName::new("HOSTNAME").unwrap()),
            Some(&VariableSource::Environment)
        );
        assert_eq!(
            result
                .variable_sources
                .get(&VariableName::new("USERNAME").unwrap()),
            Some(&VariableSource::ExplicitInput)
        );
    }

    #[test]
    fn compose_without_observer_remains_fully_functional() {
        let root = temp_root("compose_no_observer");
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
            vars_defaults: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        })
        .unwrap();

        assert_eq!(result.rendered_text, "hello world");
    }

    #[test]
    fn compose_with_observer_emits_success_outcomes() {
        let root = temp_root("compose_observer_success");
        let template_path = PathBuf::from("nested").join("template.md.j2");
        write_file(
            &root.join(&template_path),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}",
        );
        let mut observer = CapturingObserver::default();

        let result = compose_with_observer(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: template_path.clone(),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            &mut observer,
        )
        .unwrap();

        assert_eq!(result.rendered_text, "hello world");
        assert_eq!(observer.attempts.len(), 1);
        assert_eq!(observer.attempts[0].template, "nested/template.md.j2");
        assert_eq!(observer.resolve.len(), 1);
        assert_eq!(observer.include.len(), 1);
        assert_eq!(observer.validation.len(), 1);
        assert_eq!(observer.render.len(), 1);
        assert_eq!(observer.render[0].rendered_bytes, Some("hello world".len()));
    }

    #[test]
    fn compose_with_observer_emits_include_failure() {
        let root = temp_root("compose_observer_include_failure");
        write_file(&root.join("template.md.j2"), "@<missing.md>\n");
        let mut observer = CapturingObserver::default();

        let error = compose_with_observer(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            &mut observer,
        )
        .unwrap_err();

        assert!(matches!(error, ComposeError::Include(_)));
        assert_eq!(observer.resolve.len(), 1);
        assert_eq!(observer.include.len(), 1);
        assert_eq!(
            observer.include[0].code,
            Some(DiagnosticCode::ErrIncludeNotFound)
        );
        assert!(observer.validation.is_empty());
        assert!(observer.render.is_empty());
    }

    #[test]
    fn compose_with_observer_emits_render_failure() {
        let root = temp_root("compose_observer_render_failure");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\n{{ broken",
        );
        let mut observer = CapturingObserver::default();

        let error = compose_with_observer(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            &mut observer,
        )
        .unwrap_err();

        assert!(matches!(error, ComposeError::Render(_)));
        assert_eq!(observer.resolve.len(), 1);
        assert_eq!(observer.include.len(), 1);
        assert_eq!(observer.validation.len(), 1);
        assert_eq!(observer.render.len(), 1);
        assert_eq!(observer.render[0].rendered_bytes, None);
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
