//! Private JSON extraction input-limit validation.

use super::{ExtractError, MAX_JSON_INPUT_BYTES, MAX_JSON_NESTING_DEPTH, input_limit_error};

pub(super) fn validate_input_size(source: &str, label: &str) -> Result<(), ExtractError> {
    super::super::input_limits::validate_input_size(
        source,
        label,
        "JSON",
        MAX_JSON_INPUT_BYTES,
        input_limit_error,
    )
}

pub(super) fn validate_parse_depth(source: &str) -> Result<(), ExtractError> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    for byte in source.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_JSON_NESTING_DEPTH {
                    return Err(input_limit_error(format!(
                        "JSON nesting depth exceeds the maximum of {MAX_JSON_NESTING_DEPTH}"
                    )));
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn validate_value_limits(
    value: &serde_json::Value,
    depth: usize,
) -> Result<(), ExtractError> {
    if depth > MAX_JSON_NESTING_DEPTH {
        return Err(input_limit_error(format!(
            "JSON nesting depth exceeds the maximum of {MAX_JSON_NESTING_DEPTH}"
        )));
    }
    match value {
        serde_json::Value::Object(values) => {
            for value in values.values() {
                validate_value_limits(value, depth + 1)?;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_value_limits(value, depth + 1)?;
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
    Ok(())
}
