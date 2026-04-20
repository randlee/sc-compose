//! Workspace bootstrap helper.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::types::{ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot};
use crate::{ComposeError, ConfigError, Diagnostic, DiagnosticCode, InitResult};

/// Bootstrap the repository workspace for composed outputs.
///
/// # Errors
///
/// Returns [`ComposeError`] when filesystem updates fail.
pub fn init_workspace(root: impl AsRef<Path>, dry_run: bool) -> Result<InitResult, ComposeError> {
    let root = root.as_ref();
    let canonical_root = std::fs::canonicalize(root).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!("failed to canonicalize workspace root: {}", root.display()),
        )
        .with_source(error)
    })?;
    let prompts_dir = canonical_root.join(".prompts");
    let gitignore = canonical_root.join(".gitignore");
    let templates = scan_templates(&canonical_root)?;

    let prompts_dir_missing = !prompts_dir.exists();
    let mut current_gitignore = std::fs::read_to_string(&gitignore).unwrap_or_default();
    let gitignore_updated = !current_gitignore
        .lines()
        .any(|line| line.trim() == ".prompts/");

    if !dry_run && !prompts_dir_missing && !gitignore_updated {
        return Err(ConfigError::new(
            DiagnosticCode::ErrConfigReadonly,
            "workspace already initialized; rerun with --dry-run to inspect planned changes",
        )
        .into());
    }

    if !dry_run && prompts_dir_missing {
        std::fs::create_dir_all(&prompts_dir).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigReadonly,
                format!("failed to create prompts dir: {}", prompts_dir.display()),
            )
            .with_source(error)
        })?;
    }

    if !dry_run && gitignore_updated {
        if !current_gitignore.is_empty() && !current_gitignore.ends_with('\n') {
            current_gitignore.push('\n');
        }
        current_gitignore.push_str(".prompts/\n");
        std::fs::write(&gitignore, current_gitignore).map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigReadonly,
                format!("failed to update gitignore: {}", gitignore.display()),
            )
            .with_source(error)
        })?;
    }

    let mut recommendations = Vec::new();
    let mut validation_passed = true;
    for template in &templates {
        let request = ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: template
                    .strip_prefix(&canonical_root)
                    .unwrap_or(template)
                    .to_path_buf(),
            },
            root: ConfiningRoot::from_path_buf(canonical_root.clone()),
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            vars_defaults: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        };
        match crate::validate(&request) {
            Ok(report) => {
                validation_passed &= report.ok;
                recommendations.extend(report.warnings);
            }
            Err(error) => {
                validation_passed = false;
                recommendations.push(Diagnostic::new(
                    crate::DiagnosticSeverity::Error,
                    error.code().unwrap_or(DiagnosticCode::ErrConfigParse),
                    error.to_string(),
                ));
            }
        }
    }

    Ok(InitResult {
        prompts_dir,
        gitignore_updated,
        scanned_templates: templates,
        recommendations,
        validation_passed,
    })
}

fn scan_templates(root: &Path) -> Result<Vec<PathBuf>, ComposeError> {
    let mut templates = Vec::new();
    scan_templates_recursive(root, root, &mut templates)?;
    Ok(templates)
}

fn scan_templates_recursive(
    root: &Path,
    current: &Path,
    templates: &mut Vec<PathBuf>,
) -> Result<(), ComposeError> {
    for entry in std::fs::read_dir(current).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!("failed to read directory: {}", current.display()),
        )
        .with_source(error)
    })? {
        let entry = entry.map_err(|error| {
            ConfigError::new(
                DiagnosticCode::ErrConfigParse,
                format!("failed to read directory entry in {}", current.display()),
            )
            .with_source(error)
        })?;
        let path = entry.path();
        if path == root.join(".git") || path == root.join("target") {
            continue;
        }
        if path.is_dir() {
            scan_templates_recursive(root, &path, templates)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("j2"))
            })
        {
            templates.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::init_workspace;
    use crate::{ComposeError, DiagnosticCode};

    #[test]
    fn dry_run_scans_templates_without_modifying_workspace() {
        let root = temp_root("init_workspace_dry_run");
        write_file(&root.join("templates/example.md.j2"), "hello {{ name }}\n");

        let result = init_workspace(&root, true).unwrap();

        assert_eq!(result.scanned_templates.len(), 1);
        assert!(!root.join(".prompts").exists());
        assert!(!root.join(".gitignore").exists());
    }

    #[test]
    fn init_creates_prompts_dir_and_updates_gitignore() {
        let root = temp_root("init_workspace_write");
        write_file(&root.join("templates/example.md.j2"), "hello {{ name }}\n");

        let result = init_workspace(&root, false).unwrap();

        assert!(result.prompts_dir.exists());
        assert!(result.gitignore_updated);
        assert!(
            fs::read_to_string(root.join(".gitignore"))
                .unwrap()
                .contains(".prompts/")
        );
    }

    #[test]
    fn reinit_without_changes_reports_readonly_error() {
        let root = temp_root("init_workspace_reinit");
        fs::create_dir_all(root.join(".prompts")).unwrap();
        write_file(&root.join(".gitignore"), ".prompts/\n");

        let error = init_workspace(&root, false).unwrap_err();

        match error {
            ComposeError::Config(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrConfigReadonly);
            }
            other => panic!("unexpected error: {other}"),
        }
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
