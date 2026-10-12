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

pub(super) fn parse_macro_scope(
    expression: &str,
    scopes: &[LoopScope],
    tokens: &mut BTreeSet<VariableName>,
) -> Option<(String, LoopScope)> {
    let remainder = expression.trim().strip_prefix("macro ")?.trim();
    let open = remainder.find('(')?;
    let close = remainder.rfind(')')?;
    let name = parse_bindings(&remainder[..open])?.into_iter().next()?;
    let params = parse_macro_params(&remainder[open + 1..close], scopes, tokens)?;
    Some((
        name,
        LoopScope {
            bound_names: params.into_iter().chain(["caller".to_owned()]).collect(),
        },
    ))
}

fn parse_macro_params(
    params: &str,
    scopes: &[LoopScope],
    tokens: &mut BTreeSet<VariableName>,
) -> Option<Vec<String>> {
    if params.trim().is_empty() {
        return Some(Vec::new());
    }

    let mut names = Vec::new();
    for parameter in split_top_level(params, ',') {
        let (name, default) = split_top_level_once(parameter, '=')
            .map_or((parameter, None), |(name, default)| (name, Some(default)));
        let name = parse_bindings(name)?.into_iter().next()?;
        if let Some(default) = default {
            let mut default_scopes = scopes.to_vec();
            default_scopes.push(LoopScope {
                bound_names: names.iter().cloned().collect(),
            });
            collect_identifiers(default, &default_scopes, tokens);
        }
        names.push(name);
    }
    Some(names)
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

fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, character) in input.char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '\"' => quote = Some(character),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if character == separator && depth == 0 => {
                parts.push(&input[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn split_top_level_once(input: &str, separator: char) -> Option<(&str, &str)> {
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, character) in input.char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '\"' => quote = Some(character),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if character == separator && depth == 0 => {
                return Some((&input[..index], &input[index + character.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}
