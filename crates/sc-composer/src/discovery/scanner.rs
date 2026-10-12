#[derive(Clone, Copy)]
pub(super) enum Delimiter {
    Expression,
    Statement,
}

pub(super) struct ExpressionDelimiters {
    pub(super) open: String,
    pub(super) close: String,
}

impl ExpressionDelimiters {
    pub(super) fn with_delimiters(open_delimiter: &str, close_delimiter: &str) -> Self {
        Self {
            open: open_delimiter.to_owned(),
            close: close_delimiter.to_owned(),
        }
    }
}

pub(super) fn walk_template<F>(
    text: &str,
    expression_delimiters: &ExpressionDelimiters,
    mut visit: F,
) where
    F: FnMut(Delimiter, &str) -> bool,
{
    let mut cursor = text;
    while let Some((delimiter, start)) = next_delimiter(cursor, expression_delimiters) {
        // A comment renders nothing, so nothing inside it is a reference.
        if let Some(comment) = cursor.find("{#").filter(|comment| *comment < start) {
            let Some(end) = cursor[comment + 2..].find("#}") else {
                break;
            };
            cursor = &cursor[comment + 2 + end + 2..];
            continue;
        }
        let start_delimiter = match delimiter {
            Delimiter::Expression => expression_delimiters.open.as_str(),
            Delimiter::Statement => "{%",
        };
        let end_delimiter = match delimiter {
            Delimiter::Expression => expression_delimiters.close.as_str(),
            Delimiter::Statement => "%}",
        };
        let after_start = &cursor[start + start_delimiter.len()..];
        // A closing triple-brace delimiter may be immediately followed by
        // a literal `}`; consume the first delimiter in that run.
        let end = find_closing_delimiter(after_start, end_delimiter);
        let Some(end) = end else { break };
        let raw_content = &after_start[..end];
        let without_leading_marker = raw_content
            .strip_prefix('-')
            .or_else(|| raw_content.strip_prefix('+'))
            .unwrap_or(raw_content);
        let without_markers = without_leading_marker
            .strip_suffix('-')
            .or_else(|| without_leading_marker.strip_suffix('+'))
            .unwrap_or(without_leading_marker);
        let expression = without_markers.trim();
        cursor = &after_start[end + end_delimiter.len()..];
        // A raw block renders its body literally, so it holds no references.
        if matches!(delimiter, Delimiter::Statement) && expression == "raw" {
            let Some(body_end) = find_endraw(cursor) else {
                break;
            };
            cursor = &cursor[body_end..];
            continue;
        }
        if visit(delimiter, expression) {
            break;
        }
    }
}

/// Find a tag's closing delimiter while respecting quoted string literals.
///
/// A delimiter-looking sequence inside a Jinja string is ordinary string
/// content.  This intentionally only performs lexical quote handling; the
/// renderer remains responsible for reporting malformed expressions.
fn find_closing_delimiter(text: &str, delimiter: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let delimiter = delimiter.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut cursor = 0;

    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(quote_byte) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == quote_byte {
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
        if bytes[cursor..].starts_with(delimiter) {
            return Some(cursor);
        }
        cursor += 1;
    }

    None
}

fn next_delimiter(
    text: &str,
    expression_delimiters: &ExpressionDelimiters,
) -> Option<(Delimiter, usize)> {
    match (
        find_exact_delimiter(text, expression_delimiters.open.as_str()),
        text.find("{%"),
    ) {
        (Some(expression), Some(statement)) if expression <= statement => {
            Some((Delimiter::Expression, expression))
        }
        (Some(_) | None, Some(statement)) => Some((Delimiter::Statement, statement)),
        (Some(expression), None) => Some((Delimiter::Expression, expression)),
        (None, None) => None,
    }
}

fn find_exact_delimiter(text: &str, delimiter: &str) -> Option<usize> {
    let repeated_byte = delimiter.as_bytes().first().copied()?;
    let mut cursor = 0usize;
    while cursor < text.len() {
        let found = text[cursor..].find(delimiter)?;
        let absolute = cursor + found;
        let after = absolute + delimiter.len();
        if text
            .as_bytes()
            .get(after)
            .is_some_and(|byte| *byte == repeated_byte)
        {
            // With normal double-brace syntax, `{{{"key": value}` starts a
            // variable expression followed by a dictionary literal.  Keep
            // the leftmost opening delimiter when that third brace clearly
            // starts a quoted dictionary key; otherwise a longer brace run
            // belongs to a higher-brace pass.
            if delimiter == "{{"
                && matches!(
                    text[after + 1..].trim_start().chars().next(),
                    Some('\'' | '\"')
                )
            {
                return Some(absolute);
            }
            cursor = after;
            continue;
        }
        return Some(absolute);
    }
    None
}

/// Return the offset just past the `{% endraw %}` tag (with optional
/// whitespace-control markers) that closes a raw block.
fn find_endraw(text: &str) -> Option<usize> {
    let mut search = 0;
    while let Some(open) = text[search..].find("{%") {
        let tag_start = search + open + 2;
        let close = tag_start + text[tag_start..].find("%}")?;
        let content = text[tag_start..close].trim_matches(['-', '+']).trim();
        if content == "endraw" {
            return Some(close + 2);
        }
        search = tag_start;
    }
    None
}
