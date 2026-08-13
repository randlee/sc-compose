//! Shared lexical scanning for Jinja variable expressions.

/// Find the next Jinja variable expression in `source`.
///
/// The returned tuple contains the byte offsets of the opening delimiter, the
/// expression body, and the closing delimiter, respectively. Jinja comments
/// and raw blocks are skipped. This intentionally remains a conservative
/// lexical scan; expression semantics belong to the renderer.
///
/// An unterminated comment, statement, raw block, or variable expression ends
/// the scan without returning a partial match.
#[must_use]
pub fn next_jinja_variable_expression(
    source: &str,
    mut search_start: usize,
) -> Option<(usize, usize, usize)> {
    while search_start < source.len() {
        let remainder = &source[search_start..];
        let variable = remainder.find("{{");
        let comment = remainder.find("{#");
        let statement = remainder.find("{% ").or_else(|| remainder.find("{%"));
        if variable.is_none() && comment.is_none() && statement.is_none() {
            return None;
        }

        let variable_offset = variable.map(|offset| search_start + offset);
        let comment_offset = comment.map(|offset| search_start + offset);
        let statement_offset = statement.map(|offset| search_start + offset);
        let next = [variable_offset, comment_offset, statement_offset]
            .into_iter()
            .flatten()
            .min()?;

        if Some(next) == comment_offset {
            let end = source[next + 2..].find("#}")?;
            search_start = next + 2 + end + 2;
            continue;
        }

        if Some(next) == statement_offset {
            let end = source[next + 2..].find("%}")?;
            let statement_text = source[next + 2..next + 2 + end].trim();
            if statement_text == "raw" {
                let raw_end = source[next + 2 + end + 2..].find("{% endraw %}")?;
                search_start = next + 2 + end + 2 + raw_end + "{% endraw %}".len();
            } else {
                search_start = next + 2 + end + 2;
            }
            continue;
        }

        let expression_start = next + 2;
        let close = expression_start + source[expression_start..].find("}}")?;
        return Some((next, expression_start, close));
    }
    None
}
