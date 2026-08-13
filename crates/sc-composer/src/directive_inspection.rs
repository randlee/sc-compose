//! Parser-backed inspection of template-loading directives.

use std::str;

use minijinja::Environment;

use crate::{ComposeError, ConfigError, DiagnosticCode, RecoveryHint, RecoveryHintKind};

/// The byte range occupied by one complete Jinja statement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    /// Inclusive UTF-8 byte offset of the statement's opening delimiter.
    pub byte_start: usize,
    /// Exclusive UTF-8 byte offset immediately after the closing delimiter.
    pub byte_end: usize,
}

/// The template-loading statement forms recognized by the inspection API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemplateDirectiveKind {
    /// A Jinja `include` statement.
    Include,
    /// A Jinja `import` statement.
    Import,
    /// A Jinja `from ... import ...` statement.
    FromImport,
}

/// One classified template-loading directive and its source location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemplateDirective {
    /// The directive form classified by the parser-backed inspection pass.
    pub directive: TemplateDirectiveKind,
    /// The exact UTF-8 byte span of the complete Jinja statement.
    pub source_span: SourceSpan,
}

/// Inspect UTF-8 template bytes for parser-valid include/import directives.
///
/// The returned spans are in source order and include the Jinja statement
/// delimiters. Nested statements are reported independently. Directive
/// target expressions are never captured or returned; callers that need
/// resolution must apply their own root and path policy.
///
/// The template is first compiled by `MiniJinja` so malformed syntax fails with
/// a stable configuration diagnostic. A narrow source scanner then projects
/// only the three public directive classifications without exposing
/// `MiniJinja` AST or token types across the crate boundary.
///
/// # Errors
///
/// Returns [`DiagnosticCode::ErrConfigRead`] for non-UTF-8 input or
/// [`DiagnosticCode::ErrConfigParse`] when the template cannot be parsed.
pub fn inspect_template_directives(
    raw_file_bytes: &[u8],
) -> Result<Vec<TemplateDirective>, ComposeError> {
    let source = str::from_utf8(raw_file_bytes).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigRead,
            "template source is not valid UTF-8",
        )
        .with_source(error)
        .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::InspectInput {
            description: "provide template source encoded as UTF-8".to_owned(),
        }))
    })?;

    Environment::new()
        .template_from_str(source)
        .map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                "failed to parse template directives",
            )
            .with_source(error)
            .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::InspectInput {
                description: "inspect the template for valid MiniJinja syntax".to_owned(),
            }))
        })?;

    Ok(scan_directives(source))
}

fn scan_directives(source: &str) -> Vec<TemplateDirective> {
    let mut directives = Vec::new();
    let mut cursor = 0;
    let mut raw_block = false;

    while let Some((start, delimiter)) = next_tag(source, cursor) {
        match delimiter {
            TagDelimiter::Comment => {
                cursor = source[start + 2..]
                    .find("#}")
                    .map_or(source.len(), |end| start + 2 + end + 2);
            }
            TagDelimiter::Variable => {
                cursor = source[start + 2..]
                    .find("}}")
                    .map_or(source.len(), |end| start + 2 + end + 2);
            }
            TagDelimiter::Block => {
                let Some((end, content)) = block_tag(source, start + 2) else {
                    break;
                };
                let keyword = first_keyword(content);

                if raw_block {
                    if keyword == Some("endraw") {
                        raw_block = false;
                    }
                } else if keyword == Some("raw") {
                    raw_block = true;
                } else if let Some(directive) = match keyword {
                    Some("include") => Some(TemplateDirectiveKind::Include),
                    Some("import") => Some(TemplateDirectiveKind::Import),
                    Some("from") => Some(TemplateDirectiveKind::FromImport),
                    _ => None,
                } {
                    directives.push(TemplateDirective {
                        directive,
                        source_span: SourceSpan {
                            byte_start: start,
                            byte_end: end,
                        },
                    });
                }
                cursor = end;
            }
        }
    }

    directives
}

#[derive(Clone, Copy)]
enum TagDelimiter {
    Comment,
    Variable,
    Block,
}

fn next_tag(source: &str, cursor: usize) -> Option<(usize, TagDelimiter)> {
    [
        (source[cursor..].find("{#"), TagDelimiter::Comment),
        (source[cursor..].find("{{"), TagDelimiter::Variable),
        (source[cursor..].find("{%"), TagDelimiter::Block),
    ]
    .into_iter()
    .filter_map(|(offset, delimiter)| offset.map(|offset| (cursor + offset, delimiter)))
    .min_by_key(|(offset, _)| *offset)
}

fn block_tag(source: &str, content_start: usize) -> Option<(usize, &str)> {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut escaped = false;

    for index in content_start..bytes.len().saturating_sub(1) {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
        } else if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
        } else if byte == b'%' && bytes[index + 1] == b'}' {
            return Some((index + 2, &source[content_start..index]));
        }
    }

    None
}

fn first_keyword(content: &str) -> Option<&str> {
    content
        .trim()
        .trim_start_matches(['-', '+'])
        .trim()
        .trim_end_matches(['-', '+'])
        .split_whitespace()
        .next()
}

#[cfg(test)]
mod tests {
    use super::{RecoveryHintKind, SourceSpan, TemplateDirectiveKind, inspect_template_directives};

    #[test]
    fn classifies_all_three_directive_kinds_with_exact_spans() {
        let source = br#"{% include "base.j2" %}
{% import "macros.j2" as macros %}
{% from "helpers.j2" import render %}
"#;
        let directives = inspect_template_directives(source).unwrap();

        assert_eq!(directives.len(), 3);
        assert_eq!(directives[0].directive, TemplateDirectiveKind::Include);
        assert_eq!(directives[1].directive, TemplateDirectiveKind::Import);
        assert_eq!(directives[2].directive, TemplateDirectiveKind::FromImport);
        assert_eq!(
            &source[directives[0].source_span.byte_start..directives[0].source_span.byte_end],
            b"{% include \"base.j2\" %}"
        );
        assert_eq!(
            directives[2].source_span,
            SourceSpan {
                byte_start: 59,
                byte_end: 96,
            }
        );
    }

    #[test]
    fn reports_nested_and_mixed_directives_but_ignores_comments_and_raw_text() {
        let source =
            br#"{% if enabled %}{% include "one.j2" %}{% from path import item %}{% endif %}
{# {% import "comment.j2" as ignored %} #}
{% raw %}{% include "raw.j2" %}{% endraw %}
"#;
        let directives = inspect_template_directives(source).unwrap();

        assert_eq!(
            directives
                .iter()
                .map(|directive| directive.directive)
                .collect::<Vec<_>>(),
            vec![
                TemplateDirectiveKind::Include,
                TemplateDirectiveKind::FromImport
            ]
        );
    }

    #[test]
    fn spans_are_utf8_byte_offsets_not_character_offsets() {
        let source = "préface\n{% include \"child.j2\" %}".as_bytes();
        let directives = inspect_template_directives(source).unwrap();
        let span = directives[0].source_span;

        assert_eq!(span.byte_start, "préface\n".len());
        assert_eq!(
            &source[span.byte_start..span.byte_end],
            b"{% include \"child.j2\" %}"
        );
    }

    #[test]
    fn returns_empty_for_a_valid_template_without_directives() {
        assert!(
            inspect_template_directives(b"plain text {{ value }}\n")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn malformed_directive_returns_a_stable_parse_diagnostic() {
        let error = inspect_template_directives(br#"{% include "unterminated %}"#).unwrap_err();

        assert_eq!(error.code(), Some(crate::DiagnosticCode::ErrConfigParse));
        let crate::ComposeError::Config(config_error) = &error else {
            panic!("directive parse errors must use ConfigError")
        };
        assert_eq!(
            config_error.recovery_hints().first().map(|hint| &hint.kind),
            Some(&RecoveryHintKind::InspectInput {
                description: "inspect the template for valid MiniJinja syntax".to_owned(),
            })
        );
        assert!(
            error
                .to_string()
                .contains("failed to parse template directives")
        );
    }

    #[test]
    fn invalid_utf8_returns_a_stable_read_diagnostic() {
        let error = inspect_template_directives(b"hello \xff").unwrap_err();

        assert_eq!(error.code(), Some(crate::DiagnosticCode::ErrConfigRead));
        let crate::ComposeError::Config(config_error) = &error else {
            panic!("invalid UTF-8 errors must use ConfigError")
        };
        assert_eq!(
            config_error.recovery_hints().first().map(|hint| &hint.kind),
            Some(&RecoveryHintKind::InspectInput {
                description: "provide template source encoded as UTF-8".to_owned(),
            })
        );
    }
}
