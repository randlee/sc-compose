//! Lexical folding of ordinary `{{ expr }}` expressions into the formula's
//! `{{{ expr }}}` composition delimiters.
//!
//! A formula renders in exactly one pass. When sc-compose owns ordinary Beads
//! expressions, each source `{{ expr }}` is rewritten to `{{{ expr }}}` before
//! that pass, so both forms share one context, one strict token scan and one
//! render. Rendered values are never parsed again, so no value can form or
//! hide template syntax.
//!
//! The scanner follows the renderer's lexer. A tag starts at the leftmost of
//! `{{{`, `{%` or `{#`; an ordinary `{{` is folded only when no such tag
//! starts inside it, so `{{%` and `{{#` are a literal `{` before a statement
//! or comment. Comments, statements, composition expressions and
//! `{% raw %}` blocks are copied unchanged. Expression and statement ends are
//! found outside string literals and open brackets.

/// Rewrite every source `{{ expr }}` outside comments, statements, raw blocks
/// and `{{{ ... }}}` expressions to `{{{ expr }}}`.
pub(crate) fn fold_bead_expressions(text: &str) -> String {
    let mut output = String::with_capacity(text.len() + text.len() / 16);
    let mut position = 0;
    while position < text.len() {
        let rest = &text[position..];
        let Some(start) = rest.find('{') else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        position += start;
        let rest = &text[position..];
        let consumed = if rest.starts_with("{{{") {
            copy_through(&mut output, rest, 3, "}}}")
        } else if rest.starts_with("{#") {
            copy_comment(&mut output, rest)
        } else if rest.starts_with("{%") {
            copy_statement(&mut output, rest)
        } else if rest.starts_with("{{") && !matches!(rest.as_bytes().get(2), Some(b'%' | b'#')) {
            fold_expression(&mut output, rest)
        } else {
            output.push('{');
            1
        };
        position += consumed;
    }
    output
}

/// Copy a tag that ends at `close`. An unterminated tag is copied to the end
/// of the text, where the renderer reports it as a syntax error.
fn copy_through(output: &mut String, rest: &str, open_len: usize, close: &str) -> usize {
    let end =
        find_outside_strings(rest, open_len, close).map_or(rest.len(), |end| end + close.len());
    output.push_str(&rest[..end]);
    end
}

fn copy_comment(output: &mut String, rest: &str) -> usize {
    let end = rest[2..].find("#}").map_or(rest.len(), |end| 2 + end + 2);
    output.push_str(&rest[..end]);
    end
}

/// Copy a statement, or a whole `{% raw %}` block recognized as the renderer
/// does; an unterminated raw block is copied to the end of the text.
fn copy_statement(output: &mut String, rest: &str) -> usize {
    let marker = 2 + usize::from(rest[2..].starts_with(['-', '+']));
    let Some(raw) = skip_basic_tag(&rest[marker..], "raw", false) else {
        return copy_through(output, rest, 2, "%}");
    };
    let end = find_endraw(rest, marker + raw).unwrap_or(rest.len());
    output.push_str(&rest[..end]);
    end
}

/// Fold `{{ expr }}` into `{{{ expr }}}`. An unterminated expression keeps
/// the folded opener, so the renderer reports it as a syntax error.
fn fold_expression(output: &mut String, rest: &str) -> usize {
    let Some(close) = find_outside_strings(rest, 2, "}}") else {
        output.push_str("{{{");
        output.push_str(&rest[2..]);
        return rest.len();
    };
    output.push_str("{{{");
    output.push_str(&rest[2..close]);
    output.push_str("}}}");
    close + 2
}

/// Match the renderer's basic tag `[ws-control] name [-|+] %}` at the start
/// of `body` (the text after `{%`), with ASCII whitespace around `name`.
/// Returns the length through `%}`.
fn skip_basic_tag(body: &str, name: &str, skip_ws_control: bool) -> Option<usize> {
    let mut rest = body;
    if skip_ws_control && let Some(tail) = rest.strip_prefix(['-', '+']) {
        rest = tail;
    }
    rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
    rest = rest.strip_prefix(name)?;
    rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
    rest = rest.strip_prefix(['-', '+']).unwrap_or(rest);
    rest = rest.strip_prefix("%}")?;
    Some(body.len() - rest.len())
}

/// Return the end offset of the `{% endraw %}` tag that closes a raw block
/// whose body starts at `from`.
fn find_endraw(text: &str, from: usize) -> Option<usize> {
    let mut search = from;
    while let Some(offset) = text[search..].find("{%") {
        let body_start = search + offset + 2;
        if let Some(len) = skip_basic_tag(&text[body_start..], "endraw", true) {
            return Some(body_start + len);
        }
        search = body_start;
    }
    None
}

/// Find `close` after `from` as the renderer's lexer does: outside single- and
/// double-quoted strings (with backslash escapes) and only while no `(`, `[`
/// or `{` is open. Returns the offset of `close` in `text`.
fn find_outside_strings(text: &str, from: usize, close: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = from;
    let mut quote = None;
    let mut depth = 0_usize;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(_) if byte == b'\\' => {
                index += 2;
                continue;
            }
            Some(open) if byte == open => quote = None,
            None if byte == b'\'' || byte == b'"' => quote = Some(byte),
            None if depth == 0 && bytes[index..].starts_with(close.as_bytes()) => {
                return Some(index);
            }
            None if matches!(byte, b'(' | b'[' | b'{') => depth += 1,
            None if matches!(byte, b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            Some(_) | None => {}
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::fold_bead_expressions;

    #[test]
    fn folds_ordinary_expressions_only_outside_other_tags() {
        let cases = [
            ("a {{ x }} b", "a {{{ x }}} b"),
            ("{{x|upper}}{{- y -}}", "{{{x|upper}}}{{{- y -}}}"),
            ("{{{ x }}} {{ y }}", "{{{ x }}} {{{ y }}}"),
            ("{{{ '}}' }}}", "{{{ '}}' }}}"),
            ("{{ '}}' ~ x }}", "{{{ '}}' ~ x }}}"),
            ("{# {{ x }} #}{{ y }}", "{# {{ x }} #}{{{ y }}}"),
            (
                "{% if '{{' %}{{ x }}{% endif %}",
                "{% if '{{' %}{{{ x }}}{% endif %}",
            ),
            (
                "{% raw %}{{ x }}{% endraw %}{{ y }}",
                "{% raw %}{{ x }}{% endraw %}{{{ y }}}",
            ),
            (
                "{%- raw -%}{{ x }}{%- endraw +%}",
                "{%- raw -%}{{ x }}{%- endraw +%}",
            ),
            ("{{{{ x }}}}", "{{{{ x }}}}"),
            ("{ {x} }", "{ {x} }"),
            ("{{ unterminated", "{{{ unterminated"),
            ("{{% if 1 %}{% endif %}}", "{{% if 1 %}{% endif %}}"),
            ("{{# c #}{{ x }}}", "{{# c #}{{{ x }}}}"),
            (
                "{% raw %}{%-- endraw %}{{ x }}{% endraw %}",
                "{% raw %}{%-- endraw %}{{ x }}{% endraw %}",
            ),
            (
                "{% raw %}{% endraw\u{a0}%}{{ x }}{% endraw %}",
                "{% raw %}{% endraw\u{a0}%}{{ x }}{% endraw %}",
            ),
            ("{% rawx %}{{ x }}", "{% rawx %}{{{ x }}}"),
            ("ü{{ x }}é", "ü{{{ x }}}é"),
            ("{{ {\"a\": {\"b\": 1}} }}", "{{{ {\"a\": {\"b\": 1}} }}}"),
            ("{{ (x) }}{{ [x][0] }}", "{{{ (x) }}}{{{ [x][0] }}}"),
            (
                "{% if {'a': 1}%}x{% endif %}",
                "{% if {'a': 1}%}x{% endif %}",
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(fold_bead_expressions(source), expected, "{source}");
        }
    }
}
