//! Shared private input-size validation for structural extractors.

use super::ExtractError;

pub(super) fn validate_input_size(
    source: &str,
    label: &str,
    format: &str,
    maximum_bytes: usize,
    input_limit_error: impl FnOnce(String) -> ExtractError,
) -> Result<(), ExtractError> {
    if source.len() > maximum_bytes {
        return Err(input_limit_error(format!(
            "{format} {label} input is {} bytes; maximum is {maximum_bytes} bytes",
            source.len()
        )));
    }
    Ok(())
}
