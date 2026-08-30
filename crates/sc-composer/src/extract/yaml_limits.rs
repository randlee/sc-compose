//! Private YAML extraction input-limit validation.

use super::{
    ExtractError, MAX_YAML_INPUT_BYTES, MAX_YAML_NESTING_DEPTH, YamlNode, input_limit_error,
};

pub(super) fn validate_input_size(source: &str, label: &str) -> Result<(), ExtractError> {
    super::super::input_limits::validate_input_size(
        source,
        label,
        "YAML",
        MAX_YAML_INPUT_BYTES,
        input_limit_error,
    )
}

pub(super) fn validate_parse_depth(source: &str) -> Result<(), ExtractError> {
    let mut block_indents = Vec::new();
    let mut flow_depth = 0;
    let mut quote = None;
    let mut escaped = false;

    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if block_indents.last().is_none_or(|last| indent > *last) {
            block_indents.push(indent);
        } else {
            while block_indents.last().is_some_and(|last| indent < *last) {
                block_indents.pop();
            }
            if block_indents.last().is_none_or(|last| indent > *last) {
                block_indents.push(indent);
            }
        }
        if block_indents.len().saturating_sub(1) > MAX_YAML_NESTING_DEPTH {
            return Err(input_limit_error(format!(
                "YAML nesting depth exceeds the maximum of {MAX_YAML_NESTING_DEPTH}"
            )));
        }

        for byte in trimmed.bytes() {
            if let Some(active_quote) = quote {
                if active_quote == b'"' && escaped {
                    escaped = false;
                } else if active_quote == b'"' && byte == b'\\' {
                    escaped = true;
                } else if byte == active_quote {
                    quote = None;
                }
                continue;
            }
            match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'[' | b'{' => {
                    flow_depth += 1;
                    if flow_depth > MAX_YAML_NESTING_DEPTH {
                        return Err(input_limit_error(format!(
                            "YAML nesting depth exceeds the maximum of {MAX_YAML_NESTING_DEPTH}"
                        )));
                    }
                }
                b']' | b'}' => flow_depth = flow_depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    Ok(())
}

pub(super) fn validate_value_limits(value: &YamlNode, depth: usize) -> Result<(), ExtractError> {
    if depth > MAX_YAML_NESTING_DEPTH {
        return Err(input_limit_error(format!(
            "YAML nesting depth exceeds the maximum of {MAX_YAML_NESTING_DEPTH}"
        )));
    }
    match value {
        YamlNode::Mapping(values) => {
            for (_, value) in values {
                validate_value_limits(value, depth + 1)?;
            }
        }
        YamlNode::Sequence(values) => {
            for value in values {
                validate_value_limits(value, depth + 1)?;
            }
        }
        YamlNode::String(_) | YamlNode::Other(_) => {}
    }
    Ok(())
}
