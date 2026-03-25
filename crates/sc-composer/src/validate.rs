//! Public validation entrypoint.

use crate::{ComposeError, ComposeRequest, ValidationReport};

/// Validate a compose request without rendering output.
///
/// # Errors
///
/// Returns [`ComposeError`] when resolution or include expansion fails.
pub fn validate(request: &ComposeRequest) -> Result<ValidationReport, ComposeError> {
    crate::validation::validate(request)
}
