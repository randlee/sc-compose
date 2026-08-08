use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::directive::parse_include_directive;
use super::path::{canonicalize_include, resolve_include_path};
use crate::DiagnosticCode;
use crate::error::{ComposeError, IncludeError};
use crate::frontmatter::parse_template_document;
use crate::types::{ComposePolicy, ConfiningRoot, IncludeDepth};

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

#[derive(Default)]
pub(super) struct ExpansionState {
    pub(super) resolved_files: Vec<PathBuf>,
    resolved_seen: BTreeSet<PathBuf>,
    pub(super) frontmatters: Vec<(PathBuf, Vec<crate::frontmatter::Frontmatter>)>,
    pub(super) include_chains: BTreeMap<PathBuf, Vec<PathBuf>>,
    pub(super) source_texts: BTreeMap<PathBuf, String>,
}

pub(super) fn expand(
    template_path: impl AsRef<Path>,
    root: &ConfiningRoot,
    policy: &ComposePolicy,
) -> Result<(String, ExpansionState), ComposeError> {
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

    Ok((text, state))
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
        let (code, message) = match crate::diagnostics::classify_filesystem_error(path, &error) {
            crate::diagnostics::FilesystemErrorClass::InvalidData => (
                DiagnosticCode::ErrConfigRead,
                format!("template file is not valid UTF-8: {}", path.display()),
            ),
            crate::diagnostics::FilesystemErrorClass::PermissionDenied => (
                DiagnosticCode::ErrIncludePermissionDenied,
                format!("permission denied reading include file: {}", path.display()),
            ),
            crate::diagnostics::FilesystemErrorClass::IsADirectory => (
                DiagnosticCode::ErrIncludeIsADirectory,
                format!(
                    "include target is a directory, not a file: {}",
                    path.display()
                ),
            ),
            crate::diagnostics::FilesystemErrorClass::FilesystemLoop => (
                DiagnosticCode::ErrIncludeFilesystemLoop,
                format!(
                    "include target is a filesystem symlink loop: {}",
                    path.display()
                ),
            ),
            crate::diagnostics::FilesystemErrorClass::NotFound => (
                DiagnosticCode::ErrIncludeNotFound,
                format!("include file not found: {}", path.display()),
            ),
        };
        IncludeError::new(code, message, stack.clone()).with_source(error)
    })?;
    let parsed = parse_template_document(&raw).map_err(|error| match error {
        ComposeError::Config(error) => error.into(),
        other => other,
    })?;
    if is_new {
        state.source_texts.insert(path.to_path_buf(), raw.clone());
    }
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
