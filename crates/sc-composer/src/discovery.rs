//! Statement-aware template variable discovery.

use std::collections::{BTreeMap, BTreeSet};

use crate::frontmatter::ParsedTemplate;
use crate::types::VariableName;

const LOOP_CONTEXT_NAMES: &[&str] = &[
    "loop",
    "loop.index",
    "loop.index0",
    "loop.revindex",
    "loop.revindex0",
    "loop.first",
    "loop.last",
    "loop.length",
    "loop.depth",
    "loop.depth0",
    "loop.cycle",
];

#[derive(Debug, Default)]
struct LoopScope {
    bound_names: BTreeSet<String>,
}

/// Discover declared template variable tokens without running full validation.
///
/// Returns the set of variable names referenced by `text` using the standard
/// double-brace `{{ name }}` expression delimiters.
#[must_use]
pub fn discover_tokens(text: &str) -> BTreeSet<VariableName> {
    discover_tokens_with_brace_count(text, 2)
}

/// Discover declared template variable tokens for a caller-provided brace count.
#[must_use]
pub fn discover_tokens_with_brace_count(text: &str, brace_count: usize) -> BTreeSet<VariableName> {
    if brace_count < 2 {
        return BTreeSet::new();
    }

    discover_tokens_with_delimiters(text, &"{".repeat(brace_count), &"}".repeat(brace_count))
}

/// Discover template variable tokens using caller-provided expression
/// delimiters.
#[must_use]
pub fn discover_tokens_with_delimiters(
    text: &str,
    open_delimiter: &str,
    close_delimiter: &str,
) -> BTreeSet<VariableName> {
    if open_delimiter.is_empty() || close_delimiter.is_empty() {
        return BTreeSet::new();
    }

    let mut tokens = BTreeSet::new();
    let mut scopes = vec![LoopScope::default()];
    let expression_delimiters =
        ExpressionDelimiters::with_delimiters(open_delimiter, close_delimiter);
    walk_template(text, &expression_delimiters, |delimiter, expression| {
        match delimiter {
            Delimiter::Expression => collect_identifiers(expression, &scopes, &mut tokens),
            Delimiter::Statement => {
                if let Some(loop_scope) = parse_for_loop_scope(expression, &scopes, &mut tokens) {
                    scopes.push(loop_scope.scope);
                } else if expression.starts_with("endfor") {
                    if scopes.len() > 1 {
                        scopes.pop();
                    }
                } else if let Some(name) = parse_set_scope(expression, &scopes, &mut tokens) {
                    scopes[0].bound_names.insert(name);
                } else {
                    collect_identifiers(expression, &scopes, &mut tokens);
                }
            }
        }
        false
    });
    tokens
}

/// Return whether `text` contains a bare-identifier loop over `variable`.
pub(crate) fn has_bare_for_loop_over(text: &str, variable: &str) -> bool {
    let mut tokens = BTreeSet::new();
    let mut scopes = vec![LoopScope::default()];
    let expression_delimiters = ExpressionDelimiters::with_delimiters("{{", "}}");
    let mut found = false;
    walk_template(text, &expression_delimiters, |delimiter, expression| {
        if matches!(delimiter, Delimiter::Statement) {
            if let Some(loop_scope) = parse_for_loop_scope(expression, &scopes, &mut tokens) {
                if loop_scope.iterable == variable && is_bare_identifier(&loop_scope.iterable) {
                    found = true;
                    return true;
                }
                scopes.push(loop_scope.scope);
            } else if expression.starts_with("endfor") {
                if scopes.len() > 1 {
                    scopes.pop();
                }
            } else if let Some(name) = parse_set_scope(expression, &scopes, &mut tokens) {
                scopes[0].bound_names.insert(name);
            } else {
                collect_identifiers(expression, &scopes, &mut tokens);
            }
        }
        false
    });
    found
}

/// Discover tokens for every parsed pass using that pass's brace count.
#[must_use]
pub fn discover_all_pass_tokens(
    parsed: &ParsedTemplate,
) -> BTreeMap<usize, BTreeSet<VariableName>> {
    parsed
        .passes()
        .iter()
        .map(|frontmatter| {
            let pass_number = usize::from(frontmatter.pass_number());
            let brace_count = pass_number + 1;
            (
                pass_number,
                discover_tokens_with_brace_count(parsed.body(), brace_count),
            )
        })
        .collect()
}

#[derive(Clone, Copy)]
enum Delimiter {
    Expression,
    Statement,
}

struct ExpressionDelimiters {
    open: String,
    close: String,
}

impl ExpressionDelimiters {
    fn with_delimiters(open_delimiter: &str, close_delimiter: &str) -> Self {
        Self {
            open: open_delimiter.to_owned(),
            close: close_delimiter.to_owned(),
        }
    }
}

fn next_delimiter(
    text: &str,
    expression_delimiters: &ExpressionDelimiters,
) -> Option<(Delimiter, usize)> {
    match (
        find_expression_open(text, expression_delimiters.open.as_str()),
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

fn walk_template<F>(text: &str, expression_delimiters: &ExpressionDelimiters, mut visit: F)
where
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
            Delimiter::Expression => find_expression_close(after_start, end_delimiter),
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

fn find_expression_open(text: &str, open_delimiter: &str) -> Option<usize> {
    find_exact_delimiter(text, open_delimiter)
}

fn find_expression_close(text: &str, close_delimiter: &str) -> Option<usize> {
    find_exact_delimiter(text, close_delimiter)
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

fn parse_for_loop_scope(
    expression: &str,
    scopes: &[LoopScope],
    tokens: &mut BTreeSet<VariableName>,
) -> Option<ParsedForLoop> {
    let trimmed = expression.trim();
    let remainder = trimmed.strip_prefix("for ")?;
    let (binding, iterable) = remainder.split_once(" in ")?;
    collect_identifiers(iterable, scopes, tokens);

    let bound_names = binding
        .split(',')
        .filter_map(|candidate| {
            let candidate = candidate
                .trim()
                .trim_matches(|character: char| matches!(character, '(' | ')'));
            if candidate.is_empty() {
                return None;
            }
            let root = candidate.split('.').next().unwrap_or(candidate);
            Some(root.to_string())
        })
        .collect();
    Some(ParsedForLoop {
        scope: LoopScope { bound_names },
        iterable: iterable.trim().to_owned(),
    })
}

struct ParsedForLoop {
    scope: LoopScope,
    iterable: String,
}

fn is_bare_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn parse_set_scope(
    expression: &str,
    scopes: &[LoopScope],
    tokens: &mut BTreeSet<VariableName>,
) -> Option<String> {
    let remainder = expression.trim().strip_prefix("set ")?;
    let (name, value) = remainder.split_once('=')?;
    let name = name.trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    collect_identifiers(value, scopes, tokens);
    Some(name.to_owned())
}

fn collect_identifiers(
    expression: &str,
    scopes: &[LoopScope],
    tokens: &mut BTreeSet<VariableName>,
) {
    const KEYWORDS: &[&str] = &[
        "if",
        "else",
        "elif",
        "endif",
        "for",
        "endfor",
        "in",
        "set",
        "true",
        "false",
        "none",
        "not",
        "and",
        "or",
        "block",
        "endblock",
        "macro",
        "endmacro",
        "filter",
        "endfilter",
    ];

    let bound_names = scopes
        .iter()
        .flat_map(|scope| scope.bound_names.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();

    let masked_expression = mask_filter_names(&mask_quoted_literals(expression));
    for candidate in masked_expression.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    }) {
        if candidate.is_empty()
            || KEYWORDS.contains(&candidate)
            || !candidate
                .chars()
                .any(|character| character.is_ascii_alphabetic())
        {
            continue;
        }
        let root = candidate.split('.').next().unwrap_or(candidate);
        if bound_names.contains(root) || is_loop_context_name(candidate, &masked_expression, scopes)
        {
            continue;
        }
        if let Ok(variable) = VariableName::new(candidate) {
            tokens.insert(variable);
        }
    }
}

fn is_loop_context_name(candidate: &str, expression: &str, scopes: &[LoopScope]) -> bool {
    if scopes.len() <= 1 {
        return false;
    }
    if candidate != "loop.cycle" {
        return LOOP_CONTEXT_NAMES.contains(&candidate);
    }

    let Some(start) = expression.find(candidate) else {
        return false;
    };
    expression[start + candidate.len()..]
        .trim_start()
        .starts_with('(')
}

fn mask_quoted_literals(expression: &str) -> String {
    let mut masked = String::with_capacity(expression.len());
    let mut quote = None;
    let mut escaped = false;
    for character in expression.chars() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            masked.push(' ');
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
            masked.push(' ');
        } else {
            masked.push(character);
        }
    }
    masked
}

fn mask_filter_names(expression: &str) -> String {
    let mut masked = expression.chars().collect::<Vec<_>>();
    let mut cursor = 0;
    while cursor < masked.len() {
        if masked[cursor] != '|' {
            cursor += 1;
            continue;
        }
        let mut filter_start = cursor + 1;
        while filter_start < masked.len() && masked[filter_start].is_ascii_whitespace() {
            filter_start += 1;
        }
        let mut filter_end = filter_start;
        while filter_end < masked.len()
            && (masked[filter_end].is_ascii_alphanumeric() || masked[filter_end] == '_')
        {
            filter_end += 1;
        }
        for character in &mut masked[filter_start..filter_end] {
            *character = ' ';
        }
        cursor = filter_end;
    }
    masked.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{discover_tokens, has_bare_for_loop_over};

    #[test]
    fn leading_statement_whitespace_control_marker_is_not_a_token() {
        let tokens = discover_tokens("{%- if true %}Hi{% endif %}");

        assert!(tokens.is_empty(), "unexpected tokens: {tokens:?}");
    }

    #[test]
    fn trailing_statement_whitespace_control_marker_is_not_a_token() {
        let tokens = discover_tokens("{% if true -%}Hi{% endif %}");

        assert!(tokens.is_empty(), "unexpected tokens: {tokens:?}");
    }

    #[test]
    fn trailing_plus_marker_is_shared_by_token_and_loop_discovery() {
        let template = "{% for item in items +%}{{ item }}{% endfor %}";

        assert_eq!(
            discover_tokens(template),
            [crate::VariableName::new("items").unwrap()].into()
        );
        assert!(has_bare_for_loop_over(template, "items"));
    }

    #[test]
    fn both_statement_whitespace_control_markers_are_not_tokens() {
        let tokens = discover_tokens("{%- if true -%}Hi{%- endif -%}");

        assert!(tokens.is_empty(), "unexpected tokens: {tokens:?}");
    }

    #[test]
    fn expression_whitespace_control_markers_are_stripped() {
        let tokens = discover_tokens("{{- name -}}");

        assert_eq!(tokens, [crate::VariableName::new("name").unwrap()].into());
    }

    #[test]
    fn kebab_case_variable_names_remain_intact() {
        let tokens = discover_tokens("{{ task-id }}");

        assert_eq!(
            tokens,
            [crate::VariableName::new("task-id").unwrap()].into()
        );
    }

    #[test]
    fn numeric_subscripts_and_slices_do_not_become_tokens() {
        let tokens = discover_tokens("{{ items[0] }} {{ items[1:2] }}");

        assert_eq!(tokens, [crate::VariableName::new("items").unwrap()].into());
    }

    #[test]
    fn loop_builtins_are_not_discovered_inside_a_loop() {
        let tokens = discover_tokens("{% for it in items %}{{ it }}{{ loop.index0 }}{% endfor %}");

        assert_eq!(tokens, [crate::VariableName::new("items").unwrap()].into());
    }

    #[test]
    fn binary_operator_fragments_are_not_tokens() {
        let tokens = discover_tokens("{{ a - b }}");

        assert_eq!(
            tokens,
            [
                crate::VariableName::new("a").unwrap(),
                crate::VariableName::new("b").unwrap(),
            ]
            .into()
        );
    }

    #[test]
    fn filter_names_are_not_discovered_as_variables() {
        let tokens = discover_tokens("{{ x | e }} {{ x | e | lower }}");

        assert_eq!(tokens, [crate::VariableName::new("x").unwrap()].into());
    }

    #[test]
    fn set_locals_and_filter_names_are_not_discovered_as_variables() {
        let tokens = discover_tokens("{% set greeting = 'Hi ' + name %}{{ greeting | e | lower }}");

        assert_eq!(tokens, [crate::VariableName::new("name").unwrap()].into());
    }

    #[test]
    fn loop_builtins_outside_a_loop_remain_undeclared() {
        let tokens = discover_tokens("{{ loop.last }}");

        assert!(tokens.contains(&crate::VariableName::new("loop.last").unwrap()));
    }

    #[test]
    fn filter_argument_references_remain_discovered() {
        let tokens = discover_tokens("{{ x | default(other_var) }}");

        assert!(tokens.contains(&crate::VariableName::new("x").unwrap()));
        assert!(tokens.contains(&crate::VariableName::new("other_var").unwrap()));
    }
}
