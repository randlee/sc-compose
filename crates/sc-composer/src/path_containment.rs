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
/// symlinks. For missing candidates, resolve available symlink components and
/// canonicalize the existing prefix before checking the boundary: an escape
/// takes precedence over not-found, and a missing path under an alias of a
/// root (such as `/var` for `/private/var`) is inside it. Canonical roots and
/// caller-preserved lexical aliases are both accepted as containment
/// boundaries.
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
            // Decide containment on the physical path, as for existing files:
            // symlinks resolved, then the existing prefix canonicalized so it
            // compares in the same form as canonical roots (macOS `/private`,
            // Windows long names and verbatim prefixes).
            let physical_candidate = canonicalize_existing_prefix(
                &resolve_missing_symlinks(&candidate).unwrap_or_else(|| normalize_path(&candidate)),
            );
            let normalized_roots = approved_roots
                .iter()
                .flat_map(|root| {
                    [Some(normalize_path(root)), resolve_missing_symlinks(root)]
                        .into_iter()
                        .flatten()
                })
                .collect::<Vec<_>>();
            if !is_within_any(&physical_candidate, &normalized_roots) {
                return Err(ContainmentEscape { candidate });
            }
            Ok(Canonicalization::Missing { candidate, source })
        }
    }
}

/// Canonicalize the deepest existing ancestor of `path` and append the
/// components below it that do not exist.
fn canonicalize_existing_prefix(path: &Path) -> PathBuf {
    for ancestor in path.ancestors() {
        if let Ok(canonical) = std::fs::canonicalize(ancestor) {
            let missing = path
                .strip_prefix(ancestor)
                .unwrap_or_else(|_| Path::new(""));
            return canonical.join(missing);
        }
    }
    path.to_path_buf()
}

/// Resolve symlinks in the existing prefix even when the final target is missing.
/// A bounded traversal leaves loop diagnostics to the original canonicalization error.
fn resolve_missing_symlinks(candidate: &Path) -> Option<PathBuf> {
    const MAX_SYMLINKS: usize = 40;
    let mut pending = candidate.to_path_buf();
    for _ in 0..=MAX_SYMLINKS {
        let mut prefix = PathBuf::new();
        let mut replacement = None;
        for (index, component) in pending.components().enumerate() {
            match component {
                Component::CurDir => continue,
                Component::ParentDir => {
                    prefix.pop();
                    continue;
                }
                _ => prefix.push(component.as_os_str()),
            }
            if std::fs::symlink_metadata(&prefix)
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                let target = std::fs::read_link(&prefix).ok()?;
                prefix.pop();
                let mut next = if target.is_absolute() {
                    target
                } else {
                    prefix.join(target)
                };
                for rest in pending.components().skip(index + 1) {
                    next.push(rest.as_os_str());
                }
                replacement = Some(next);
                break;
            }
        }
        match replacement {
            Some(next) => pending = next,
            None => return Some(normalize_path(&prefix)),
        }
    }
    None
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

    /// A missing path written through an alias of the root (macOS `/var` for
    /// `/private/var`) is inside the canonical root; a missing path under a
    /// symlink that leaves the root is still an escape.
    #[cfg(unix)]
    #[test]
    fn missing_paths_under_a_root_alias_are_contained() {
        let base = std::env::temp_dir().join(format!(
            "sc-compose-path-alias-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let real = base.join("real");
        let outside = base.join("outside");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let alias = base.join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        std::os::unix::fs::symlink(&outside, real.join("out")).unwrap();

        let root = ConfiningRoot::new(&alias).unwrap();
        let inside = canonicalize_within_roots(alias.join("missing.md.j2"), &root, &[]);
        let escaping = canonicalize_within_roots(alias.join("out/missing.md.j2"), &root, &[]);
        fs::remove_dir_all(&base).unwrap();

        assert!(matches!(inside, Ok(Canonicalization::Missing { .. })));
        assert!(matches!(escaping, Err(super::ContainmentEscape { .. })));
    }
}
