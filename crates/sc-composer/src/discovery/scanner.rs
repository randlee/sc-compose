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
        let start_delimiter = match delimiter {
            Delimiter::Expression => expression_delimiters.open.as_str(),
            Delimiter::Statement => "{%",
        };
        let end_delimiter = match delimiter {
            Delimiter::Expression => expression_delimiters.close.as_str(),
            Delimiter::Statement => "%}",
        };
        let after_start = &cursor[start + start_delimiter.len()..];
        let end = match delimiter {
            Delimiter::Expression => find_exact_delimiter(after_start, end_delimiter),
            Delimiter::Statement => after_start.find(end_delimiter),
        };
        let Some(end) = end else { break };
        let raw_content = &after_start[..end];
        let without_leading_marker = raw_content.strip_prefix('-').unwrap_or(raw_content);
        let without_markers = without_leading_marker
            .strip_suffix('-')
            .or_else(|| without_leading_marker.strip_suffix('+'))
            .unwrap_or(without_leading_marker);
        let expression = without_markers.trim();
        if visit(delimiter, expression) {
            break;
        }
        cursor = &after_start[end + end_delimiter.len()..];
    }
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
            cursor = after;
            continue;
        }
        return Some(absolute);
    }
    None
}
