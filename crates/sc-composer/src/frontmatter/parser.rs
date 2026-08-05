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
        let raw = serde_yaml::from_str::<RawFrontmatter>(frontmatter_text).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                "failed to parse YAML frontmatter",
            )
            .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "frontmatter".to_owned(),
            }))
            .with_source(error)
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
        if !headers.is_empty() && contains_jinja_syntax(&input[content_start..content_end]) {
            break;
        }

        headers.push(&input[content_start..content_end]);
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

fn opening_delimiter_len(input: &str, cursor: usize) -> Option<usize> {
    let remainder = input.get(cursor..)?;
    if remainder.starts_with("---\r\n") {
        Some(5)
    } else if remainder.starts_with("---\n") {
        Some(4)
    } else if remainder == "---" {
        Some(3)
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
