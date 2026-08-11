use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::directive::{IncludeDirective, parse_include_directive};
use super::fingerprint::{add_node, canonical_source, next_occurrence};
use super::path::{canonicalize_include, resolve_include_path};
use crate::DiagnosticCode;
use crate::error::{ComposeError, IncludeError};
use crate::frontmatter::parse_template_document;
use crate::types::{ComposePolicy, ConfiningRoot, IncludeDepth};
use sc_sha::{CanonicalSource, HashInput, TemplateSha256, calculate_hash};

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
    pub(super) nodes: Vec<sc_sha::ResolvedTemplateNode>,
    pub(super) edges: Vec<sc_sha::ResolvedIncludeEdge>,
    source_hashes: BTreeMap<CanonicalSource, TemplateSha256>,
    occurrence_counts: BTreeMap<CanonicalSource, u32>,
    pub(super) active_chain: Vec<PathBuf>,
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
    state.active_chain.clone_from(stack);

    if is_new {
        state.resolved_files.push(path_buf.clone());
        state.include_chains.insert(path_buf.clone(), stack.clone());
    }

    let raw = read_and_hash_source(path, root, allowed_roots, stack, state, is_new)?;
    let parsed = parse_template_document(&raw).map_err(|error| match error {
        ComposeError::Config(error) => error.into(),
        other => other,
    })?;
    if is_new {
        state.source_texts.insert(path.to_path_buf(), raw.clone());
        state
            .frontmatters
            .push((path.to_path_buf(), parsed.passes().to_vec()));
    }
    if is_new {
        state.resolved_seen.insert(path_buf);
    }

    let mut context = ExpansionContext {
        root,
        allowed_roots,
        max_depth,
        depth,
        stack,
        state,
    };
    let expanded = context.expand_body(parsed.body(), path)?;

    stack.pop();
    state.active_chain.clone_from(stack);
    Ok(expanded)
}

struct ExpansionContext<'a> {
    root: &'a Path,
    allowed_roots: &'a [ConfiningRoot],
    max_depth: IncludeDepth,
    depth: CurrentIncludeDepth,
    stack: &'a mut Vec<PathBuf>,
    state: &'a mut ExpansionState,
}

impl ExpansionContext<'_> {
    fn expand_body(&mut self, body: &str, path: &Path) -> Result<String, ComposeError> {
        let mut expanded = String::new();
        for line in body.split_inclusive('\n') {
            match parse_include_directive(line) {
                Some(IncludeDirective::Static(target)) if is_static_target(&target) => {
                    expanded.push_str(&self.expand_candidate(&target, path)?);
                }
                Some(IncludeDirective::Conditional {
                    condition,
                    candidates,
                }) => {
                    let mut candidates = candidates.into_iter();
                    let first = self.expand_candidate(
                        &candidates
                            .next()
                            .expect("conditional include has a then candidate"),
                        path,
                    )?;
                    let second = self.expand_candidate(
                        &candidates
                            .next()
                            .expect("conditional include has an else candidate"),
                        path,
                    )?;
                    let _ = write!(
                        expanded,
                        "{{% if {condition} %}}{first}{{% else %}}{second}{{% endif %}}"
                    );
                }
                Some(IncludeDirective::Dynamic | IncludeDirective::Static(_)) | None
                    if line.trim_start().starts_with("@<") =>
                {
                    return Err(dynamic_include_error(self.stack));
                }
                _ => expanded.push_str(line),
            }
        }
        Ok(expanded)
    }

    fn expand_candidate(&mut self, target: &str, path: &Path) -> Result<String, ComposeError> {
        let resolved =
            resolve_include_path(target, path, self.root, self.allowed_roots, self.stack)?;
        let parent = canonical_source(path, self.root, self.allowed_roots)?;
        let child = canonical_source(&resolved, self.root, self.allowed_roots)?;
        let occurrence = next_occurrence(&mut self.state.occurrence_counts, &parent);
        self.state.edges.push(sc_sha::ResolvedIncludeEdge {
            parent,
            child,
            occurrence,
        });
        expand_file(
            &resolved,
            self.root,
            self.allowed_roots,
            self.max_depth,
            self.depth.next(),
            self.stack,
            self.state,
        )
    }
}

fn is_static_target(target: &str) -> bool {
    !target.contains("{{") && !target.contains("{%") && !target.contains("${")
}

fn dynamic_include_error(stack: &[PathBuf]) -> ComposeError {
    IncludeError::new(
        DiagnosticCode::ErrIncludeDynamicUnresolved,
        "include target is dynamic or malformed; enumerate a static @<path> candidate",
        stack.to_vec(),
    )
    .into()
}

fn classify_read_error(path: &Path, error: &std::io::Error) -> (DiagnosticCode, String) {
    match crate::diagnostics::classify_filesystem_error(path, error) {
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
    }
}

fn read_and_hash_source(
    path: &Path,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
    stack: &[PathBuf],
    state: &mut ExpansionState,
    is_new: bool,
) -> Result<String, ComposeError> {
    if !is_new {
        return state.source_texts.get(path).cloned().ok_or_else(|| {
            IncludeError::new(
                DiagnosticCode::ErrConfigRead,
                format!("cached include source is unavailable: {}", path.display()),
                stack.to_vec(),
            )
            .into()
        });
    }

    let raw_bytes = std::fs::read(path).map_err(|error| {
        let (code, message) = classify_read_error(path, &error);
        IncludeError::new(code, message, stack.to_vec()).with_source(error)
    })?;
    let source = canonical_source(path, root, allowed_roots)?;
    let hash = calculate_hash(HashInput::TextFileBytes {
        utf8_file_bytes: &raw_bytes,
    })
    .map_err(|error| {
        IncludeError::new(
            DiagnosticCode::ErrConfigRead,
            format!("failed to hash template {}: {error}", path.display()),
            stack.to_vec(),
        )
    })?
    .template()
    .to_owned();
    add_node(&mut state.nodes, &mut state.source_hashes, source, hash);
    String::from_utf8(raw_bytes).map_err(|error| {
        IncludeError::new(
            DiagnosticCode::ErrConfigRead,
            format!("template file is not valid UTF-8: {}", path.display()),
            stack.to_vec(),
        )
        .with_source(error)
        .into()
    })
}
