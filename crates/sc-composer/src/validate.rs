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
