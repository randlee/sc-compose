use std::path::Path;

use sc_composer::{ComposeRequest, Diagnostic, DiagnosticCode, DiagnosticSeverity};

use crate::CommandError;

pub(crate) fn lint_request(request: &ComposeRequest) -> Result<Vec<Diagnostic>, CommandError> {
    let resolved = sc_composer::resolve_template_path(request).map_err(CommandError::compose)?;
    let expanded =
        sc_composer::expand_includes(&resolved.resolved_path, &request.root, &request.policy)
            .map_err(CommandError::compose)?;

    Ok(expanded
        .source_texts
        .iter()
        .flat_map(|(path, source)| lint_source(path, source))
        .collect())
}

fn lint_source(path: &Path, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut search_start = 0;

    while let Some(open_offset) = source[search_start..].find("{{") {
        let open_offset = search_start + open_offset;
        let expression_start = open_offset + 2;
        let Some(close_offset) = source[expression_start..].find("}}") else {
            break;
        };
        let close_offset = expression_start + close_offset;
        let expression = &source[expression_start..close_offset];

        if let Some(chain_offset) = redundant_chain_offset(expression) {
            let source_offset = expression_start + chain_offset;
            let (line, column) = line_and_column(source, source_offset);
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticSeverity::Warning,
                    DiagnosticCode::WarnLintRedundantFilterChain,
                    "redundant filter chain `frontmatter_safe | yaml_safe`; recommendation: use `yaml_safe` alone",
                )
                .with_path(path)
                .with_location(line, column),
            );
        }

        search_start = close_offset + 2;
    }

    diagnostics
}

fn redundant_chain_offset(expression: &str) -> Option<usize> {
    let mut search_start = 0;
    while let Some(relative_offset) = expression[search_start..].find("frontmatter_safe") {
        let frontmatter_offset = search_start + relative_offset;
        if !is_identifier_boundary(expression, frontmatter_offset, "frontmatter_safe") {
            search_start = frontmatter_offset + "frontmatter_safe".len();
            continue;
        }

        let after_frontmatter =
            skip_whitespace(expression, frontmatter_offset + "frontmatter_safe".len());
        if expression.as_bytes().get(after_frontmatter) != Some(&b'|') {
            search_start = frontmatter_offset + "frontmatter_safe".len();
            continue;
        }

        let yaml_offset = skip_whitespace(expression, after_frontmatter + 1);
        if is_identifier_boundary(expression, yaml_offset, "yaml_safe") {
            return Some(frontmatter_offset);
        }
        search_start = frontmatter_offset + "frontmatter_safe".len();
    }
    None
}

fn is_identifier_boundary(source: &str, offset: usize, identifier: &str) -> bool {
    let Some(candidate) = source.get(offset..offset + identifier.len()) else {
        return false;
    };
    if candidate != identifier {
        return false;
    }

    let before_is_identifier = source[..offset]
        .chars()
        .next_back()
        .is_some_and(is_identifier_character);
    let after_is_identifier = source[offset + identifier.len()..]
        .chars()
        .next()
        .is_some_and(is_identifier_character);
    !before_is_identifier && !after_is_identifier
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn skip_whitespace(source: &str, mut offset: usize) -> usize {
    while source[offset..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        offset += source[offset..].chars().next().map_or(0, char::len_utf8);
    }
    offset
}

fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(1, |line| line.chars().count() + 1);
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::lint_source;
    use sc_composer::DiagnosticCode;

    #[test]
    fn finds_redundant_chain_with_source_location() {
        let diagnostics = lint_source(
            std::path::Path::new("template.md.j2"),
            "title: static\nvalue: {{ value | frontmatter_safe | yaml_safe }}\n",
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            DiagnosticCode::WarnLintRedundantFilterChain
        );
        assert_eq!(diagnostics[0].line, Some(2));
        assert_eq!(diagnostics[0].column, Some(19));
    }

    #[test]
    fn ignores_similar_text_outside_a_variable_expression() {
        let diagnostics = lint_source(
            std::path::Path::new("template.md.j2"),
            "frontmatter_safe | yaml_safe\n{{ value | yaml_safe }}\n",
        );

        assert!(diagnostics.is_empty());
    }
}
