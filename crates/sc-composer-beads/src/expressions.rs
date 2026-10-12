//! Lexical folding of ordinary `{{ expr }}` expressions into the formula's
//! `{{{ expr }}}` composition delimiters.
//!
//! A formula renders in exactly one pass. When sc-compose owns ordinary Beads
//! expressions, each source `{{ expr }}` is rewritten to `{{{ expr }}}` before
//! that pass, so both forms share one context, one strict token scan and one
//! render. Rendered values are never parsed again, so no value can form or
//! hide template syntax.
//!
//! The scanner follows the renderer's lexer: at each position the earliest of
//! `{{{`, `{%`, `{#` or `{{` starts a tag, `{{{` winning over `{{`. Comments,
//! statements, composition expressions and `{% raw %}` blocks are copied
//! unchanged; expression and statement ends are found outside string
//! literals.

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
        } else if rest.starts_with("{{") {
            fold_expression(&mut output, rest)
        } else {
            output.push('{');
            1
        };
        position += consumed;
    }
    output
}

/// Copy a tag that ends at `close`, found outside string literals; an
/// unterminated tag is copied whole so the renderer reports it.
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

fn copy_statement(output: &mut String, rest: &str) -> usize {
    let Some(close) = find_outside_strings(rest, 2, "%}") else {
        output.push_str(rest);
        return rest.len();
    };
    let mut end = close + 2;
    if tag_name(&rest[2..close]) == "raw" {
        end = find_endraw(rest, end).unwrap_or(rest.len());
    }
    output.push_str(&rest[..end]);
    end
}

fn fold_expression(output: &mut String, rest: &str) -> usize {
    let Some(close) = find_outside_strings(rest, 2, "}}") else {
        output.push_str(rest);
        return rest.len();
    };
    output.push_str("{{{");
    output.push_str(&rest[2..close]);
    output.push_str("}}}");
    close + 2
}

/// The tag name of a statement body, ignoring whitespace-control markers.
fn tag_name(body: &str) -> &str {
    body.trim_start_matches(['-', '+'])
        .trim_end_matches(['-', '+'])
        .trim()
}

/// Return the end offset of the `{% endraw %}` tag that closes a raw block
/// whose body starts at `from`.
fn find_endraw(text: &str, from: usize) -> Option<usize> {
    let mut search = from;
    while let Some(offset) = text[search..].find("{%") {
        let start = search + offset;
        let body_start = start + 2;
        if let Some(close) = text[body_start..].find("%}")
            && tag_name(&text[body_start..body_start + close]) == "endraw"
        {
            return Some(body_start + close + 2);
        }
        search = body_start;
    }
    None
}

/// Find `close` after `from`, skipping single- and double-quoted strings with
/// backslash escapes. Returns the offset of `close` in `text`.
fn find_outside_strings(text: &str, from: usize, close: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = from;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(_) if byte == b'\\' => {
                index += 2;
                continue;
            }
            Some(open) if byte == open => quote = None,
            None if byte == b'\'' || byte == b'"' => quote = Some(byte),
            None if bytes[index..].starts_with(close.as_bytes()) => return Some(index),
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
            ("{{ unterminated", "{{ unterminated"),
            ("ü{{ x }}é", "ü{{{ x }}}é"),
        ];
        for (source, expected) in cases {
            assert_eq!(fold_bead_expressions(source), expected, "{source}");
        }
    }
}
