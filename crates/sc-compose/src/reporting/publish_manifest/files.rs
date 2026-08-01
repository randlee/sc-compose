use std::path::{Path, PathBuf};

use crate::path_utils::normalize_relative_path;
use crate::reporting::index::ReportIndexEntry;
use crate::reporting::output::LATEST_ROOT_RELATIVE_PATH;

use super::error::{PublishManifestError, invalid_artifact_path};
use super::model::PublishManifestFile;

pub(super) fn build_manifest_files(
    entry: &ReportIndexEntry,
) -> Result<Vec<PublishManifestFile>, PublishManifestError> {
    let latest_report_root = PathBuf::from(LATEST_ROOT_RELATIVE_PATH).join(&entry.report_id);
    let publish_root = PathBuf::from("reports").join(&entry.report_id);

    entry
        .artifacts
        .iter()
        .map(|artifact| {
            Ok(PublishManifestFile {
                role: artifact_role(&entry.entrypoint, &entry.metadata, artifact),
                path: artifact.clone(),
                publish_to: artifact_publish_path(
                    &entry.report_id,
                    artifact,
                    &latest_report_root,
                    &publish_root,
                )?,
            })
        })
        .collect()
}

pub(super) fn artifact_publish_path(
    report_id: &str,
    artifact: &Path,
    latest_report_root: &Path,
    publish_root: &Path,
) -> Result<PathBuf, PublishManifestError> {
    let relative = artifact
        .strip_prefix(latest_report_root)
        .map_err(|error| invalid_artifact_path(report_id, artifact, latest_report_root, error))?;
    let relative = normalize_relative_path(relative).map_err(|message| {
        invalid_artifact_path(report_id, artifact, latest_report_root, message)
    })?;
    Ok(publish_root.join(relative))
}

fn artifact_role(entrypoint: &Path, metadata: &Path, artifact: &Path) -> String {
    if artifact == entrypoint {
        return "entrypoint".to_owned();
    }
    if artifact == metadata {
        return "metadata".to_owned();
    }
    "artifact".to_owned()
}
