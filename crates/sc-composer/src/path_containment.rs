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
/// Canonical roots and caller-preserved lexical aliases are both accepted as
/// containment boundaries; missing-path checks normalize both candidates and
/// roots before comparing them.
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
        Err(source) => {
            let normalized_candidate = normalize_path(&candidate);
            let normalized_roots = approved_roots
                .iter()
                .map(|root| normalize_path(root))
                .collect::<Vec<_>>();
            if !is_within_any(&normalized_candidate, &normalized_roots) {
                return Err(ContainmentEscape { candidate });
            }
            Ok(Canonicalization::Missing { candidate, source })
        }
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{Canonicalization, canonicalize_within_roots};
    use crate::types::ConfiningRoot;

    #[test]
    fn missing_paths_compare_normalized_approved_roots() {
        let base = std::env::temp_dir().join(format!(
            "sc-compose-path-containment-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let primary = base.join("primary");
        let allowed = base.join("allowed");
        fs::create_dir_all(&primary).unwrap();
        fs::create_dir_all(allowed.join("nested")).unwrap();

        let root = ConfiningRoot::new(&primary).unwrap();
        let raw_allowed = ConfiningRoot::from_path_buf(allowed.join("nested/.."));
        let candidate = allowed.join("missing.md.j2");

        let result = canonicalize_within_roots(&candidate, &root, &[raw_allowed]);

        assert!(matches!(result, Ok(Canonicalization::Missing { .. })));
    }
}
