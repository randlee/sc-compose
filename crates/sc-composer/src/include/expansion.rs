use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

/// Include chain from the root template to one source file, inclusive.
pub(super) type SourceChain = Arc<[PathBuf]>;

/// Source file and one-based line that one line of expanded text came from.
#[derive(Clone, Debug)]
pub(super) struct LineOrigin {
    pub(super) chain: SourceChain,
    pub(super) line: usize,
    /// Characters of synthetic conditional-include text (`{% if %}`,
    /// `{% else %}`, `{% endif %}`) that precede the source text on this
    /// expanded line. Source columns are expanded columns minus this.
    pub(super) column_offset: usize,
    /// Directive line that wrote the synthetic prefix, when there is one.
    pub(super) directive: Option<(SourceChain, usize)>,
}

impl LineOrigin {
    fn new(chain: &SourceChain, line: usize) -> Self {
        Self {
            chain: Arc::clone(chain),
            line,
            column_offset: 0,
            directive: None,
        }
    }
}

/// Expanded text plus the origin of each of its lines.
///
/// `origins[k]` is the origin of the line that `text.split_inclusive('\n')`
/// yields at index `k`. When text from several sources shares one expanded
/// line, the line keeps the origin of the source that started it, except that
/// synthetic conditional-include text never owns a line that source text
/// continues: that line takes the origin of the source text, shifted by the
/// synthetic prefix.
#[derive(Debug, Default)]
pub(super) struct ExpandedText {
    pub(super) text: String,
    pub(super) origins: Vec<LineOrigin>,
    /// The last origin belongs to synthetic text on a line that is still open.
    provisional: bool,
}

impl ExpandedText {
    fn push_include(&mut self, child: Self, line: &str, chain: &SourceChain, line_number: usize) {
        let needs_terminator = !child.text.ends_with(['\n', '\r']);
        self.push_expanded(child);
        if needs_terminator {
            let terminator = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            self.push_source(terminator, chain, line_number);
        }
    }
    fn at_line_start(&self) -> bool {
        self.text.is_empty() || self.text.ends_with('\n')
    }

    /// Append text taken from one source, whose first line is `first_line`.
    fn push_source(&mut self, text: &str, chain: &SourceChain, first_line: usize) {
        for (offset, piece) in text.split_inclusive('\n').enumerate() {
            self.push_piece(piece, Some(LineOrigin::new(chain, first_line + offset)));
        }
    }

    /// Append already-expanded text, keeping the origins of its lines.
    fn push_expanded(&mut self, other: Self) {
        let mut origins = other.origins.into_iter();
        for piece in other.text.split_inclusive('\n') {
            self.push_piece(piece, origins.next());
        }
    }

    /// Append synthetic conditional-include text written for the directive
    /// at `line` of `chain`. It has no line break, so it either starts a line
    /// or continues one.
    fn push_synthetic(&mut self, text: &str, chain: &SourceChain, line: usize) {
        let width = text.chars().count();
        if self.at_line_start() {
            let mut origin = LineOrigin::new(chain, line);
            origin.column_offset = width;
            origin.directive = Some((Arc::clone(chain), line));
            self.origins.push(origin);
            self.provisional = true;
        } else if self.provisional
            && let Some(last) = self.origins.last_mut()
        {
            last.column_offset += width;
        }
        self.text.push_str(text);
    }

    /// Append one piece holding at most one line break, at its end.
    fn push_piece(&mut self, piece: &str, origin: Option<LineOrigin>) {
        if piece.is_empty() {
            return;
        }
        if self.at_line_start() {
            if let Some(origin) = origin {
                self.origins.push(origin);
            }
        } else if self.provisional
            && let (Some(origin), Some(last)) = (origin, self.origins.last_mut())
        {
            // Source text continues a line that synthetic text started.
            *last = LineOrigin {
                column_offset: last.column_offset + origin.column_offset,
                directive: last.directive.take(),
                ..origin
            };
        }
        self.provisional = false;
        self.text.push_str(piece);
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
) -> Result<(ExpandedText, ExpansionState), ComposeError> {
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
) -> Result<ExpandedText, ComposeError> {
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

    // File line of the first body line: the body follows the frontmatter.
    let body_first_line = raw
        .strip_suffix(parsed.body())
        .map_or(1, |head| head.matches('\n').count() + 1);
    let chain: SourceChain = stack.clone().into();
    let mut context = ExpansionContext {
        root,
        allowed_roots,
        max_depth,
        depth,
        stack,
        state,
    };
    let expanded = context.expand_body(parsed.body(), path, &chain, body_first_line)?;

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
    fn expand_body(
        &mut self,
        body: &str,
        path: &Path,
        chain: &SourceChain,
        body_first_line: usize,
    ) -> Result<ExpandedText, ComposeError> {
        let mut expanded = ExpandedText::default();
        for (index, line) in body.split_inclusive('\n').enumerate() {
            let line_number = body_first_line + index;
            match parse_include_directive(line) {
                Some(IncludeDirective::Static(target)) if is_static_target(&target) => {
                    expanded.push_include(
                        self.expand_candidate(&target, path)?,
                        line,
                        chain,
                        line_number,
                    );
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
                    expanded.push_synthetic(&format!("{{% if {condition} %}}"), chain, line_number);
                    expanded.push_include(first, line, chain, line_number);
                    expanded.push_synthetic("{% else %}", chain, line_number);
                    expanded.push_include(second, line, chain, line_number);
                    expanded.push_synthetic("{% endif %}", chain, line_number);
                }
                Some(IncludeDirective::Dynamic | IncludeDirective::Static(_)) | None
                    if line.trim_start().starts_with("@<") =>
                {
                    return Err(dynamic_include_error(self.stack));
                }
                _ => expanded.push_source(line, chain, line_number),
            }
        }
        Ok(expanded)
    }

    fn expand_candidate(
        &mut self,
        target: &str,
        path: &Path,
    ) -> Result<ExpandedText, ComposeError> {
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
