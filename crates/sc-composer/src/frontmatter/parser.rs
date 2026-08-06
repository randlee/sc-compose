//! Frontmatter delimiter scanning and stacked-header parsing.

use crate::diagnostics::DiagnosticCode;
use crate::error::{ComposeError, ConfigError, RecoveryHint, RecoveryHintKind};

use super::model::{ParsedTemplate, RawFrontmatter};

/// Parse a full template document and normalize its frontmatter if present.
///
/// # Errors
///
/// Returns [`ComposeError`] when the frontmatter block is malformed, missing a
/// terminating delimiter, or contains values outside the supported schema.
pub fn parse_template_document(input: &str) -> Result<ParsedTemplate, ComposeError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let Some((frontmatter_texts, body)) = split_frontmatter(input)? else {
        return Ok(ParsedTemplate {
            passes: Vec::new(),
            body: input.to_owned(),
        });
    };

    let mut passes = Vec::with_capacity(frontmatter_texts.len());
    for frontmatter_text in frontmatter_texts {
        let raw = serde_yaml::from_str::<RawFrontmatter>(frontmatter_text).map_err(|_error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                "failed to parse YAML frontmatter",
            )
            .with_recovery_hint(RecoveryHint::new(
                RecoveryHintKind::ReviewConfiguration {
                    key: "frontmatter".to_owned(),
                },
            ))
        })?;
        passes.push(super::normalizer::normalize_frontmatter(raw)?);
    }

    super::normalizer::validate_explicit_pass_numbers(&passes)?;

    Ok(ParsedTemplate {
        passes,
        body: body.to_owned(),
    })
}

fn split_frontmatter(input: &str) -> Result<Option<(Vec<&str>, &str)>, ComposeError> {
    let mut cursor = 0usize;
    let mut headers = Vec::new();

    while let Some(open_len) = opening_delimiter_len(input, cursor) {
        let content_start = cursor + open_len;
        let mut line_cursor = content_start;
        let mut closing = None;

        while line_cursor < input.len() {
            let line_end = next_line_end(input, line_cursor);
            let line = &input[line_cursor..line_end];
            let trimmed = line.trim_end_matches(['\n', '\r']);
            if matches!(trimmed, "---" | "...") {
                closing = Some((line_cursor, line_end));
                break;
            }
            line_cursor = line_end;
        }

        let Some((content_end, after_close)) = closing else {
            return Err(ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                "frontmatter block started with `---` but no closing delimiter was found",
            )
            .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "frontmatter".to_owned(),
            }))
            .into());
        };

        // A rendered document can legitimately begin with its own `---` block
        // immediately after the sc-compose config block. Once that candidate
        // contains Jinja syntax, it is template body rather than another YAML
        // config pass. Preserve the existing stacked-header behavior for
        // candidates that contain plain YAML (including empty headers).
        let candidate = &input[content_start..content_end];
        if !headers.is_empty()
            && (contains_jinja_syntax(candidate) || !is_recognized_frontmatter(candidate))
        {
            break;
        }

        headers.push(candidate);
        cursor = after_close;
    }

    if headers.is_empty() {
        Ok(None)
    } else {
        Ok(Some((headers, &input[cursor..])))
    }
}

fn contains_jinja_syntax(content: &str) -> bool {
    content.contains("{{") || content.contains("{%") || content.contains("{#")
}

fn is_recognized_frontmatter(content: &str) -> bool {
    if content.trim().is_empty() {
        return true;
    }

    let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(content) else {
        return false;
    };
    let serde_yaml::Value::Mapping(mapping) = value else {
        return false;
    };

    mapping.keys().all(|key| {
        let serde_yaml::Value::String(key) = key else {
            return false;
        };
        matches!(
            key.as_str(),
            "pass"
                | "required_variables"
                | "variables"
                | "defaults"
                | "input_defaults"
                | "metadata"
        )
    })
}

fn opening_delimiter_len(input: &str, cursor: usize) -> Option<usize> {
    let remainder = input.get(cursor..)?;
    let suffix = remainder.strip_prefix("---")?;
    let horizontal_whitespace_len = suffix
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let after_horizontal_whitespace = &suffix[horizontal_whitespace_len..];

    if after_horizontal_whitespace.starts_with("\r\n") {
        Some(3 + horizontal_whitespace_len + 2)
    } else if after_horizontal_whitespace.starts_with('\n') {
        Some(3 + horizontal_whitespace_len + 1)
    } else if after_horizontal_whitespace.is_empty() {
        Some(3 + horizontal_whitespace_len)
    } else {
        None
    }
}

fn next_line_end(input: &str, cursor: usize) -> usize {
    match input[cursor..].find('\n') {
        Some(offset) => cursor + offset + 1,
        None => input.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::{opening_delimiter_len, parse_template_document};

    #[test]
    fn opening_delimiter_without_trailing_whitespace_is_unchanged() {
        assert_eq!(opening_delimiter_len("---\nbody", 0), Some(4));
        assert_eq!(opening_delimiter_len("---\r\nbody", 0), Some(5));
        assert_eq!(opening_delimiter_len("---", 0), Some(3));
    }

    #[test]
    fn opening_delimiter_accepts_trailing_spaces() {
        assert_eq!(opening_delimiter_len("---   \nbody", 0), Some(7));
        assert_eq!(opening_delimiter_len("---   \r\nbody", 0), Some(8));
        assert_eq!(opening_delimiter_len("---   ", 0), Some(6));

        let parsed =
            parse_template_document("---   \nrequired_variables:\n  - name\n---\nbody").unwrap();
        assert_eq!(parsed.passes().len(), 1);
    }

    #[test]
    fn opening_delimiter_accepts_trailing_tabs() {
        assert_eq!(opening_delimiter_len("---\t\nbody", 0), Some(5));
        assert_eq!(opening_delimiter_len("---\t\r\nbody", 0), Some(6));

        let parsed = parse_template_document("---\t\nmetadata: {}\n---\nbody").unwrap();
        assert_eq!(parsed.passes().len(), 1);
    }

    #[test]
    fn closing_delimiter_with_trailing_whitespace_still_fails() {
        let error = parse_template_document("---\nmetadata: {}\n--- \nbody").unwrap_err();

        assert!(error.to_string().contains("no closing delimiter was found"));
    }
}
