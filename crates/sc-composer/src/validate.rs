//! Public validation entrypoint.

use std::collections::BTreeMap;

use crate::observer::{
    CompositionObserver, IncludeOutcomeEvent, NoopObserver, ResolveAttemptEvent,
    ResolveOutcomeEvent, ValidationOutcomeEvent,
};
use crate::path_utils::to_forward_slash;
use crate::{ComposeError, ComposeRequest, ExpandedTemplate, ValidationReport};

/// Validate a compose request without rendering output.
///
/// # Errors
///
/// Returns [`ComposeError`] when resolution or include expansion fails.
pub fn validate(request: &ComposeRequest) -> Result<ValidationReport, ComposeError> {
    let mut observer = NoopObserver;
    validate_with_observer(request, &mut observer)
}

/// Validate a compose request while emitting observer callbacks.
///
/// # Errors
///
/// Returns [`ComposeError`] when resolution or include expansion fails.
pub fn validate_with_observer(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
) -> Result<ValidationReport, ComposeError> {
    validate_with_observer_and_delimiters(request, observer, None)
}

/// Validate a compose request using caller-provided variable delimiters.
///
/// The delimiters must match those used for the subsequent render operation.
/// Statement delimiters remain the standard Jinja `{% ... %}` delimiters.
///
/// # Errors
///
/// Returns [`ComposeError`] when resolution or include expansion fails.
pub fn validate_with_observer_and_delimiters(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
    variable_delimiters: Option<(&str, &str)>,
) -> Result<ValidationReport, ComposeError> {
    validate_with_observer_and_delimiters_with_expansion(request, observer, variable_delimiters)
        .map(|(report, _)| report)
}

/// Validate a compose request and return the exact expansion used for validation.
///
/// Callers that render after validation can pass the returned expansion to a
/// rendering operation to avoid resolving and reading includes a second time.
///
/// # Errors
///
/// Returns [`ComposeError`] when resolution or include expansion fails.
pub fn validate_with_observer_and_delimiters_with_expansion(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
    variable_delimiters: Option<(&str, &str)>,
) -> Result<(ValidationReport, ExpandedTemplate), ComposeError> {
    observer.on_resolve_attempt(&ResolveAttemptEvent {
        template: match &request.mode {
            crate::types::ComposeMode::Profile { kind, name } => format!("{kind:?}:{name}"),
            crate::types::ComposeMode::File { template_path } => to_forward_slash(template_path),
        },
    });
    let resolve_result = crate::resolve_template_path(request).inspect_err(|error| {
        if let ComposeError::Resolve(resolve_error) = &error {
            observer.on_resolve_outcome(&ResolveOutcomeEvent {
                resolved_path: None,
                attempted_paths: resolve_error.attempted_paths().to_vec(),
                code: Some(resolve_error.code()),
            });
        }
    })?;
    observer.on_resolve_outcome(&ResolveOutcomeEvent {
        resolved_path: Some(resolve_result.resolved_path.clone()),
        attempted_paths: resolve_result.attempted_paths.clone(),
        code: None,
    });

    let expanded = crate::expand_includes(
        &resolve_result.resolved_path,
        &request.root,
        &request.policy,
    )
    .inspect_err(|error| {
        if let ComposeError::Include(include_error) = &error {
            observer.on_include_outcome(&IncludeOutcomeEvent {
                resolved_files: Vec::new(),
                include_chain: include_error.include_chain().to_vec(),
                code: Some(include_error.code()),
            });
        }
    })?;
    observer.on_include_outcome(&IncludeOutcomeEvent {
        resolved_files: expanded.resolved_files.clone(),
        include_chain: Vec::new(),
        code: None,
    });

    let (mut report, _) = crate::validation::validate_expanded_with_delimiters(
        request,
        &expanded,
        resolve_result,
        variable_delimiters,
    );
    let event = ValidationOutcomeEvent {
        warnings: std::mem::take(&mut report.warnings),
        errors: std::mem::take(&mut report.errors),
    };
    observer.on_validation_outcome(&event);
    report.warnings = event.warnings;
    report.errors = event.errors;

    Ok((report, expanded))
}

/// Validate an already-expanded template with the variable delimiters it will
/// render with, and return the merged render context.
///
/// Callers that rewrite the expanded text lexically before rendering (for
/// example to fold a second expression syntax into these delimiters) validate
/// the exact text they render, in one token scan. The context holds the
/// frontmatter defaults of every expanded file, then environment values, then
/// caller input; built-in render variables are not injected.
#[must_use]
pub fn validate_expanded_with_context(
    request: &ComposeRequest,
    expanded: &ExpandedTemplate,
    resolve_result: crate::ResolveResult,
    variable_delimiters: (&str, &str),
) -> (ValidationReport, BTreeMap<String, crate::InputValue>) {
    let (report, state) = crate::validation::validate_expanded_with_delimiters(
        request,
        expanded,
        resolve_result,
        Some(variable_delimiters),
    );
    let context = state
        .context
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect();
    (report, context)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::observer::{
        CompositionObserver, IncludeOutcomeEvent, ResolveAttemptEvent, ResolveOutcomeEvent,
        ValidationOutcomeEvent,
    };
    use crate::types::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};
    use crate::{DiagnosticCode, validate_with_observer};

    #[derive(Default)]
    struct CapturingObserver {
        attempts: Vec<ResolveAttemptEvent>,
        resolve: Vec<ResolveOutcomeEvent>,
        include: Vec<IncludeOutcomeEvent>,
        validation: Vec<ValidationOutcomeEvent>,
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
    }

    #[test]
    fn validate_with_observer_emits_failed_validation_outcome() {
        let root = temp_root("validate_observer_failure");
        let template_path = PathBuf::from("nested").join("template.md.j2");
        write_file(
            &root.join(&template_path),
            "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
        );
        let mut observer = CapturingObserver::default();

        let report = validate_with_observer(
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

        assert!(!report.ok);
        assert_eq!(observer.attempts.len(), 1);
        assert_eq!(observer.attempts[0].template, "nested/template.md.j2");
        assert_eq!(observer.resolve.len(), 1);
        assert_eq!(observer.include.len(), 1);
        assert_eq!(observer.validation.len(), 1);
        assert_eq!(
            observer.validation[0].errors[0].code,
            DiagnosticCode::ErrValMissingRequired
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
