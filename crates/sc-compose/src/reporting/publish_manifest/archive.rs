use std::path::{Path, PathBuf};

use crate::reporting::output::ARCHIVE_ROOT_RELATIVE_PATH;

use super::error::PublishManifestError;

pub(super) fn latest_archive_root(
    root: &Path,
    report_id: &str,
) -> Result<Option<PathBuf>, PublishManifestError> {
    let archive_root = root.join(ARCHIVE_ROOT_RELATIVE_PATH);
    if !archive_root.exists() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(&archive_root).map_err(|source| {
        PublishManifestError::ReadArchiveRoot {
            path: archive_root.clone(),
            source,
        }
    })? {
        let entry = entry.map_err(|source| PublishManifestError::ReadArchiveRoot {
            path: archive_root.clone(),
            source,
        })?;
        let report_root = entry.path().join(report_id);
        if report_root.is_dir() {
            candidates.push(
                PathBuf::from(ARCHIVE_ROOT_RELATIVE_PATH)
                    .join(entry.file_name())
                    .join(report_id),
            );
        }
    }
    candidates.sort();
    Ok(candidates.pop())
}
