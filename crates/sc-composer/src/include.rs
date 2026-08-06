//! Recursive include expansion and confinement enforcement.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::DiagnosticCode;
use crate::error::{ComposeError, IncludeError};
use crate::frontmatter::{Frontmatter, parse_template_document};
use crate::types::{ComposePolicy, ConfiningRoot, IncludeDepth};

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
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct CurrentIncludeDepth(u16);

impl CurrentIncludeDepth {
    const fn root() -> Self {
        Self(0)
    }

    const fn get(self) -> u16 {
        self.0
    }

    const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Hard ceiling for native include-chain recursion, independent of caller
/// configuration, so depth failures return diagnostics before stack overflow.
const MAX_SAFE_INCLUDE_DEPTH: u16 = 128;

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
    let template_path = canonicalize_include(
        template_path.as_ref(),
        root.as_path(),
        &policy.allowed_roots,
        &[],
    )?;

    let effective_max_depth =
        IncludeDepth::new(policy.max_include_depth.get().min(MAX_SAFE_INCLUDE_DEPTH));

    let mut state = ExpansionState::default();
    let text = expand_file(
        &template_path,
        root.as_path(),
        &policy.allowed_roots,
        effective_max_depth,
        CurrentIncludeDepth::root(),
        &mut Vec::new(),
        &mut state,
    )?;

    Ok(ExpandedTemplate {
        text,
        resolved_files: state.resolved_files,
        frontmatters: state.frontmatters,
        include_chains: state.include_chains,
    })
}

#[derive(Default)]
struct ExpansionState {
    resolved_files: Vec<PathBuf>,
    resolved_seen: BTreeSet<PathBuf>,
    frontmatters: Vec<(PathBuf, Vec<Frontmatter>)>,
    include_chains: BTreeMap<PathBuf, Vec<PathBuf>>,
}

fn expand_file(
    path: &Path,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
    max_depth: IncludeDepth,
    depth: CurrentIncludeDepth,
    stack: &mut Vec<PathBuf>,
    state: &mut ExpansionState,
) -> Result<String, ComposeError> {
    let path_buf = path.to_path_buf();
    if depth.get() > max_depth.get() {
        return Err(IncludeError::new(
            DiagnosticCode::ErrIncludeDepth,
            format!("include depth exceeded maximum of {}", max_depth.get()),
            stack.clone(),
        )
        .into());
    }
    if stack.iter().any(|existing| existing == &path_buf) {
        let mut cycle_stack = stack.clone();
        cycle_stack.push(path_buf.clone());
        return Err(IncludeError::new(
            DiagnosticCode::ErrIncludeCycle,
            format!("include cycle detected at {}", path.display()),
            cycle_stack,
        )
        .into());
    }

    let is_new = !state.resolved_seen.contains(&path_buf);
    stack.push(path_buf.clone());

    if is_new {
        state.resolved_files.push(path_buf.clone());
        state.include_chains.insert(path_buf.clone(), stack.clone());
    }

    let raw = std::fs::read_to_string(path).map_err(|error| {
        let (code, message) = if error.kind() == std::io::ErrorKind::InvalidData {
            (
                DiagnosticCode::ErrConfigRead,
                format!("template file is not valid UTF-8: {}", path.display()),
            )
        } else {
            (
                DiagnosticCode::ErrIncludeNotFound,
                format!("include file not found: {}", path.display()),
            )
        };
        IncludeError::new(code, message, stack.clone()).with_source(error)
    })?;
    let parsed = parse_template_document(&raw).map_err(|error| match error {
        ComposeError::Config(error) => error.into(),
        other => other,
    })?;
    state
        .frontmatters
        .push((path.to_path_buf(), parsed.passes().to_vec()));
    if is_new {
        state.resolved_seen.insert(path_buf);
    }

    let mut expanded = String::new();
    for line in parsed.body().split_inclusive('\n') {
        if let Some(include_target) = parse_include_directive(line) {
            let resolved_include =
                resolve_include_path(include_target, path, root, allowed_roots, stack)?;
            let nested = expand_file(
                &resolved_include,
                root,
                allowed_roots,
                max_depth,
                depth.next(),
                stack,
                state,
            )?;
            expanded.push_str(&nested);
        } else {
            expanded.push_str(line);
        }
    }

    stack.pop();
    Ok(expanded)
}

fn resolve_include_path(
    include_target: &str,
    containing_file: &Path,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
    stack: &[PathBuf],
) -> Result<PathBuf, ComposeError> {
    let relative_candidate = containing_file
        .parent()
        .unwrap_or(root)
        .join(include_target);
    if let Ok(path) = canonicalize_include(&relative_candidate, root, allowed_roots, stack) {
        return Ok(path);
    }

    let root_candidate = root.join(include_target);
    canonicalize_include(&root_candidate, root, allowed_roots, stack)
}

fn canonicalize_include(
    candidate: &Path,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
    stack: &[PathBuf],
) -> Result<PathBuf, ComposeError> {
    let mut allowed_canonical = Vec::with_capacity(allowed_roots.len() + 1);
    allowed_canonical.push(root.to_path_buf());
    allowed_canonical.extend(
        allowed_roots
            .iter()
            .map(|allowed_root| allowed_root.as_path().to_path_buf()),
    );

    match std::fs::canonicalize(candidate) {
        Ok(canonical) => {
            if allowed_canonical
                .iter()
                .any(|allowed_root| canonical.starts_with(allowed_root))
            {
                Ok(canonical)
            } else {
                Err(IncludeError::new(
                    DiagnosticCode::ErrIncludeEscape,
                    format!(
                        "include path escapes confinement root: {}",
                        candidate.display()
                    ),
                    stack.to_vec(),
                )
                .into())
            }
        }
        Err(error) => {
            let normalized_candidate = normalize_path(candidate);
            let allowed_normalized = allowed_canonical
                .iter()
                .map(|allowed_root| normalize_path(allowed_root))
                .collect::<Vec<_>>();

            if !allowed_normalized
                .iter()
                .any(|allowed_root| normalized_candidate.starts_with(allowed_root))
            {
                return Err(IncludeError::new(
                    DiagnosticCode::ErrIncludeEscape,
                    format!(
                        "include path escapes confinement root: {}",
                        candidate.display()
                    ),
                    stack.to_vec(),
                )
                .into());
            }

            Err(IncludeError::new(
                DiagnosticCode::ErrIncludeNotFound,
                format!("include file not found: {}", candidate.display()),
                stack.to_vec(),
            )
            .with_source(error)
            .into())
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}

fn parse_include_directive(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    (trimmed.starts_with("@<") && trimmed.ends_with('>') && trimmed.len() > 3)
        .then(|| &trimmed[2..trimmed.len() - 1])
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
    #[ignore = "red baseline: fails until directory targets receive a distinct diagnostic"]
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
                assert_eq!(error.code().as_str(), "ERR_INCLUDE_IS_A_DIRECTORY");
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
