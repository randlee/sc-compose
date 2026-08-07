use std::collections::BTreeSet;

use crate::types::VariableName;

use super::scope::LoopScope;

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

pub(super) fn collect_identifiers(
    expression: &str,
    scopes: &[LoopScope],
    tokens: &mut BTreeSet<VariableName>,
) {
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
