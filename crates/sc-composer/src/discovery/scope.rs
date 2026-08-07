use std::collections::BTreeSet;

use crate::types::VariableName;

use super::identifiers::collect_identifiers;

#[derive(Debug, Default)]
pub(super) struct LoopScope {
    pub(super) bound_names: BTreeSet<String>,
}

pub(super) struct ParsedForLoop {
    pub(super) scope: LoopScope,
}

pub(super) fn parse_for_loop_scope(
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
    })
}

pub(super) fn parse_bare_for_loop_over(expression: &str, variable: &str) -> bool {
    let Some(remainder) = expression.strip_prefix("for ") else {
        return false;
    };
    let Some((_binding, iterable)) = remainder.split_once(" in ") else {
        return false;
    };
    let iterable = iterable.trim();
    iterable == variable && is_bare_identifier(iterable)
}

fn is_bare_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

pub(super) fn parse_set_scope(
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
