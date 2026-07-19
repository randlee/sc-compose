//! Frontmatter initialization helper.

use std::collections::BTreeMap;
use std::path::Path;

use crate::frontmatter::parse_template_document;
use crate::resolver::canonicalize_with_roots;
use crate::validation::discover_tokens;
use crate::{
    ComposeError, ConfigError, DiagnosticCode, FrontmatterInitResult, InitPass, VariableName,
};

/// Insert or rewrite normalized frontmatter for a single template.
///
/// # Errors
///
/// Returns [`ComposeError`] when the target cannot be read or already contains
/// frontmatter and `force` is not enabled.
pub fn frontmatter_init(
    path: impl AsRef<Path>,
    force: bool,
    dry_run: bool,
) -> Result<FrontmatterInitResult, ComposeError> {
    let path = path.as_ref();
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    let canonical = canonicalize_with_roots(path, root, &[])?;
    let contents = std::fs::read_to_string(&canonical).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!("failed to read template: {}", canonical.display()),
        )
        .with_source(error)
    })?;
    let parsed = parse_template_document(&contents)?;
    if parsed.frontmatter().is_some() && !force {
        return Err(ConfigError::new(
            DiagnosticCode::ErrConfigReadonly,
            "frontmatter already exists; rerun with --force to rewrite it",
        )
        .into());
    }

    let would_change = parsed.frontmatter().is_none() || force;
    let discovered = discover_tokens(parsed.body())
        .into_iter()
        .collect::<Vec<_>>();
    let frontmatter_text = build_frontmatter(&discovered);
    let template_text = format!("{frontmatter_text}{}", parsed.body());
    if !dry_run {
        std::fs::write(&canonical, &template_text).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigReadonly,
                format!("failed to write template: {}", canonical.display()),
            )
            .with_source(error)
        })?;
    }

    Ok(FrontmatterInitResult {
        target_path: canonical,
        frontmatter_text,
        template_text,
        discovered_variables: discovered,
        changed: !dry_run && would_change,
        would_change,
    })
}

/// Convert a concrete file into a stacked template using pass-scoped
/// replacements.
///
/// # Errors
///
/// Returns [`ComposeError`] when the file cannot be read, already contains
/// frontmatter without `force`, any literal replacement is missing, or the
/// rewritten template cannot be written.
pub fn template_init(
    path: impl AsRef<Path>,
    passes: &[InitPass],
    force: bool,
    dry_run: bool,
) -> Result<FrontmatterInitResult, ComposeError> {
    let path = path.as_ref();
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    let canonical = canonicalize_with_roots(path, root, &[])?;
    let contents = std::fs::read_to_string(&canonical).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!("failed to read template: {}", canonical.display()),
        )
        .with_source(error)
    })?;
    let parsed = parse_template_document(&contents)?;
    if parsed.frontmatter().is_some() && !force {
        return Err(ConfigError::new(
            DiagnosticCode::ErrConfigReadonly,
            "frontmatter already exists; rerun with --force to rewrite it",
        )
        .into());
    }
    if passes.is_empty() {
        return Err(ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            "template-init requires at least one --pass N group",
        )
        .into());
    }

    let mut replacements = Vec::new();
    for init_pass in passes {
        let brace_count = usize::from(init_pass.pass_number) + 1;
        for (name, value) in &init_pass.variables {
            replacements.push(Replacement {
                pass_number: init_pass.pass_number,
                variable_name: name.clone(),
                value: value.clone(),
                brace_count,
            });
        }
    }
    replacements.sort_by(|left, right| {
        right
            .value
            .len()
            .cmp(&left.value.len())
            .then_with(|| right.pass_number.cmp(&left.pass_number))
    });

    let mut missing = Vec::new();
    for replacement in &replacements {
        if !contents.contains(&replacement.value) {
            missing.push(format!(
                "{}={}",
                replacement.variable_name.as_str(),
                replacement.value
            ));
        }
    }
    if !missing.is_empty() {
        return Err(ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!(
                "values not found in file: {}. Check for typos or differences in whitespace/encoding.",
                missing.join(", ")
            ),
        )
        .into());
    }

    let mut rewritten_body = contents.clone();
    for replacement in &replacements {
        rewritten_body = rewritten_body.replace(&replacement.value, &replacement.render_token());
    }

    let frontmatter_text = build_stacked_frontmatter(passes);
    let template_text = format!(
        "{frontmatter_text}{}",
        rewritten_body.trim_start_matches(['\n', '\r'])
    );
    let would_change = template_text != contents;

    if !dry_run && would_change {
        std::fs::write(&canonical, &template_text).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigReadonly,
                format!("failed to write template: {}", canonical.display()),
            )
            .with_source(error)
        })?;
    }

    Ok(FrontmatterInitResult {
        target_path: canonical,
        frontmatter_text,
        template_text,
        discovered_variables: replacements
            .iter()
            .map(|replacement| replacement.variable_name.clone())
            .collect(),
        changed: !dry_run && would_change,
        would_change,
    })
}

fn build_frontmatter(discovered: &[VariableName]) -> String {
    let mut text = String::from("---\nrequired_variables:\n");
    for variable in discovered {
        text.push_str("  - ");
        text.push_str(variable.as_str());
        text.push('\n');
    }
    text.push_str("defaults: {}\nmetadata: {}\n---\n");
    text
}

#[derive(Clone, Debug)]
struct Replacement {
    pass_number: u8,
    variable_name: VariableName,
    value: String,
    brace_count: usize,
}

impl Replacement {
    fn render_token(&self) -> String {
        format!(
            "{} {} {}",
            "{".repeat(self.brace_count),
            self.variable_name.as_str(),
            "}".repeat(self.brace_count)
        )
    }
}

#[derive(serde::Serialize)]
struct TemplateInitHeader {
    #[serde(skip_serializing_if = "Option::is_none")]
    pass: Option<u8>,
    required_variables: Vec<String>,
    defaults: BTreeMap<String, String>,
}

fn build_stacked_frontmatter(passes: &[InitPass]) -> String {
    let single_pass_compat = passes.len() == 1 && passes[0].pass_number <= 1;
    let mut text = String::new();
    for init_pass in passes {
        let header = TemplateInitHeader {
            pass: if single_pass_compat {
                None
            } else {
                Some(init_pass.pass_number)
            },
            required_variables: init_pass
                .variables
                .iter()
                .map(|(name, _)| name.as_str().to_owned())
                .collect(),
            defaults: init_pass
                .variables
                .iter()
                .map(|(name, value)| (name.as_str().to_owned(), value.clone()))
                .collect(),
        };
        let yaml = serde_yaml::to_string(&header)
            .unwrap_or_else(|error| panic!("failed to serialize template-init header: {error}"));
        text.push_str("---\n");
        text.push_str(&yaml);
        text.push_str("---\n");
    }
    text
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{ComposeError, InitPass, VariableName, frontmatter_init, template_init};

    #[test]
    fn dry_run_reports_frontmatter_without_writing_file() {
        let root = temp_root("frontmatter_init_dry_run");
        let template = root.join("template.md.j2");
        write_file(&template, "hello {{ name }}\n");

        let result = frontmatter_init(&template, false, true).unwrap();

        assert!(result.frontmatter_text.contains("required_variables"));
        assert!(!result.changed);
        assert!(result.would_change);
        assert_eq!(
            result.discovered_variables,
            vec![VariableName::new("name").unwrap()]
        );
        assert_eq!(fs::read_to_string(&template).unwrap(), "hello {{ name }}\n");
    }

    #[test]
    fn existing_frontmatter_requires_force() {
        let root = temp_root("frontmatter_init_force");
        let template = root.join("template.md.j2");
        write_file(
            &template,
            "---\nrequired_variables:\n  - name\n---\nhello {{ name }}\n",
        );

        let error = frontmatter_init(&template, false, true).unwrap_err();
        assert!(matches!(error, ComposeError::Config(_)));
    }

    #[test]
    fn frontmatter_init_discovers_iterable_from_loop_body_references() {
        let root = temp_root("frontmatter_init_loop_body");
        let template = root.join("template.md.j2");
        write_file(
            &template,
            "{% for sprint in sprints %}{{ sprint.id }} {{ sprint.stage }} {{ report.title }}{% endfor %}\n",
        );

        let result = frontmatter_init(&template, false, true).unwrap();

        assert_eq!(
            result.discovered_variables,
            vec![
                VariableName::new("report.title").unwrap(),
                VariableName::new("sprints").unwrap(),
            ]
        );
    }

    #[test]
    fn template_init_builds_multi_pass_template() {
        let root = temp_root("template_init_multi_pass");
        let template = root.join("agent.md");
        write_file(&template, "deploy test for wyvern");

        let result = template_init(
            &template,
            &[
                InitPass {
                    pass_number: 2,
                    variables: vec![(VariableName::new("team").unwrap(), "wyvern".to_owned())],
                },
                InitPass {
                    pass_number: 1,
                    variables: vec![(VariableName::new("task").unwrap(), "test".to_owned())],
                },
            ],
            false,
            true,
        )
        .unwrap();

        assert!(result.template_text.contains("pass: 2"));
        assert!(result.template_text.contains("pass: 1"));
        assert!(result.template_text.contains("{{{ team }}}"));
        assert!(result.template_text.contains("{{ task }}"));
    }

    #[test]
    fn template_init_omits_pass_one_for_single_pass_output() {
        let root = temp_root("template_init_single_pass");
        let template = root.join("agent.md");
        write_file(&template, "deploy test");

        let result = template_init(
            &template,
            &[InitPass {
                pass_number: 1,
                variables: vec![(VariableName::new("task").unwrap(), "test".to_owned())],
            }],
            false,
            true,
        )
        .unwrap();

        assert!(!result.template_text.contains("pass: 1"));
        assert!(result.template_text.contains("{{ task }}"));
    }

    #[test]
    fn template_init_replaces_longest_value_first() {
        let root = temp_root("template_init_longest_first");
        let template = root.join("agent.md");
        write_file(&template, "/home/wyvern/worktrees/wyvern");

        let result = template_init(
            &template,
            &[
                InitPass {
                    pass_number: 2,
                    variables: vec![(
                        VariableName::new("worktree_path").unwrap(),
                        "/home/wyvern/worktrees/wyvern".to_owned(),
                    )],
                },
                InitPass {
                    pass_number: 1,
                    variables: vec![(VariableName::new("team").unwrap(), "wyvern".to_owned())],
                },
            ],
            false,
            true,
        )
        .unwrap();

        assert!(result.template_text.contains("{{{ worktree_path }}}"));
        assert!(
            !result
                .template_text
                .contains("{{ team }}/worktrees/{{ team }}")
        );
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}
