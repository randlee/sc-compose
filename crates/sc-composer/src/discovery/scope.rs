use std::collections::BTreeSet;

use crate::types::VariableName;

use super::identifiers::collect_identifiers;

#[derive(Clone, Debug, Default)]
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
    let iterable = iterable
        .trim()
        .strip_suffix(" recursive")
        .unwrap_or(iterable)
        .trim();

    let (iterable, condition) = iterable
        .split_once(" if ")
        .map_or((iterable, None), |(iterable, condition)| {
            (iterable, Some(condition))
        });
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
    let scope = LoopScope { bound_names };
    if let Some(condition) = condition {
        let mut condition_scopes = scopes.to_vec();
        condition_scopes.push(LoopScope {
            bound_names: scope.bound_names.clone(),
        });
        collect_identifiers(condition, &condition_scopes, tokens);
    }
    Some(ParsedForLoop { scope })
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
) -> Option<Vec<String>> {
    let remainder = expression.trim().strip_prefix("set ")?;
    let (bindings, value) = remainder
        .split_once('=')
        .map_or((remainder, None), |(bindings, value)| {
            (bindings, Some(value))
        });
    let names = parse_bindings(bindings)?;
    if let Some(value) = value {
        collect_identifiers(value, scopes, tokens);
    }
    Some(names)
}

pub(super) fn parse_with_scope(
    expression: &str,
    scopes: &[LoopScope],
    tokens: &mut BTreeSet<VariableName>,
) -> Option<LoopScope> {
    let remainder = expression.trim().strip_prefix("with ")?;
    let mut bound_names = BTreeSet::new();
    for assignment in remainder.split(',') {
        let (binding, value) = assignment.split_once('=')?;
        let names = parse_bindings(binding)?;
        collect_identifiers(value, scopes, tokens);
        bound_names.extend(names);
    }
    Some(LoopScope { bound_names })
}

pub(super) fn parse_macro_scope(expression: &str) -> Option<(String, LoopScope)> {
    let remainder = expression.trim().strip_prefix("macro ")?.trim();
    let open = remainder.find('(')?;
    let close = remainder.rfind(')')?;
    let name = parse_bindings(&remainder[..open])?.into_iter().next()?;
    let params = if remainder[open + 1..close].trim().is_empty() {
        Vec::new()
    } else {
        parse_bindings(&remainder[open + 1..close])?
    };
    Some((
        name,
        LoopScope {
            bound_names: params.into_iter().chain(["caller".to_owned()]).collect(),
        },
    ))
}

fn parse_bindings(bindings: &str) -> Option<Vec<String>> {
    let bindings = bindings
        .trim()
        .trim_matches(|character| matches!(character, '(' | ')'));
    let names = bindings
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if names.is_empty()
        || names.iter().any(|name| {
            !name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        })
    {
        return None;
    }
    Some(names)
}
