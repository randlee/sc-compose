//! Shared path canonicalization and confinement checks.

use std::collections::BTreeSet;
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
/// symlinks, and their requested path may not leave an approved root. For missing candidates, resolve available symlink components
/// before checking the boundary: an escape takes precedence over not-found.
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

    let normalized_roots = approved_roots
        .iter()
        .flat_map(|root| {
            [
                Some(normalize_path(root)),
                resolve_missing_symlinks(root),
                canonicalize_existing_prefix(&normalize_path(root)),
            ]
            .into_iter()
            .flatten()
        })
        .collect::<Vec<_>>();
    let alias_boundary = root_alias_boundary(&candidate, &normalized_roots);
    if alias_boundary == Some(false) {
        return Err(ContainmentEscape { candidate });
    }

    match std::fs::canonicalize(&candidate) {
        Ok(canonical) if is_within_any(&canonical, &approved_roots) => {
            Ok(Canonicalization::Existing(canonical))
        }
        Ok(_) => Err(ContainmentEscape { candidate }),
        Err(source) => {
            let normalized_candidate =
                resolve_missing_symlinks(&candidate).unwrap_or_else(|| normalize_path(&candidate));
            let lexical_contained = match alias_boundary {
                Some(contained) => contained,
                None => is_within_any(&normalize_path(&candidate), &normalized_roots),
            };
            let physical_candidate =
                canonicalize_existing_prefix(&normalized_candidate).unwrap_or(normalized_candidate);
            if !lexical_contained || !is_within_any(&physical_candidate, &normalized_roots) {
                return Err(ContainmentEscape { candidate });
            }
            Ok(Canonicalization::Missing { candidate, source })
        }
    }
}

/// An existing prefix may be a filesystem alias for a canonical approved root.
/// Once that boundary is reached, a parent component may not leave it.
fn root_alias_boundary(candidate: &Path, roots: &[PathBuf]) -> Option<bool> {
    let mut prefix = PathBuf::new();
    let mut crossed = BTreeSet::new();
    let components = candidate.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        prefix.push(component.as_os_str());
        if let Ok(canonical) = std::fs::canonicalize(&prefix)
            && roots.contains(&canonical)
            && !crossed.contains(&canonical)
        {
            let mut depth = 0usize;
            let mut leaves_root = false;
            for remainder in &components[index + 1..] {
                match remainder {
                    Component::Normal(_) => depth += 1,
                    Component::ParentDir if depth == 0 => {
                        leaves_root = true;
                        break;
                    }
                    Component::ParentDir => depth -= 1,
                    _ => {}
                }
            }
            if !leaves_root {
                return Some(true);
            }
            crossed.insert(canonical);
        }
    }
    if crossed.is_empty() {
        None
    } else {
        Some(false)
    }
}

/// Canonicalize the longest existing prefix, retaining the missing suffix.
fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
    let mut ancestor = path;
    let mut suffix = Vec::new();
    loop {
        if let Ok(mut canonical) = std::fs::canonicalize(ancestor) {
            for component in suffix.iter().rev() {
                canonical.push(component);
            }
            return Some(canonical);
        }
        suffix.push(ancestor.file_name()?.to_os_string());
        ancestor = ancestor.parent()?;
    }
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
    fn missing_paths_accept_native_and_canonical_root_spellings_without_traversal() {
        let base = std::env::temp_dir().join(format!(
            "ci1020-roots-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let real = base.join("root");
        let sibling = base.join("root-sibling");
        let allowed = base.join("allowed");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(real.join("nested")).unwrap();
        fs::write(real.join("present"), "content").unwrap();
        fs::create_dir_all(&sibling).unwrap();
        fs::create_dir_all(&allowed).unwrap();
        fs::write(allowed.join("present"), "allowed").unwrap();
        let root = ConfiningRoot::new(&real).unwrap();
        let extra = ConfiningRoot::new(&allowed).unwrap();
        for candidate in [real.join("missing"), root.as_path().join("missing")] {
            assert!(matches!(
                canonicalize_within_roots(candidate, &root, &[]),
                Ok(Canonicalization::Missing { .. })
            ));
        }
        assert!(matches!(
            canonicalize_within_roots(allowed.join("missing"), &root, std::slice::from_ref(&extra)),
            Ok(Canonicalization::Missing { .. })
        ));
        for name in ["present", "missing"] {
            let candidate = real.join("../allowed").join(name);
            canonicalize_within_roots(candidate, &root, std::slice::from_ref(&extra)).unwrap();
        }
        let _escape = canonicalize_within_roots(real.join("../outside"), &root, &[])
            .expect_err("parent traversal must fail");
        for spelling in [&real, root.as_path()] {
            let reentry = |name: &str| {
                let mut raw = spelling.as_os_str().to_os_string();
                let sep = std::path::MAIN_SEPARATOR.to_string();
                raw.push(format!("{sep}..{sep}root{sep}{name}"));
                std::path::PathBuf::from(raw)
            };
            let _escape = canonicalize_within_roots(reentry("missing"), &root, &[])
                .expect_err("leaving and re-entering must fail");
            let _escape = canonicalize_within_roots(reentry("present"), &root, &[])
                .expect_err("existing re-entry must fail like missing re-entry");
        }
        assert!(matches!(
            canonicalize_within_roots(real.join("nested/../present"), &root, &[]),
            Ok(Canonicalization::Existing(_))
        ));
        let _escape = canonicalize_within_roots(sibling.join("missing"), &root, &[])
            .expect_err("a sibling prefix is not a root");
    }

    #[cfg(windows)]
    #[test]
    fn windows_outward_existing_and_dangling_symlinks_are_rejected() {
        use std::os::windows::fs::symlink_file;
        let base = std::env::temp_dir().join(format!(
            "ci1020-symlinks-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root_path = base.join("root");
        fs::create_dir_all(&root_path).unwrap();
        let outside = base.join("outside.txt");
        fs::write(&outside, "outside").unwrap();
        let existing = root_path.join("existing-link");
        if let Err(error) = symlink_file(&outside, &existing) {
            eprintln!("Windows symlink controls unavailable: {error}");
            return;
        }
        let dangling = root_path.join("dangling-link");
        symlink_file(base.join("missing-outside"), &dangling).unwrap();
        let root = ConfiningRoot::new(&root_path).unwrap();
        let _escape = canonicalize_within_roots(&existing, &root, &[])
            .expect_err("existing outside symlink must fail");
        let _escape = canonicalize_within_roots(&dangling, &root, &[])
            .expect_err("dangling outside symlink must fail");
    }

    #[cfg(unix)]
    #[test]
    fn missing_root_alias_is_allowed_but_escape_is_rejected() {
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir().join(format!(
            "ci1016-alias-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let real = base.join("real");
        let alias = base.join("alias");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(real.join("nested")).unwrap();
        fs::write(real.join("present"), "content").unwrap();
        symlink(&real, &alias).unwrap();
        let root = ConfiningRoot::new(&real).unwrap();
        assert!(matches!(
            canonicalize_within_roots(alias.join("missing"), &root, &[]),
            Ok(Canonicalization::Missing { .. })
        ));
        let _escape = canonicalize_within_roots(alias.join("../outside"), &root, &[])
            .expect_err("parent traversal must leave the approved root");
        let _escape = canonicalize_within_roots(real.join("../alias/missing"), &root, &[])
            .expect_err("leaving and re-entering a root alias must be rejected");
        let _escape = canonicalize_within_roots(real.join("../alias/present"), &root, &[])
            .expect_err("existing root alias re-entry must be rejected");
        assert!(matches!(
            canonicalize_within_roots(alias.join("nested/../present"), &root, &[]),
            Ok(Canonicalization::Existing(_))
        ));
        symlink(base.join("outside"), real.join("dangling")).unwrap();
        let _escape = canonicalize_within_roots(alias.join("dangling/file"), &root, &[])
            .expect_err("a dangling symlink outside the root must be rejected");
        let allowed = base.join("allowed");
        let allowed_alias = base.join("allowed-alias");
        fs::create_dir_all(&allowed).unwrap();
        symlink(&allowed, &allowed_alias).unwrap();
        let extra_root = ConfiningRoot::new(&allowed).unwrap();
        assert!(matches!(
            canonicalize_within_roots(allowed_alias.join("missing"), &root, &[extra_root]),
            Ok(Canonicalization::Missing { .. })
        ));
    }

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
