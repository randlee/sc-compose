//! Public validation entrypoint.

use crate::observer::{
    CompositionObserver, IncludeOutcomeEvent, NoopObserver, ResolveOutcomeEvent,
    ValidationOutcomeEvent,
};
use crate::{ComposeError, ComposeRequest, ValidationReport};

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
    let resolve_result = crate::resolve_template_path(request).inspect_err(|error| {
        if let ComposeError::Resolve(resolve_error) = &error {
            observer.on_resolve_outcome(&ResolveOutcomeEvent {
                resolved_path: None,
                attempted_paths: resolve_error.attempted_paths().to_vec(),
                code: resolve_error.code(),
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
                code: include_error.code(),
            });
        }
    })?;
    observer.on_include_outcome(&IncludeOutcomeEvent {
        resolved_files: expanded.resolved_files.clone(),
        include_chain: Vec::new(),
        code: None,
    });

    let report = crate::validation::validate_expanded(request, &expanded, resolve_result);
    observer.on_validation_outcome(&ValidationOutcomeEvent {
        warnings: report.warnings.clone(),
        errors: report.errors.clone(),
    });

    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::observer::{
        CompositionObserver, IncludeOutcomeEvent, ResolveOutcomeEvent, ValidationOutcomeEvent,
    };
    use crate::types::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};
    use crate::{DiagnosticCode, validate_with_observer};

    #[derive(Default)]
    struct CapturingObserver {
        resolve: Vec<ResolveOutcomeEvent>,
        include: Vec<IncludeOutcomeEvent>,
        validation: Vec<ValidationOutcomeEvent>,
    }

    impl CompositionObserver for CapturingObserver {
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
        write_file(
            &root.join("template.md.j2"),
            "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
        );
        let mut observer = CapturingObserver::default();

        let report = validate_with_observer(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            &mut observer,
        )
        .unwrap();

        assert!(!report.ok);
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
