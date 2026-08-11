//! Construction of the caller-owned sc-sha composition manifest.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sc_sha::{
    CanonicalSource, CanonicalTemplatePath, CompositionSha256, ManifestSchemaVersion,
    ResolvedTemplateManifest, ResolvedTemplateNode, TemplateSha256, calculate_composition_hash,
};

use super::expansion::ExpansionState;
use crate::error::{ComposeError, IncludeError};
use crate::{ConfiningRoot, DiagnosticCode};

/// Source-composition identity plus the evidence used to calculate it.
#[derive(Clone, Debug, PartialEq)]
pub struct CompositionFingerprint {
    /// The structural hash returned by `sc_sha`.
    pub source_sha: CompositionSha256,
    /// Ordered, deduplicated source nodes and ordered include occurrences.
    pub manifest: ResolvedTemplateManifest,
    /// Files visited in deterministic first-discovery order.
    pub resolved_files: Vec<PathBuf>,
}

impl ExpansionState {
    pub(super) fn composition_fingerprint(&self) -> Result<CompositionFingerprint, ComposeError> {
        let manifest = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
        };
        let source_sha = calculate_composition_hash(&manifest).map_err(|error| {
            ComposeError::from(IncludeError::new(
                DiagnosticCode::ErrConfigRead,
                format!("resolved include manifest is invalid: {error}"),
                self.active_chain.clone(),
            ))
        })?;

        Ok(CompositionFingerprint {
            source_sha,
            manifest,
            resolved_files: self
                .resolved_files
                .iter()
                .map(Clone::clone)
                .collect::<Vec<_>>(),
        })
    }
}

/// Convert a confined canonical path to a host-independent manifest source.
pub(super) fn canonical_source(
    path: &Path,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
) -> Result<CanonicalSource, IncludeError> {
    let relative = if let Ok(relative) = path.strip_prefix(root) {
        relative.to_path_buf()
    } else if let Some((index, allowed)) = allowed_roots
        .iter()
        .enumerate()
        .find(|(_, allowed)| path.starts_with(allowed.as_path()))
    {
        let relative = path.strip_prefix(allowed.as_path()).map_err(|_error| {
            IncludeError::new(
                DiagnosticCode::ErrConfigRead,
                format!("cannot derive a canonical source for {}", path.display()),
                Vec::new(),
            )
        })?;
        let mut prefixed = PathBuf::from(format!("allowed-root-{index}"));
        prefixed.push(relative);
        prefixed
    } else {
        return Err(IncludeError::new(
            DiagnosticCode::ErrIncludeEscape,
            format!(
                "include path is outside the configured source roots: {}",
                path.display()
            ),
            Vec::new(),
        ));
    };

    let value = crate::to_forward_slash(&relative);
    CanonicalTemplatePath::try_from(value)
        .map(CanonicalSource::LocalPath)
        .map_err(|error| {
            IncludeError::new(
                DiagnosticCode::ErrConfigRead,
                format!("invalid canonical include source: {error}"),
                Vec::new(),
            )
        })
}

/// Insert a node only once and return the source identity used by edges.
pub(super) fn add_node(
    nodes: &mut Vec<ResolvedTemplateNode>,
    seen: &mut BTreeMap<CanonicalSource, TemplateSha256>,
    source: CanonicalSource,
    content_hash: TemplateSha256,
) {
    if seen.insert(source.clone(), content_hash).is_none() {
        nodes.push(ResolvedTemplateNode {
            source,
            content_hash,
        });
    }
}

/// Return the next ordered occurrence number for an including source.
pub(super) fn next_occurrence(
    counts: &mut BTreeMap<CanonicalSource, u32>,
    parent: &CanonicalSource,
) -> u32 {
    let occurrence = counts.entry(parent.clone()).or_default();
    let current = *occurrence;
    *occurrence = occurrence.saturating_add(1);
    current
}

impl Default for CompositionFingerprint {
    fn default() -> Self {
        let manifest = ResolvedTemplateManifest {
            schema: ManifestSchemaVersion::V1,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        Self {
            source_sha: calculate_composition_hash(&manifest)
                .expect("empty v1 manifest is structurally valid"),
            manifest,
            resolved_files: Vec::new(),
        }
    }
}
