//! Shared path canonicalization and confinement checks.

use std::io;
use std::path::{Component, Path, PathBuf};

use crate::types::ConfiningRoot;

/// Result of canonicalizing a candidate path while preserving a filesystem
/// error for the caller to classify.
#[derive(Debug)]
pub(crate) enum Canonicalization {
    /// The candidate exists and is contained by one of the approved roots.
    Existing(PathBuf),
    /// The candidate could not be canonicalized, but its normalized path is
    /// contained by an approved root.
    Missing {
        candidate: PathBuf,
        source: io::Error,
    },
}

/// Failure raised when a candidate is outside every approved root.
#[derive(Debug)]
pub(crate) struct ContainmentEscape {
    /// Candidate path used for the containment check.
    pub(crate) candidate: PathBuf,
}

/// Canonicalize a candidate and enforce component-aware root containment.
///
/// Existing candidates are checked using their canonical path, which follows
/// symlinks. Missing candidates are checked using normalized lexical paths so
/// callers can preserve their existing not-found or filesystem diagnostics.
/// `ConfiningRoot` values are already canonical by contract; the root and
/// allowed-root list are therefore used as the shared containment boundary.
pub(crate) fn canonicalize_within_roots(
    candidate: impl AsRef<Path>,
    root: &ConfiningRoot,
    allowed_roots: &[ConfiningRoot],
) -> Result<Canonicalization, ContainmentEscape> {
    let candidate = candidate.as_ref();
    let candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.as_path().join(candidate)
    };

    let mut approved_roots = Vec::with_capacity(allowed_roots.len() + 1);
    approved_roots.push(root.as_path().to_path_buf());
    approved_roots.extend(
        allowed_roots
            .iter()
            .map(|allowed_root| allowed_root.as_path().to_path_buf()),
    );

    match std::fs::canonicalize(&candidate) {
        Ok(canonical) if is_within_any(&canonical, &approved_roots) => {
            Ok(Canonicalization::Existing(canonical))
        }
        Ok(_) => Err(ContainmentEscape { candidate }),
        Err(_source) if !is_within_any(&normalize_path(&candidate), &approved_roots) => {
            Err(ContainmentEscape { candidate })
        }
        Err(source) => Ok(Canonicalization::Missing { candidate, source }),
    }
}

fn is_within_any(candidate: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| candidate.starts_with(root))
}

fn normalize_path(path: &Path) -> PathBuf {
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
