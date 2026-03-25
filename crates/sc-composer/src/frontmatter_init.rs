//! Frontmatter initialization helper.

use std::path::Path;

use crate::frontmatter::parse_template_document;
use crate::resolver::canonicalize_with_roots;
use crate::validation::discover_tokens;
use crate::{ComposeError, ConfigError, DiagnosticCode, FrontmatterInitResult, VariableName};

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
    if !dry_run {
        let rewritten = format!("{frontmatter_text}{}", parsed.body());
        std::fs::write(&canonical, rewritten).map_err(|error| {
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
        discovered_variables: discovered,
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{ComposeError, VariableName, frontmatter_init};

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
