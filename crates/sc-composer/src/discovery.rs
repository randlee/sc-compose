//! Statement-aware template variable discovery.

use std::collections::{BTreeMap, BTreeSet};

use crate::frontmatter::ParsedTemplate;
use crate::types::VariableName;

mod identifiers;
mod scanner;
mod scope;

use identifiers::collect_identifiers;
use scanner::{Delimiter, ExpressionDelimiters, walk_template};
use scope::{LoopScope, parse_bare_for_loop_over, parse_for_loop_scope, parse_set_scope};

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
    let expression_delimiters = ExpressionDelimiters::with_delimiters("{{", "}}");
    let mut found = false;
    walk_template(text, &expression_delimiters, |delimiter, expression| {
        if matches!(delimiter, Delimiter::Statement)
            && parse_bare_for_loop_over(expression, variable)
        {
            found = true;
            return true;
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

#[cfg(test)]
mod tests {
    use super::{
        discover_all_pass_tokens, discover_tokens, discover_tokens_with_brace_count,
        discover_tokens_with_delimiters, has_bare_for_loop_over,
    };
    use crate::parse_template_document;

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
    fn bare_for_loop_detection_matches_the_contract_matrix() {
        let cases = [
            (
                "simple bare loop",
                "{% for item in items %}{{ item }}{% endfor %}",
                "items",
                true,
            ),
            (
                "nested bare loop",
                "{% for group in groups %}{% for item in items %}{{ item }}{% endfor %}{% endfor %}",
                "items",
                true,
            ),
            (
                "filtered loop",
                "{% for item in items|sort %}{{ item }}{% endfor %}",
                "items",
                false,
            ),
            (
                "no loop",
                "{% if items %}{{ items }}{% endif %}",
                "items",
                false,
            ),
        ];

        for (description, template, variable, expected) in cases {
            assert_eq!(
                has_bare_for_loop_over(template, variable),
                expected,
                "unexpected result for {description}"
            );
        }
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

    #[test]
    fn custom_delimiters_are_scanned_without_matching_standard_expressions() {
        let tokens =
            discover_tokens_with_delimiters("{{ standard }} [[ custom.value ]]", "[[", "]]");

        assert_eq!(
            tokens,
            [crate::VariableName::new("custom.value").unwrap()].into()
        );
    }

    #[test]
    fn brace_count_and_unclosed_delimiters_keep_scanner_boundaries() {
        assert_eq!(
            discover_tokens_with_brace_count("{{{ outer }}} {{ inner }}", 3),
            [crate::VariableName::new("outer").unwrap()].into()
        );
        assert!(discover_tokens("{{ unclosed").is_empty());
        assert!(discover_tokens("{% for item in items").is_empty());
    }

    #[test]
    fn nested_shadowed_scopes_keep_outer_and_iterable_references() {
        let tokens = discover_tokens(
            "{% for item in items %}{% for item in nested %}{{ item.name }} {{ report.url }}{% endfor %}{% endfor %}",
        );

        assert_eq!(
            tokens,
            [
                crate::VariableName::new("items").unwrap(),
                crate::VariableName::new("nested").unwrap(),
                crate::VariableName::new("report.url").unwrap(),
            ]
            .into()
        );
    }

    #[test]
    fn quoted_literals_and_filter_names_are_masked_but_arguments_remain() {
        let tokens =
            discover_tokens("{{ \"literal.variable\" }} {{ value | default(fallback) | lower }}");

        assert_eq!(
            tokens,
            [
                crate::VariableName::new("fallback").unwrap(),
                crate::VariableName::new("value").unwrap(),
            ]
            .into()
        );
    }

    #[test]
    fn pass_maps_use_each_pass_brace_count() {
        let parsed = parse_template_document(
            "---\npass: 1\n---\n---\npass: 2\n---\n{{ inner }} {{{ outer }}}",
        )
        .unwrap();

        let tokens = discover_all_pass_tokens(&parsed);

        assert_eq!(
            tokens.get(&1).cloned().unwrap_or_default(),
            [crate::VariableName::new("inner").unwrap()].into()
        );
        assert_eq!(
            tokens.get(&2).cloned().unwrap_or_default(),
            [crate::VariableName::new("outer").unwrap()].into()
        );
    }
}
