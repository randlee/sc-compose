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
    let mut scopes = Vec::<LoopScope>::new();
    let mut cursor = text;
    let expression_delimiters =
        ExpressionDelimiters::with_delimiters(open_delimiter, close_delimiter);

    while let Some((delimiter, start)) = next_delimiter(cursor, &expression_delimiters) {
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
        let expression = after_start[..end].trim();
        match delimiter {
            Delimiter::Expression => collect_identifiers(expression, &scopes, &mut tokens),
            Delimiter::Statement => {
                if let Some(loop_scope) = parse_for_loop_scope(expression, &scopes, &mut tokens) {
                    scopes.push(loop_scope);
                } else if expression.starts_with("endfor") {
                    scopes.pop();
                } else {
                    collect_identifiers(expression, &scopes, &mut tokens);
                }
            }
        }
        cursor = &after_start[end + end_delimiter.len()..];
    }
    tokens
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
) -> Option<LoopScope> {
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
    Some(LoopScope { bound_names })
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

    let masked_expression = mask_quoted_literals(expression);
    for candidate in masked_expression.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    }) {
        if candidate.is_empty() || KEYWORDS.contains(&candidate) {
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
    if scopes.is_empty() {
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
