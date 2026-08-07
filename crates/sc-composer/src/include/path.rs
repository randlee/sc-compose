use std::path::{Path, PathBuf};

use super::super::path_containment::{Canonicalization, canonicalize_within_roots};
use crate::DiagnosticCode;
use crate::error::{ComposeError, IncludeError};
use crate::types::ConfiningRoot;

pub(super) fn resolve_include_path(
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

pub(super) fn canonicalize_include(
    candidate: &Path,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
    stack: &[PathBuf],
) -> Result<PathBuf, ComposeError> {
    let root = ConfiningRoot::from_path_buf(root.to_path_buf());
    match canonicalize_within_roots(candidate, &root, allowed_roots) {
        Ok(Canonicalization::Existing(canonical)) => Ok(canonical),
        Ok(Canonicalization::Missing { candidate, source }) => {
            let (code, message) =
                match crate::diagnostics::classify_filesystem_error(&candidate, &source) {
                    crate::diagnostics::FilesystemErrorClass::IsADirectory => (
                        DiagnosticCode::ErrIncludeIsADirectory,
                        format!(
                            "include target is a directory, not a file: {}",
                            candidate.display()
                        ),
                    ),
                    crate::diagnostics::FilesystemErrorClass::FilesystemLoop => (
                        DiagnosticCode::ErrIncludeFilesystemLoop,
                        format!(
                            "include path is a filesystem symlink loop: {}",
                            candidate.display()
                        ),
                    ),
                    crate::diagnostics::FilesystemErrorClass::PermissionDenied => (
                        DiagnosticCode::ErrIncludePermissionDenied,
                        format!(
                            "permission denied resolving include: {}",
                            candidate.display()
                        ),
                    ),
                    _ => (
                        DiagnosticCode::ErrIncludeNotFound,
                        format!("include file not found: {}", candidate.display()),
                    ),
                };
            Err(IncludeError::new(code, message, stack.to_vec())
                .with_source(source)
                .into())
        }
        Err(escape) => Err(IncludeError::new(
            DiagnosticCode::ErrIncludeEscape,
            format!(
                "include path escapes confinement root: {}",
                escape.candidate.display()
            ),
            stack.to_vec(),
        )
        .into()),
    }
}
