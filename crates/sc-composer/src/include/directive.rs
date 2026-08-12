/// A native include directive after static candidate analysis.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum IncludeDirective {
    /// One statically known include path.
    Static(String),
    /// A Jinja conditional path whose branches are both statically known.
    Conditional {
        condition: String,
        candidates: Vec<String>,
    },
    /// A native include was found, but its target cannot be enumerated safely.
    Dynamic,
}

pub(super) fn parse_include_directive(line: &str) -> Option<IncludeDirective> {
    let trimmed = line.trim();
    if !trimmed.starts_with("@<") || !trimmed.ends_with('>') || trimmed.len() <= 3 {
        return None;
    }

    let target = &trimmed[2..trimmed.len() - 1];
    if let Some(expression) = target
        .trim()
        .strip_prefix("{{")
        .and_then(|value| value.strip_suffix("}}"))
    {
        return Some(parse_conditional_expression(expression));
    }

    Some(IncludeDirective::Static(target.to_owned()))
}

fn parse_conditional_expression(expression: &str) -> IncludeDirective {
    let expression = expression.trim();
    if let Some((then_branch, rest)) = expression.split_once(" if ")
        && let Some((condition, else_branch)) = rest.split_once(" else ")
        && let (Some(then_path), Some(else_path)) =
            (quoted_literal(then_branch), quoted_literal(else_branch))
        && !condition.trim().is_empty()
    {
        return IncludeDirective::Conditional {
            condition: condition.trim().to_owned(),
            candidates: vec![then_path, else_path],
        };
    }

    quoted_literal(expression).map_or(IncludeDirective::Dynamic, IncludeDirective::Static)
}

fn quoted_literal(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.as_bytes().first().copied()?;
    if !matches!(quote, b'\'' | b'"') || value.as_bytes().last().copied()? != quote {
        return None;
    }
    let literal = &value[1..value.len() - 1];
    (!literal.is_empty()
        && !literal.chars().any(char::is_control)
        && !literal.contains("{{")
        && !literal.contains("}}")
        // A chained conditional can begin and end with quotes while the
        // middle arm still contains `if`/`else` expression text. Treat that
        // as an expression boundary rather than a single literal path.
        && !literal.contains(" if ")
        && !literal.contains(" else "))
    .then(|| literal.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{IncludeDirective, parse_include_directive};

    #[test]
    fn chained_ternary_is_dynamic_instead_of_a_garbage_literal() {
        let line = "@<{{ \"a.md\" if x else \"b.md\" if y else \"c.md\" }}>";

        assert_eq!(
            parse_include_directive(line),
            Some(IncludeDirective::Dynamic)
        );
    }

    #[test]
    fn single_ternary_remains_conditional() {
        let line = "@<{{ \"a.md\" if x else \"b.md\" }}>";

        assert_eq!(
            parse_include_directive(line),
            Some(IncludeDirective::Conditional {
                condition: "x".to_owned(),
                candidates: vec!["a.md".to_owned(), "b.md".to_owned()],
            })
        );
    }
}
