use super::VarFileDecodeError;
use super::decode::{DecodedVarEntry, DecodedVarKey, DecodedVarObject, DecodedVarValue};

pub(super) fn find_yaml_merge_key(contents: &str) -> Option<(usize, usize)> {
    let mut block_scalar_indent = None;

    for (line_index, line) in contents.lines().enumerate() {
        let indentation = line.bytes().take_while(|byte| *byte == b' ').count();
        let has_content = line[indentation..]
            .chars()
            .next()
            .is_some_and(|character| character != '#');

        if let Some(base_indent) = block_scalar_indent {
            if has_content && indentation > base_indent {
                continue;
            }
            block_scalar_indent = None;
        }

        let scan = scan_yaml_line(line);
        if let Some(byte_index) = scan.merge_key {
            let column = line[..byte_index].chars().count() + 1;
            return Some((line_index + 1, column));
        }

        if scan.block_scalar {
            block_scalar_indent = Some(indentation);
        }
    }

    None
}

pub(super) struct YamlLineScan {
    pub(super) merge_key: Option<usize>,
    pub(super) block_scalar: bool,
}

pub(super) fn scan_yaml_line(line: &str) -> YamlLineScan {
    let outside_quote = unquoted_uncommented(line);
    let merge_key = outside_quote.iter().find_map(|(byte_index, _)| {
        if !line[*byte_index..].starts_with("<<") {
            return None;
        }
        let suffix = &line[*byte_index + 2..];
        if suffix
            .chars()
            .find(|character| !character.is_ascii_whitespace())
            .is_none_or(|character| character != ':')
        {
            return None;
        }

        let prefix = line[..*byte_index].trim_end();
        (prefix.is_empty()
            || prefix == "-"
            || prefix.ends_with('{')
            || prefix.ends_with(',')
            || prefix.ends_with('?'))
        .then_some(*byte_index)
    });
    let block_scalar = outside_quote
        .iter()
        .map(|(_, character)| *character)
        .collect::<String>()
        .split_whitespace()
        .any(|token| {
            matches!(
                token.trim_end_matches(','),
                "|" | ">" | "|-" | "|+" | ">-" | ">+"
            )
        });

    YamlLineScan {
        merge_key,
        block_scalar,
    }
}

pub(super) fn unquoted_uncommented(line: &str) -> Vec<(usize, char)> {
    let mut outside_quote = Vec::new();
    let mut quote = None;
    let mut escaped = false;

    for (byte_index, character) in line.char_indices() {
        match quote {
            Some('"') => {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    quote = None;
                }
            }
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                }
            }
            Some(_) => unreachable!("only YAML single and double quotes are tracked"),
            None if character == '#' => break,
            None if character == '"' || character == '\'' => quote = Some(character),
            None => outside_quote.push((byte_index, character)),
        }
    }

    outside_quote
}

pub(super) fn decode_yaml_object(
    value: serde_yaml::Value,
) -> Result<DecodedVarObject, VarFileDecodeError> {
    let serde_yaml::Value::Mapping(object) = value else {
        return Err(VarFileDecodeError::NotAnObject);
    };
    Ok(DecodedVarObject {
        entries: object
            .into_iter()
            .map(|(key, value)| DecodedVarEntry {
                key: DecodedVarKey::Yaml(key),
                value: DecodedVarValue::Yaml(value),
            })
            .collect(),
    })
}
