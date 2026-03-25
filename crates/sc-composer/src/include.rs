//! Recursive include expansion and confinement enforcement.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::DiagnosticCode;
use crate::error::{ComposeError, IncludeError};
use crate::frontmatter::{Frontmatter, parse_template_document};
use crate::types::{ComposePolicy, ConfiningRoot};

/// Expanded include graph returned from the include engine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExpandedTemplate {
    /// Final text with all includes expanded in deterministic order.
    pub text: String,
    /// Files visited during include expansion, in first-seen order.
    pub resolved_files: Vec<PathBuf>,
    /// Parsed frontmatter values keyed by the file they came from.
    pub frontmatters: Vec<(PathBuf, Option<Frontmatter>)>,
    /// Include chain recorded for each resolved file.
    pub include_chains: BTreeMap<PathBuf, Vec<PathBuf>>,
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
    let template_path = canonicalize_include(
        template_path.as_ref(),
        root.as_path(),
        &policy.allowed_roots,
        &[],
    )?;

    let mut state = ExpansionState::default();
    let text = expand_file(
        &template_path,
        root.as_path(),
        &policy.allowed_roots,
        policy.max_include_depth.get(),
        0,
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
    frontmatters: Vec<(PathBuf, Option<Frontmatter>)>,
    include_chains: BTreeMap<PathBuf, Vec<PathBuf>>,
}

fn expand_file(
    path: &Path,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
    max_depth: u16,
    depth: u16,
    stack: &mut Vec<PathBuf>,
    state: &mut ExpansionState,
) -> Result<String, ComposeError> {
    if depth > max_depth {
        return Err(IncludeError::new(
            DiagnosticCode::ErrIncludeDepth,
            format!("include depth exceeded maximum of {max_depth}"),
            stack.clone(),
        )
        .into());
    }
    if stack.contains(&path.to_path_buf()) {
        let mut cycle_stack = stack.clone();
        cycle_stack.push(path.to_path_buf());
        return Err(IncludeError::new(
            DiagnosticCode::ErrIncludeCycle,
            format!("include cycle detected at {}", path.display()),
            cycle_stack,
        )
        .into());
    }

    stack.push(path.to_path_buf());

    if state.resolved_seen.insert(path.to_path_buf()) {
        state.resolved_files.push(path.to_path_buf());
    }
    state
        .include_chains
        .entry(path.to_path_buf())
        .or_insert_with(|| {
            let mut chain = stack.clone();
            chain.push(path.to_path_buf());
            chain
        });

    let raw = std::fs::read_to_string(path).map_err(|error| {
        IncludeError::new(
            DiagnosticCode::ErrIncludeNotFound,
            format!("include file not found: {}", path.display()),
            stack.clone(),
        )
        .with_source(error)
    })?;
    let parsed = parse_template_document(&raw).map_err(|error| match error {
        ComposeError::Config(error) => error.into(),
        other => other,
    })?;
    state
        .frontmatters
        .push((path.to_path_buf(), parsed.frontmatter().cloned()));

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
                depth + 1,
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
    let canonical = std::fs::canonicalize(candidate).map_err(|error| {
        IncludeError::new(
            DiagnosticCode::ErrIncludeNotFound,
            format!("include file not found: {}", candidate.display()),
            stack.to_vec(),
        )
        .with_source(error)
    })?;

    let mut allowed = Vec::with_capacity(allowed_roots.len() + 1);
    allowed.push(root.to_path_buf());
    allowed.extend(
        allowed_roots
            .iter()
            .map(|root| root.as_path().to_path_buf()),
    );

    if allowed
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
                assert_eq!(error.code(), Some(DiagnosticCode::ErrIncludeNotFound));
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
                assert_eq!(error.code(), Some(DiagnosticCode::ErrIncludeCycle));
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
                assert_eq!(error.code(), Some(DiagnosticCode::ErrIncludeDepth));
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
                assert_eq!(error.code(), Some(DiagnosticCode::ErrIncludeEscape));
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
                assert_eq!(error.code(), Some(DiagnosticCode::ErrIncludeEscape));
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
