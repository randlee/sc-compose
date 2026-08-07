//! Recursive include expansion and confinement enforcement.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::ComposeError;
use crate::frontmatter::Frontmatter;
use crate::types::{ComposePolicy, ConfiningRoot};

mod directive;
mod expansion;
mod path;

/// Expanded include graph returned from the include engine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExpandedTemplate {
    /// Final text with all includes expanded in deterministic order.
    pub text: String,
    /// Files visited during include expansion, in first-seen order.
    pub resolved_files: Vec<PathBuf>,
    /// Parsed frontmatter values keyed by the file they came from.
    pub frontmatters: Vec<(PathBuf, Vec<Frontmatter>)>,
    /// Include chain recorded for each resolved file.
    pub include_chains: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// Raw source text keyed by each file visited during include expansion.
    pub source_texts: BTreeMap<PathBuf, String>,
}

/// Expand `@<path>` directives starting from the provided template path.
///
/// # Errors
///
/// Returns [`ComposeError`] when include resolution fails, when the include
/// graph exceeds the configured depth, or when an include escapes the allowed
/// roots.
pub fn expand_includes(
    template_path: impl AsRef<Path>,
    root: &ConfiningRoot,
    policy: &ComposePolicy,
) -> Result<ExpandedTemplate, ComposeError> {
    let (text, state) = expansion::expand(template_path, root, policy)?;

    Ok(ExpandedTemplate {
        text,
        resolved_files: state.resolved_files,
        frontmatters: state.frontmatters,
        include_chains: state.include_chains,
        source_texts: state.source_texts,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::types::{ComposePolicy, ConfiningRoot, IncludeDepth};
    use crate::{ComposeError, DiagnosticCode};

    use super::expand_includes;

    #[test]
    fn expands_successful_include_chain() {
        let root = temp_root("include_success");
        write_file(&root.join("root.md.j2"), "top\n@<partials/one.md>\n");
        write_file(&root.join("partials/one.md"), "middle\n@<two.md>\n");
        write_file(&root.join("partials/two.md"), "bottom\n");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert!(expanded.text.contains("top"));
        assert!(expanded.text.contains("middle"));
        assert!(expanded.text.contains("bottom"));
        assert_eq!(expanded.resolved_files.len(), 3);
    }

    #[test]
    fn duplicate_includes_reuse_cached_source_and_preserve_graph_order() {
        let root = temp_root("include_duplicate");
        write_file(&root.join("root.md.j2"), "@<child.md>\n@<child.md>\n");
        write_file(&root.join("child.md"), "child\n");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert_eq!(expanded.text, "child\nchild\n");
        assert_eq!(expanded.resolved_files.len(), 2);
        assert_eq!(expanded.source_texts.len(), 2);
        assert_eq!(expanded.include_chains.len(), 2);
        assert_eq!(expanded.frontmatters.len(), 3);
    }

    #[test]
    fn captures_frontmatter_and_preserves_custom_delimiter_text() {
        let root = temp_root("include_frontmatter");
        write_file(
            &root.join("root.md.j2"),
            "---\ndefaults:\n  greeting: hello\n---\n@<child.md>\n",
        );
        write_file(&root.join("child.md"), "[[ greeting ]]\n");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert_eq!(expanded.frontmatters.len(), 2);
        assert_eq!(expanded.frontmatters[0].1.len(), 1);
        assert_eq!(expanded.text, "[[ greeting ]]\n");
    }

    #[test]
    fn missing_include_reports_not_found() {
        let root = temp_root("include_missing");
        write_file(&root.join("root.md.j2"), "@<missing.md>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeNotFound);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn directory_include_target_reports_is_a_directory() {
        let root = temp_root("include_directory_target");
        write_file(&root.join("root.md.j2"), "@<partials>\n");
        fs::create_dir(root.join("partials")).unwrap();

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeIsADirectory);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn permission_denied_include_reports_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("include_permission_denied");
        let restricted = root.join("restricted.md");
        write_file(&root.join("root.md.j2"), "@<restricted.md>\n");
        write_file(&restricted, "secret\n");
        let mut permissions = fs::metadata(&restricted).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&restricted, permissions).unwrap();

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        let mut restore = fs::metadata(&restricted).unwrap().permissions();
        restore.set_mode(0o600);
        fs::set_permissions(&restricted, restore).unwrap();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludePermissionDenied);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlink_loop_include_reports_filesystem_loop() {
        use std::os::unix::fs::symlink;

        let root = temp_root("include_filesystem_loop");
        write_file(&root.join("root.md.j2"), "@<loop-a.md>\n");
        symlink("loop-b.md", root.join("loop-a.md")).unwrap();
        symlink("loop-a.md", root.join("loop-b.md")).unwrap();

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeFilesystemLoop);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn cycle_detection_is_rejected() {
        let root = temp_root("include_cycle");
        write_file(&root.join("root.md.j2"), "@<one.md>\n");
        write_file(&root.join("one.md"), "@<root.md.j2>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeCycle);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn depth_overflow_is_rejected() {
        let root = temp_root("include_depth");
        write_file(&root.join("root.md.j2"), "@<a.md>\n");
        write_file(&root.join("a.md"), "@<b.md>\n");
        write_file(&root.join("b.md"), "@<c.md>\n");
        write_file(&root.join("c.md"), "done\n");

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(1),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn escape_attempts_are_rejected() {
        let root = temp_root("include_escape");
        write_file(&root.join("root.md.j2"), "@<../outside.md>\n");
        let outside = root.parent().unwrap().join("outside.md");
        write_file(&outside, "nope\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn nonexistent_escape_attempts_are_rejected_before_not_found() {
        let root = temp_root("include_escape_missing");
        write_file(&root.join("root.md.j2"), "@<../outside-missing.md>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn single_line_include_expands_exactly_once() {
        let root = temp_root("include_single_line");
        write_file(&root.join("root.md.j2"), "@<child.md>");
        write_file(&root.join("child.md"), "child body");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert_eq!(expanded.text, "child body");
    }

    #[test]
    fn absolute_escape_attempts_are_rejected() {
        let root = temp_root("include_absolute_escape");
        let outside = root.parent().unwrap().join("absolute-outside.md");
        write_file(
            &root.join("root.md.j2"),
            &format!("@<{}>\n", outside.display()),
        );
        write_file(&outside, "outside\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn symlink_escape_attempts_are_rejected_when_supported() {
        let root = temp_root("include_symlink_escape");
        let outside = root.parent().unwrap().join("symlink-outside.md");
        write_file(&outside, "outside\n");
        let symlink_path = root.join("linked-outside.md");
        if !create_symlink_if_supported(&outside, &symlink_path) {
            return;
        }
        write_file(&root.join("root.md.j2"), "@<linked-outside.md>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn sibling_prefix_escape_attempts_are_rejected() {
        let root = temp_root("include_sibling_prefix");
        let sibling = root.parent().unwrap().join(format!(
            "{}-evil",
            root.file_name().unwrap().to_string_lossy()
        ));
        write_file(&sibling.join("outside.md"), "outside\n");
        write_file(
            &root.join("root.md.j2"),
            &format!(
                "@<../{}/outside.md>\n",
                sibling.file_name().unwrap().to_string_lossy()
            ),
        );

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
                assert!(
                    error
                        .message()
                        .contains("include path escapes confinement root")
                );
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn configured_allowed_root_remains_available_to_includes() {
        let root = temp_root("include_allowed_root");
        let allowed = temp_root("include_allowed_root_shared");
        write_file(
            &root.join("root.md.j2"),
            &format!(
                "@<../{}/part.md>\n",
                allowed.file_name().unwrap().to_string_lossy()
            ),
        );
        write_file(&allowed.join("part.md"), "shared\n");

        let policy = ComposePolicy {
            allowed_roots: vec![ConfiningRoot::new(&allowed).unwrap()],
            ..ComposePolicy::default()
        };
        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap();

        assert_eq!(expanded.text, "shared\n");
    }

    #[test]
    fn deep_include_chain_above_safe_ceiling_returns_include_depth_error() {
        const CHAIN_DEPTH: usize = 1_900;
        let root = temp_root("include_stack_overflow");
        let chain_root = root.join("chain");
        fs::create_dir_all(&chain_root).unwrap();
        write_file(&root.join("root.md.j2"), "@<chain/0000.md>\n");

        for index in 0..CHAIN_DEPTH {
            let current = chain_root.join(format!("{index:04}.md"));
            let contents = if index + 1 == CHAIN_DEPTH {
                "done\n".to_owned()
            } else {
                format!("@<{:04}.md>\n", index + 1)
            };
            write_file(&current, &contents);
        }

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(1_905),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn include_chain_at_safe_depth_still_expands_with_high_configured_limit() {
        const CHAIN_DEPTH: usize = 50;
        let root = temp_root("include_safe_depth");
        write_linear_chain(&root, CHAIN_DEPTH);

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(1_000),
            ..ComposePolicy::default()
        };

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap();

        assert_eq!(expanded.text, "done\n");
    }

    #[test]
    fn configured_depth_below_safety_ceiling_remains_the_effective_bound() {
        const CHAIN_DEPTH: usize = 10;
        let root = temp_root("include_configured_depth");
        write_linear_chain(&root, CHAIN_DEPTH);

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(5),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn include_depth_exact_limit_succeeds_and_one_over_fails() {
        let exact_root = temp_root("include_depth_exact_limit");
        write_linear_chain(&exact_root, 3);
        let exact_policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(3),
            ..ComposePolicy::default()
        };

        let expanded = expand_includes(
            exact_root.join("root.md.j2"),
            &ConfiningRoot::new(&exact_root).unwrap(),
            &exact_policy,
        )
        .unwrap();
        assert_eq!(expanded.text, "done\n");

        let over_root = temp_root("include_depth_one_over");
        write_linear_chain(&over_root, 4);
        let over_policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(3),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            over_root.join("root.md.j2"),
            &ConfiningRoot::new(&over_root).unwrap(),
            &over_policy,
        )
        .unwrap_err();
        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
                assert_eq!(error.message(), "include depth exceeded maximum of 3");
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

    fn write_linear_chain(root: &Path, depth: usize) {
        let chain_root = root.join("chain");
        fs::create_dir_all(&chain_root).unwrap();
        write_file(&root.join("root.md.j2"), "@<chain/0000.md>\n");

        for index in 0..depth {
            let current = chain_root.join(format!("{index:04}.md"));
            let contents = if index + 1 == depth {
                "done\n".to_owned()
            } else {
                format!("@<{:04}.md>\n", index + 1)
            };
            write_file(&current, &contents);
        }
    }

    #[cfg(unix)]
    fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(windows)]
    fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("failed to create symlink: {error}"),
        }
    }
}
