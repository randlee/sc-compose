use std::path::{Path, PathBuf};

use super::OutputError;

pub(super) fn write_metadata(
    root: &Path,
    relative_path: &Path,
    contents: Vec<u8>,
) -> Result<(), OutputError> {
    let absolute_path = root.join(relative_path);
    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| OutputError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(&absolute_path, contents).map_err(|source| OutputError::WriteMetadata {
        path: absolute_path,
        source,
    })
}

pub(super) fn archive_artifacts<'a>(
    root: &Path,
    latest_report_root: &Path,
    archive_root: &Path,
    artifacts: impl Iterator<Item = &'a Path>,
) -> Result<Vec<PathBuf>, OutputError> {
    let mut archived_artifacts = Vec::new();
    for artifact in artifacts {
        let relative_artifact = super::layout::relative_artifact(artifact, latest_report_root)?;
        let archive_path = archive_root.join(relative_artifact);
        copy_relative_file(root, artifact, &archive_path)?;
        archived_artifacts.push(archive_path);
    }
    Ok(archived_artifacts)
}

fn copy_relative_file(root: &Path, from: &Path, to: &Path) -> Result<(), OutputError> {
    let absolute_from = root.join(from);
    let absolute_to = root.join(to);
    if let Some(parent) = absolute_to.parent() {
        std::fs::create_dir_all(parent).map_err(|source| OutputError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::copy(&absolute_from, &absolute_to).map_err(|source| OutputError::CopyFile {
        from: absolute_from,
        to: absolute_to,
        source,
    })?;
    Ok(())
}
