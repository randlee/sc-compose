use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use sc_composer::{
    ComposePolicy, ComposeRequest, ConfiningRoot, Diagnostic, DiagnosticCode, DiagnosticSeverity,
    JsonEscapeMode, expand_includes, resolve_json_escape_mode, strip_template_suffix,
};
use serde_json::{Value, json};

use crate::CommandError;

pub(crate) fn lint_request(request: &ComposeRequest) -> Result<Vec<Diagnostic>, CommandError> {
    let resolved = sc_composer::resolve_template_path(request).map_err(CommandError::compose)?;
    let expanded =
        sc_composer::expand_includes(&resolved.resolved_path, &request.root, &request.policy)
            .map_err(CommandError::compose)?;

    let mode = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| {
            (path == &resolved.resolved_path)
                .then(|| {
                    passes
                        .first()
                        .and_then(sc_composer::Frontmatter::json_escape_mode)
                })
                .flatten()
        })
        .map_or(JsonEscapeMode::Auto, |frontmatter_mode| {
            resolve_json_escape_mode(request.policy.json_escape_mode, Some(frontmatter_mode))
        });
    let json_context = is_json_template_path(&resolved.resolved_path);

    Ok(expanded
        .source_texts
        .iter()
        .flat_map(|(path, source)| {
            let chain = expanded
                .include_chains
                .get(path)
                .cloned()
                .unwrap_or_default();
            lint_source_with_mode(path, source, mode, json_context, &chain)
        })
        .collect())
}

#[cfg(test)]
fn lint_source(path: &Path, source: &str) -> Vec<Diagnostic> {
    lint_source_with_mode(path, source, JsonEscapeMode::Auto, false, &[])
}

fn lint_source_with_mode(
    path: &Path,
    source: &str,
    mode: JsonEscapeMode,
    json_context: bool,
    include_chain: &[PathBuf],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut search_start = 0;

    while let Some((open_offset, expression_start, close_offset)) =
        next_variable_expression(source, search_start)
    {
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
                .with_location(line, column)
                .with_include_chain(include_chain.to_vec()),
            );
        }

        if (json_context || is_json_template_path(path))
            && is_literal_quoted_scalar(source, open_offset, close_offset)
            && is_scalar_expression(expression)
        {
            let (line, column) = line_and_column(source, open_offset);
            let (severity, message) = match mode {
                JsonEscapeMode::Legacy => (
                    DiagnosticSeverity::Warning,
                    sc_composer::JSON_LEGACY_WARNING.to_owned(),
                ),
                JsonEscapeMode::Auto => (
                    DiagnosticSeverity::Error,
                    format!(
                        "quoted JSON placeholder `{}` is incompatible with auto escape mode; migrate to a bare placeholder. See docs/migration/json-escape-mode.md",
                        expression.trim()
                    ),
                ),
            };
            diagnostics.push(
                Diagnostic::new(severity, DiagnosticCode::WarnJsonLegacyEscapeMode, message)
                    .with_path(path)
                    .with_location(line, column)
                    .with_include_chain(include_chain.to_vec()),
            );
        }

        search_start = close_offset + 2;
    }

    diagnostics
}

fn is_json_template_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    Path::new(strip_template_suffix(name))
        .extension()
        .and_then(|extension| extension.to_str())
        == Some("json")
}

fn is_literal_quoted_scalar(source: &str, open_offset: usize, close_offset: usize) -> bool {
    let before = source[..open_offset]
        .chars()
        .rev()
        .find(|character| !character.is_whitespace());
    let after = source[close_offset + 2..]
        .chars()
        .find(|character| !character.is_whitespace());
    before == Some('"') && after == Some('"')
}

fn is_scalar_expression(expression: &str) -> bool {
    let expression = expression.trim();
    if expression.is_empty()
        || expression.starts_with('"')
        || expression.starts_with('\'')
        || expression.contains("{%")
        || expression.contains("{{")
    {
        return false;
    }
    let base = expression
        .split('|')
        .next()
        .and_then(|part| part.split_whitespace().next())
        .unwrap_or_default();
    base.split('.')
        .all(|part| !part.is_empty() && is_identifier(part))
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Return the next rendered Jinja variable while skipping comments and
/// malformed/unterminated blocks. This is deliberately a small lexical pass;
/// expression semantics remain owned by Minijinja and the renderer.
fn next_variable_expression(
    source: &str,
    mut search_start: usize,
) -> Option<(usize, usize, usize)> {
    while search_start < source.len() {
        let remainder = &source[search_start..];
        let variable = remainder.find("{{");
        let comment = remainder.find("{#");
        match (variable, comment) {
            (None, None) => return None,
            (Some(variable), Some(comment)) if comment < variable => {
                let comment_start = search_start + comment;
                let end = source[comment_start + 2..].find("#}")?;
                search_start = comment_start + 2 + end + 2;
            }
            (Some(variable), _) => {
                let open_offset = search_start + variable;
                let expression_start = open_offset + 2;
                let close_offset = expression_start + source[expression_start..].find("}}")?;
                return Some((open_offset, expression_start, close_offset));
            }
            (None, Some(comment)) => {
                let comment_start = search_start + comment;
                let end = source[comment_start + 2..].find("#}")?;
                search_start = comment_start + 2 + end + 2;
            }
        }
    }
    None
}

/// Structured result for the repository-level template contract target.
#[derive(Debug)]
pub(crate) struct TemplateContractsReport {
    pub(crate) payload: Value,
    pub(crate) raw_json: String,
    pub(crate) exit_status: i32,
}

/// Scan every repository JSON template with the same source scanner used by
/// `validate --lint`. No external tools or duplicate parser are involved.
pub(crate) fn lint_repository_templates(
    root: &Path,
) -> Result<TemplateContractsReport, CommandError> {
    let templates = discover_json_templates(root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigRead)
    })?;
    let confining_root = ConfiningRoot::new(root).map_err(|error| {
        CommandError::usage_with_code(anyhow!(error), DiagnosticCode::ErrConfigMode)
    })?;
    let mut findings = Vec::new();
    let mut diagnostics = Vec::new();
    let mut template_count = 0usize;
    let mut error_count = 0usize;

    for template in templates {
        template_count += 1;
        let (template_findings, template_diagnostics, template_errors) =
            lint_repository_template(&template, &confining_root)?;
        findings.extend(template_findings);
        diagnostics.extend(template_diagnostics);
        error_count += template_errors;
    }

    let payload = json!({
        "ok": error_count == 0,
        "command": "template-contracts",
        "data": {
            "status": if error_count == 0 { "pass" } else { "fail" },
            "templates_scanned": template_count,
            "context_backed_render": false,
            "findings": findings,
            "diagnostics": diagnostics,
        },
        "diagnostics": diagnostics,
    });
    let raw_json = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| String::from("{}"));
    Ok(TemplateContractsReport {
        payload,
        raw_json,
        exit_status: if error_count == 0 { 0 } else { 2 },
    })
}

fn lint_repository_template(
    template: &Path,
    confining_root: &ConfiningRoot,
) -> Result<(Vec<Value>, Vec<Value>, usize), CommandError> {
    let expanded = expand_includes(template, confining_root, &ComposePolicy::default())
        .map_err(CommandError::compose)?;
    let mode = expanded
        .frontmatters
        .iter()
        .find_map(|(path, passes)| {
            (path == template)
                .then(|| {
                    passes
                        .first()
                        .and_then(sc_composer::Frontmatter::json_escape_mode)
                })
                .flatten()
        })
        .map_or(JsonEscapeMode::Auto, |declared| {
            resolve_json_escape_mode(None, Some(declared))
        });
    let mode_name = match mode {
        JsonEscapeMode::Auto => "auto",
        JsonEscapeMode::Legacy => "legacy",
    };
    let mut findings = Vec::new();
    let mut diagnostics = Vec::new();
    let mut error_count = 0;

    for (path, source) in &expanded.source_texts {
        let chain = expanded
            .include_chains
            .get(path)
            .cloned()
            .unwrap_or_default();
        for diagnostic in lint_source_with_mode(path, source, mode, true, &chain) {
            if diagnostic.severity == DiagnosticSeverity::Error {
                error_count += 1;
            }
            let diagnostic_code = diagnostic.code.as_str();
            let diagnostic_json = json!(diagnostic);
            diagnostics.push(diagnostic_json.clone());
            findings.push(json!({
                "template": path.to_string_lossy().replace('\\', "/"),
                "mode": mode_name,
                "location": {
                    "line": diagnostic.line,
                    "column": diagnostic.column,
                },
                "diagnostic": diagnostic_json,
                "diagnostic_code": diagnostic_code,
                "migration_recommendation": sc_composer::JSON_LEGACY_WARNING,
                "context_backed_render": false,
            }));
        }
    }

    if matches!(mode, JsonEscapeMode::Legacy) && findings.is_empty() {
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticCode::WarnJsonLegacyEscapeMode,
            sc_composer::JSON_LEGACY_WARNING,
        )
        .with_path(template.to_path_buf());
        let diagnostic_code = diagnostic.code.as_str();
        let diagnostic_json = json!(diagnostic);
        diagnostics.push(diagnostic_json.clone());
        findings.push(json!({
            "template": template.to_string_lossy().replace('\\', "/"),
            "mode": mode_name,
            "location": {"line": null, "column": null},
            "diagnostic": diagnostic_json,
            "diagnostic_code": diagnostic_code,
            "migration_recommendation": sc_composer::JSON_LEGACY_WARNING,
            "context_backed_render": false,
        }));
    }

    Ok((findings, diagnostics, error_count))
}

fn discover_json_templates(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut templates = Vec::new();
    visit_template_dir(root, root, &mut templates)?;
    templates.sort();
    Ok(templates)
}

fn visit_template_dir(
    root: &Path,
    directory: &Path,
    templates: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if matches!(name.as_ref(), ".git" | "target" | "reports" | ".sc") {
                continue;
            }
            visit_template_dir(root, &path, templates)?;
        } else if path.is_file() && is_json_template_path(&path) {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            templates.push(root.join(relative));
        }
    }
    Ok(())
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

    #[test]
    fn finds_auto_mode_quoted_json_scalar_with_location() {
        let diagnostics = super::lint_source_with_mode(
            std::path::Path::new("payload.json.j2"),
            "{\"value\": \"{{ value }}\"}\n",
            sc_composer::JsonEscapeMode::Auto,
            true,
            &[],
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].severity,
            sc_composer::DiagnosticSeverity::Error
        );
        assert_eq!(
            diagnostics[0].code,
            DiagnosticCode::WarnJsonLegacyEscapeMode
        );
        assert_eq!(diagnostics[0].line, Some(1));
        assert_eq!(diagnostics[0].column, Some(12));
    }

    #[test]
    fn legacy_mode_warns_but_comments_and_jinja_literals_are_ignored() {
        let diagnostics = super::lint_source_with_mode(
            std::path::Path::new("payload.json.j2"),
            "{# \"ignored\": \"{{ comment }}\" #}\n{\"value\": \"{{ value }}\"}\n{{ \"{{ literal }}\" }}\n",
            sc_composer::JsonEscapeMode::Legacy,
            true,
            &[],
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].severity,
            sc_composer::DiagnosticSeverity::Warning
        );
        assert_eq!(diagnostics[0].message, sc_composer::JSON_LEGACY_WARNING);
    }

    #[test]
    fn bare_json_placeholder_is_clean() {
        let diagnostics = super::lint_source_with_mode(
            std::path::Path::new("payload.json.j2"),
            "{\"value\": {{ value }}}\n",
            sc_composer::JsonEscapeMode::Auto,
            true,
            &[],
        );

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn filtered_json_placeholder_is_a_conservative_finding() {
        let diagnostics = super::lint_source_with_mode(
            std::path::Path::new("payload.json.j2"),
            "{\"value\": \"{{ value | upper }}\"}\n",
            sc_composer::JsonEscapeMode::Auto,
            true,
            &[],
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].severity,
            sc_composer::DiagnosticSeverity::Error
        );
        assert!(diagnostics[0].message.contains("incompatible with auto"));
    }
}
