//! Filesystem-free inspection of MiniJinja dependency statements.
//!
//! This module recognizes dependency-bearing MiniJinja statements without
//! parsing, loading, or rendering their targets. Callers that deliberately do
//! not support template loaders can use it to reject such templates before
//! invoking a renderer.

use std::ops::Range;

/// Classifies a `MiniJinja` statement that requires template loading.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemplateDirectiveKind {
    /// A `{% include ... %}` statement.
    Include,
    /// An `{% import ... %}` statement.
    Import,
    /// A `{% from ... import ... %}` statement.
    FromImport,
}

/// Locates a dependency-bearing `MiniJinja` statement in a template source.
///
/// The span is a UTF-8 byte range over the exact `{% ... %}` statement. The
/// inspector deliberately does not evaluate the statement or resolve its
/// target, so a caller can fail before any loader or resolver is touched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateDirective {
    kind: TemplateDirectiveKind,
    span: Range<usize>,
}

impl TemplateDirective {
    /// Return the classified statement kind.
    #[must_use]
    pub const fn kind(&self) -> TemplateDirectiveKind {
        self.kind
    }

    /// Return the UTF-8 byte span of the whole statement.
    #[must_use]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }
}

/// Inspect a template for statements that require loading another template.
///
/// Recognizes `MiniJinja` `{% include %}`, `{% import %}`, and
/// `{% from ... import %}` statements, including whitespace-control markers.
/// Statements inside `MiniJinja` comments, expressions, and raw blocks are not
/// reported. This function performs no I/O and never evaluates templates.
#[must_use]
pub fn inspect_template_directives(source: &str) -> Vec<TemplateDirective> {
    let bytes = source.as_bytes();
    let mut directives = Vec::new();
    let mut cursor = 0;
    let mut in_raw_block = false;

    while cursor < bytes.len() {
        let Some(relative_start) = source[cursor..].find('{') else {
            break;
        };
        let start = cursor + relative_start;

        let Some(marker) = bytes.get(start + 1) else {
            break;
        };
        match *marker {
            b'#' => {
                cursor = skip_block(source, start + 2, "#}").unwrap_or(bytes.len());
            }
            b'{' => {
                cursor = skip_delimited(source, start + 2, "}}").unwrap_or(bytes.len());
            }
            b'%' => {
                let Some(end) = skip_delimited(source, start + 2, "%}") else {
                    break;
                };
                let content_end = end - 2;
                let statement = &source[start + 2..content_end];
                let keyword = statement_keyword(statement);

                if in_raw_block {
                    in_raw_block = keyword.is_some_and(|value| value != "endraw");
                } else if keyword == Some("raw") {
                    in_raw_block = true;
                } else if let Some(kind) = directive_kind(statement, keyword) {
                    directives.push(TemplateDirective {
                        kind,
                        span: start..end,
                    });
                }

                cursor = end;
            }
            _ => cursor = start + 1,
        }
    }

    directives
}

fn directive_kind(statement: &str, keyword: Option<&str>) -> Option<TemplateDirectiveKind> {
    match keyword {
        Some("include") => Some(TemplateDirectiveKind::Include),
        Some("import") => Some(TemplateDirectiveKind::Import),
        Some("from") if has_import_keyword(statement) => Some(TemplateDirectiveKind::FromImport),
        _ => None,
    }
}

fn has_import_keyword(statement: &str) -> bool {
    statement
        .trim()
        .trim_start_matches('-')
        .split_whitespace()
        .skip(1)
        .any(|token| token.trim_matches('-') == "import")
}

fn statement_keyword(statement: &str) -> Option<&str> {
    statement
        .trim()
        .trim_start_matches('-')
        .trim_start()
        .split(|character: char| character.is_whitespace() || character == '-')
        .next()
        .filter(|keyword| !keyword.is_empty())
}

fn skip_block(source: &str, content_start: usize, closing: &str) -> Option<usize> {
    source[content_start..]
        .find(closing)
        .map(|offset| content_start + offset + closing.len())
}

fn skip_delimited(source: &str, content_start: usize, closing: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let closing = closing.as_bytes();
    let mut cursor = content_start;
    let mut quote = None;
    let mut escaped = false;

    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            cursor += 1;
            continue;
        }

        if matches!(byte, b'\'' | b'\"') {
            quote = Some(byte);
            cursor += 1;
            continue;
        }
        if bytes[cursor..].starts_with(closing) {
            return Some(cursor + closing.len());
        }
        cursor += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{TemplateDirectiveKind, inspect_template_directives};

    #[test]
    fn classifies_loading_statements_and_reports_exact_spans() {
        let source = "{% include \"card.j2\" %}\n{% import \"macros.j2\" as macros %}\n{% from \"shared.j2\" import button %}";
        let directives = inspect_template_directives(source);

        assert_eq!(directives.len(), 3);
        assert_eq!(directives[0].kind(), TemplateDirectiveKind::Include);
        assert_eq!(directives[1].kind(), TemplateDirectiveKind::Import);
        assert_eq!(directives[2].kind(), TemplateDirectiveKind::FromImport);
        assert_eq!(&source[directives[0].span()], "{% include \"card.j2\" %}");
        assert_eq!(
            &source[directives[1].span()],
            "{% import \"macros.j2\" as macros %}"
        );
        assert_eq!(
            &source[directives[2].span()],
            "{% from \"shared.j2\" import button %}"
        );
    }

    #[test]
    fn recognizes_whitespace_control_and_preserves_utf8_byte_spans() {
        let source = "é {% - include \"card.j2\" - %}";
        let directives = inspect_template_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind(), TemplateDirectiveKind::Include);
        assert_eq!(
            &source[directives[0].span()],
            "{% - include \"card.j2\" - %}"
        );
    }

    #[test]
    fn ignores_directive_text_inside_comments_expressions_and_raw_blocks() {
        let source = concat!(
            "{# {% include \"comment.j2\" %} #}",
            "{{ \"{% import 'expression.j2' %}\" }}",
            "{% raw %}{% from \"raw.j2\" import ignored %}{% endraw %}",
            "{% include \"actual.j2\" %}"
        );
        let directives = inspect_template_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind(), TemplateDirectiveKind::Include);
        assert_eq!(&source[directives[0].span()], "{% include \"actual.j2\" %}");
    }

    #[test]
    fn does_not_classify_prefixes_or_keywords_inside_quoted_arguments() {
        let source = concat!(
            "{% includex \"not-a-directive.j2\" %}",
            "{% from helper %}",
            "{% set source = \"%} include\" %}",
            "{% include \"a%}b.j2\" %}"
        );
        let directives = inspect_template_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind(), TemplateDirectiveKind::Include);
        assert_eq!(&source[directives[0].span()], "{% include \"a%}b.j2\" %}");
    }
}
